//! Relocate a watched project root and keep the suite's saved references together.
use super::*;
use std::io::Write;

#[derive(Clone, Serialize, Deserialize)]
struct MoveRecord {
    source: PathBuf,
    destination: PathBuf,
    ready: bool,
}

pub(super) struct Dialog {
    record: MoveRecord,
    receiver: Option<Receiver<Result<String, String>>>,
    result: Option<Result<String, String>>,
}
impl Dialog {
    pub(super) fn running(&self) -> bool {
        self.receiver.is_some()
    }
}

fn journal() -> Result<PathBuf, String> {
    Ok(app_data()
        .ok_or("Settings folder is unavailable.")?
        .join("project-folder-move.json"))
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(path.parent().ok_or("Settings folder is unavailable.")?)
        .map_err(|e| e.to_string())?;
    let temp = path.with_extension("move-new");
    let mut output = fs::File::create(&temp).map_err(|e| e.to_string())?;
    output.write_all(bytes).map_err(|e| e.to_string())?;
    output.sync_all().map_err(|e| e.to_string())?;
    drop(output);
    fs::rename(temp, path).map_err(|e| e.to_string())
}
fn save_record(record: &MoveRecord) -> Result<(), String> {
    atomic_write(
        &journal()?,
        &serde_json::to_vec(record).map_err(|e| e.to_string())?,
    )
}
fn remap(path: &mut PathBuf, record: &MoveRecord) {
    if let Ok(relative) = path.strip_prefix(&record.source) {
        *path = record.destination.join(relative);
    }
}

impl Launcher {
    pub(super) fn choose_folder_move(&mut self, source: PathBuf) {
        if self.folder_move.is_some() {
            return;
        }
        if journal().ok().is_some_and(|p| p.exists()) {
            self.toast = Some("A previous folder move needs recovery. Restart Master Suite before moving another folder.".into());
            return;
        }
        let Some(parent) = rfd::FileDialog::new()
            .set_title(tr("Choose where to move the projects folder"))
            .set_directory(source.parent().unwrap_or(&source))
            .pick_folder()
        else {
            return;
        };
        let Some(name) = source.file_name() else {
            self.toast = Some("Choose a project folder, not an entire drive.".into());
            return;
        };
        let record = MoveRecord {
            destination: parent.join(name),
            source,
            ready: false,
        };
        let validation = validate(&record).and_then(|_| {
            let resolved = fs::canonicalize(&record.source).map_err(|e| e.to_string())?;
            let mut protected: Vec<PathBuf> = self.cloud.settings.folders.values().map(|f| f.path.clone()).collect();
            protected.extend(app_data());
            protected.extend(std::env::current_exe().ok());
            for path in protected {
                if fs::canonicalize(path).is_ok_and(|p| p.starts_with(&resolved)) {
                    return Err("This folder contains the suite, its settings, or a connected backup destination. Choose a folder containing only your projects and workspace files.".into());
                }
            }
            Ok(())
        });
        if let Err(error) = validation {
            self.toast = Some(error);
            return;
        }
        self.folder_move = Some(Dialog {
            record,
            receiver: None,
            result: None,
        });
    }

    fn apply_folder_move(&mut self, record: &MoveRecord) -> Result<(), String> {
        for root in &mut self.prefs.roots {
            remap(root, record);
        }
        if let Some(root) = &mut self.prefs.default_project_root {
            remap(root, record);
        }
        for path in &mut self.prefs.favorite_projects {
            remap(path, record);
        }
        for path in &mut self.cloud.settings.selected {
            remap(path, record);
        }
        for receipt in &mut self.cloud.settings.receipts {
            remap(&mut receipt.path, record);
        }
        self.cloud.errors.clear();
        self.cloud.monitor = cloud::windows_backup::Monitor::new();
        let path = preferences_path().ok_or("Could not find the preferences folder.")?;
        atomic_write(
            &path,
            &serde_json::to_vec_pretty(&self.prefs).map_err(|e| e.to_string())?,
        )
        .map_err(|e| {
            format!("Files moved, but settings could not be saved: {e}. Restart to retry recovery.")
        })?;
        self.cloud.persist()?;
        plugins::relocate_workspace(&record.source, &record.destination)?;
        for app in APPS {
            workspace_bridge::prepare(&record.destination, app)?;
        }
        self.projects.clear();
        self.project_previews.clear();
        self.show_project_rename = None;
        self.show_project_delete = None;
        if let Ok(path) = journal() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub(super) fn recover_folder_move(&mut self) {
        let Some(record) = journal()
            .ok()
            .and_then(|p| fs::read(p).ok())
            .and_then(|b| serde_json::from_slice::<MoveRecord>(&b).ok())
        else {
            return;
        };
        if record.destination.is_dir() && (record.ready || !record.source.exists()) {
            self.toast = Some(match self.apply_folder_move(&record) {
                Ok(()) => format!(
                    "Recovered your moved projects folder at {}.",
                    record.destination.display()
                ),
                Err(e) => format!("Folder move recovery needs attention: {e}"),
            });
        } else {
            self.toast = Some(format!(
                "The previous move did not finish. Your original folder is at {}. Any partial copy at {} has been kept for review.",
                record.source.display(),
                record.destination.display()
            ));
            if let Ok(path) = journal() {
                let _ = fs::remove_file(path);
            }
        }
    }

    pub(super) fn poll_folder_move(&mut self) {
        let outcome = self.folder_move.as_ref().and_then(|dialog| dialog.receiver.as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("The move stopped unexpectedly. Restart Master Suite to recover its saved location.".into())),
                Err(mpsc::TryRecvError::Empty) => None,
            }));
        if let Some(outcome) = outcome {
            let record = self.folder_move.as_ref().unwrap().record.clone();
            let outcome =
                outcome.and_then(|message| self.apply_folder_move(&record).map(|_| message));
            let dialog = self.folder_move.as_mut().unwrap();
            dialog.receiver = None;
            dialog.result = Some(outcome);
            self.scan_projects();
        }
    }

    pub(super) fn folder_move_dialog(&mut self, ctx: &egui::Context) {
        let ready = !self.prefs.scanning
            && !self.cloud.busy
            && self.pending_launch.is_none()
            && !self.suite_update_busy && !self.plugins.busy;
        let Some(dialog) = &mut self.folder_move else {
            return;
        };
        let mut start = false;
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("move-project-folder"))
            .frame(egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(UI_RADIUS).inner_margin(24))
            .show(ctx, |ui| {
                ui.set_width(500.0);
                ui.label(RichText::new(tr("Move projects folder")).size(text_size(23.0)).strong());
                ui.add_space(12.0);
                ui.label(RichText::new(tr("Current location")).color(muted()));
                ui.label(dialog.record.source.display().to_string());
                ui.add_space(8.0);
                ui.label(RichText::new(tr("New location")).color(muted()));
                ui.label(dialog.record.destination.display().to_string());
                ui.add_space(16.0);
                if dialog.running() {
                    ui.horizontal(|ui| { ui.spinner(); ui.label(tr("Moving your folder…")); });
                    ui.label(tr("Keep Master Suite open until the move finishes."));
                } else if let Some(result) = &dialog.result {
                    match result {
                        Ok(message) => { ui.label(tr(message)); }
                        Err(error) => { ui.colored_label(theme_rgb(230, 102, 112), tr(error)); }
                    }
                    close = secondary_button(ui, "Close").clicked();
                } else {
                    ui.label(tr("Move this folder and everything inside it, including projects, assets, exports and plugins. Existing destination folders will not be overwritten."));
                    ui.add_space(8.0);
                    ui.label(RichText::new(tr("Save your work and close apps using this folder before continuing.")).strong());
                    if !ready { ui.label(tr("Waiting for the current scan, backup, launch or update to finish…")); }
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        start = if ui_themes::is_v2() {
                            ui.add_enabled(ready, ui_v2::primary_button("Move folder"))
                        } else {
                            ui.add_enabled(ready, egui::Button::new(tr("Move folder")).fill(ACCENT))
                        }.clicked();
                        close = secondary_button(ui, "Cancel").clicked();
                    });
                }
            });
        if start {
            let record = dialog.record.clone();
            let (tx, rx) = mpsc::channel();
            dialog.receiver = Some(rx);
            thread::spawn(move || {
                let _ = tx.send(move_folder(record));
            });
        } else if !dialog.running() && (close || modal.should_close()) {
            self.folder_move = None;
        }
    }
}

fn validate(record: &MoveRecord) -> Result<(), String> {
    let source = fs::canonicalize(&record.source)
        .map_err(|e| format!("Could not open the source folder: {e}"))?;
    let parent = fs::canonicalize(
        record
            .destination
            .parent()
            .ok_or("Choose a destination folder.")?,
    )
    .map_err(|e| e.to_string())?;
    if !source.is_dir() || source.parent().is_none() {
        return Err("Choose a project folder, not an entire drive.".into());
    }
    if parent.starts_with(&source) {
        return Err("Choose a destination outside the folder being moved.".into());
    }
    if fs::symlink_metadata(&record.destination).is_ok() {
        return Err("A folder with this name already exists at the destination. Choose another location; existing files will not be merged or replaced.".into());
    }
    Ok(())
}

fn inventory(root: &Path) -> Result<Vec<(PathBuf, bool)>, String> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
        let linked = metadata.file_type().is_symlink();
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            linked || metadata.file_attributes() & 0x400 != 0
        };
        if linked || (!metadata.is_dir() && !metadata.is_file()) {
            return Err(format!(
                "{} is a link, cloud placeholder or special file. Make it a local file or move it separately before moving this folder.",
                entry.path().display()
            ));
        }
        entries.push((
            entry
                .path()
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_path_buf(),
            metadata.is_dir(),
        ));
    }
    entries.sort();
    Ok(entries)
}
fn digest(path: &Path) -> Result<Vec<u8>, String> {
    let mut input = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hash.finalize().to_vec())
}

fn move_folder(mut record: MoveRecord) -> Result<String, String> {
    validate(&record)?;
    let entries = inventory(&record.source)?;
    save_record(&record)?;
    #[cfg(windows)]
    match fs::rename(&record.source, &record.destination) {
        Ok(()) => {
            return Ok(format!(
                "Projects folder moved to {}.",
                record.destination.display()
            ));
        }
        Err(error) => {
            let cross_device = if cfg!(windows) { Some(17) } else { Some(18) };
            if error.raw_os_error() != cross_device {
                if let Ok(path) = journal() {
                    let _ = fs::remove_file(path);
                }
                return Err(format!(
                    "Could not move the folder: {error}. Close apps using it and try again."
                ));
            }
        }
    }
    // Cross-drive moves copy into a new directory, then verify before removing originals.
    let copy_result = (|| -> Result<Vec<(PathBuf, Vec<u8>)>, String> {
        fs::create_dir(&record.destination).map_err(|e| e.to_string())?;
        let mut hashes = Vec::new();
        for (relative, directory) in &entries {
            if relative.as_os_str().is_empty() {
                continue;
            }
            let source = record.source.join(relative);
            let target = record.destination.join(relative);
            if *directory {
                fs::create_dir(&target).map_err(|e| e.to_string())?;
                continue;
            }
            let mut input = fs::File::open(&source).map_err(|e| e.to_string())?;
            let metadata = input.metadata().map_err(|e| e.to_string())?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|e| e.to_string())?;
            std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
            output
                .flush()
                .and_then(|_| output.sync_all())
                .map_err(|e| e.to_string())?;
            if let Ok(modified) = metadata.modified() {
                output
                    .set_times(fs::FileTimes::new().set_modified(modified))
                    .map_err(|e| e.to_string())?;
            }
            drop(output);
            fs::set_permissions(&target, metadata.permissions()).map_err(|e| e.to_string())?;
            let hash = digest(&source)?;
            if digest(&target)? != hash {
                return Err(format!(
                    "{} changed during the move. Close the app using it and retry.",
                    source.display()
                ));
            }
            hashes.push((relative.clone(), hash));
        }
        if inventory(&record.source)? != entries {
            return Err("The folder contents changed during the move. Close apps using the folder and retry.".into());
        }
        for (relative, hash) in &hashes {
            if digest(&record.source.join(relative))? != *hash {
                return Err(
                    "A file changed during the move. The original folder has been kept.".into(),
                );
            }
        }
        Ok(hashes)
    })();
    let hashes = match copy_result {
        Ok(hashes) => hashes,
        Err(error) => {
            if let Ok(path) = journal() {
                let _ = fs::remove_file(path);
            }
            return Err(format!(
                "Could not finish the move: {error}. Originals remain at {}. A partial destination may remain at {}; choose a new destination when retrying.",
                record.source.display(),
                record.destination.display()
            ));
        }
    };
    record.ready = true;
    save_record(&record)?;
    let mut retained = false;
    for (relative, hash) in hashes {
        let source = record.source.join(&relative);
        if digest(&source).is_ok_and(|current| current == hash)
            && digest(&record.destination.join(&relative)).is_ok_and(|current| current == hash)
        {
            if fs::remove_file(source).is_err() {
                retained = true;
            }
        } else {
            return Err(format!(
                "A file changed during final cleanup. Copies are at {} and remaining originals are at {}. Review both folders, then restart Master Suite to recover the new location.",
                record.destination.display(),
                record.source.display()
            ));
        }
    }
    for (relative, directory) in entries.iter().rev() {
        if *directory && fs::remove_dir(record.source.join(relative)).is_err() {
            retained = true;
        }
    }
    Ok(if retained {
        format!(
            "Your projects now use {}. Some original files were changed or could not be removed and remain at {}; review those files before deleting the old folder.",
            record.destination.display(),
            record.source.display()
        )
    } else {
        format!("Projects folder moved to {}.", record.destination.display())
    })
}
