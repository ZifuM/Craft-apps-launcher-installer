//! A local asset library. Importing copies files; using an asset never edits app preferences.
use super::*;
use std::{
    collections::{HashSet, VecDeque},
    io::Write,
};

pub const KINDS: &[&str] = &[
    "All types",
    "Images",
    "Video",
    "Audio",
    "Presets",
    "Fonts",
    "Models",
    "Documents",
    "Other",
];
pub const PRESETS: &[&str] = &[
    "abr",
    "grd",
    "pat",
    "aco",
    "ase",
    "asl",
    "atn",
    "kys",
    "psp",
    "cube",
    "3dl",
    "look",
    "xmp",
    "lcpreset",
    "lrtemplate",
    "ffx",
    "prfpset",
    "vstpreset",
    "fxp",
    "fxb",
    "pcbrushes",
];

pub fn extension(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase()
}
pub fn kind(path: &Path) -> &'static str {
    let ext = extension(path);
    if PRESETS.contains(&ext.as_str()) {
        return "Presets";
    }
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tif" | "tiff" | "svg" | "eps" | "ai"
        | "psd" | "psb" | "pcraft" | "ico" | "tga" | "exr" | "hdr" | "heic" | "heif" | "avif"
        | "dng" | "cr2" | "cr3" | "nef" | "arw" | "raf" | "orf" | "rw2" | "pef" => "Images",
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" | "mpeg" | "mpg" | "mxf" => "Video",
        "wav" | "aiff" | "aif" | "flac" | "mp3" | "ogg" | "opus" | "m4a" | "aac" | "wma"
        | "mid" | "midi" | "smf" => "Audio",
        "ttf" | "otf" | "woff" | "woff2" => "Fonts",
        "obj" | "fbx" | "gltf" | "glb" | "stl" | "blend" | "dxf" | "dwg" | "step" | "stp" => {
            "Models"
        }
        "pdf" | "txt" | "md" | "docx" | "odt" | "rtf" | "xlsx" | "csv" | "tsv" | "pptx" | "odp" => {
            "Documents"
        }
        _ => "Other",
    }
}

#[derive(Clone)]
pub struct Asset {
    pub file: Project,
    pub root: PathBuf,
    pub kind: &'static str,
}
#[derive(Default)]
pub struct Library {
    pub items: Vec<Asset>,
    pub note: String,
}
pub struct Import {
    pub paths: Vec<PathBuf>,
    pub app: String,
    pub presets: bool,
}
pub struct Use {
    pub asset: Asset,
    pub app: String,
}
pub struct Rename {
    pub asset: Asset,
    pub name: String,
}
pub struct Report {
    pub message: String,
    pub failed: bool,
    pub remaps: Vec<(PathBuf, Option<PathBuf>)>,
}
enum Progress {
    Update(String, f32),
    Done(Result<Report, String>),
}
type PreviewResult = (PathBuf, u64, Option<(image::RgbaImage, &'static str)>);
pub struct Manager {
    pub library: Library,
    pub textures: HashMap<PathBuf, egui::TextureHandle>,
    pub search: String,
    pub app_filter: String,
    pub type_filter: String,
    pub scope: String,
    pub sort: String,
    pub selected: HashSet<PathBuf>,
    pub folders_tab: bool,
    pub import: Option<Import>,
    pub use_asset: Option<Use>,
    pub rename: Option<Rename>,
    pub remove: Vec<Asset>,
    pub preset_help: Option<String>,
    pub message: String,
    pub failed: bool,
    pub busy: bool,
    pub progress: f32,
    pub rescan_projects: bool,
    scan_rx: Option<Receiver<Library>>,
    pending_scan: Option<Vec<PathBuf>>,
    rx: Option<Receiver<Progress>>,
    preview_rx: Option<Receiver<PreviewResult>>,
    preview_attempts: HashSet<(PathBuf, u64)>,
    preview_order: VecDeque<PathBuf>,
}
impl Default for Manager {
    fn default() -> Self {
        Self {
            library: Library::default(),
            textures: HashMap::new(),
            search: String::new(),
            app_filter: "All apps".into(),
            type_filter: "All types".into(),
            scope: "All assets".into(),
            sort: "Recently modified".into(),
            selected: HashSet::new(),
            folders_tab: false,
            import: None,
            use_asset: None,
            rename: None,
            remove: Vec::new(),
            preset_help: None,
            message: String::new(),
            failed: false,
            busy: false,
            progress: 0.0,
            rescan_projects: false,
            scan_rx: None,
            pending_scan: None,
            rx: None,
            preview_rx: None,
            preview_attempts: HashSet::new(),
            preview_order: VecDeque::new(),
        }
    }
}
impl Manager {
    pub fn scanning(&self) -> bool {
        self.scan_rx.is_some()
    }
    pub fn loading_preview(&self) -> bool {
        self.preview_rx.is_some()
    }
    pub fn request_preview(&mut self, asset: &Asset) {
        self.preview_order.retain(|p| p != &asset.file.path);
        self.preview_order.push_back(asset.file.path.clone());
        while self.preview_order.len() > 96 {
            if let Some(path) = self.preview_order.pop_front() {
                self.textures.remove(&path);
                if let Some(item) = self.library.items.iter_mut().find(|a| a.file.path == path) {
                    item.file.preview = None;
                    self.preview_attempts.remove(&(path, item.file.revision));
                }
            }
        }
        if asset.file.preview.is_some()
            || self.preview_rx.is_some()
            || self
                .preview_attempts
                .contains(&(asset.file.path.clone(), asset.file.revision))
        {
            return;
        }
        self.preview_attempts
            .insert((asset.file.path.clone(), asset.file.revision));
        let path = asset.file.path.clone();
        let revision = asset.file.revision;
        let size = asset.file.size_bytes;
        let (tx, rx) = mpsc::channel();
        self.preview_rx = Some(rx);
        thread::spawn(move || {
            let image = preview(&path, size);
            let image = if project_revision(&path) == revision {
                image
            } else {
                None
            };
            let _ = tx.send((path, revision, image));
        });
    }
    pub fn refresh(&mut self, roots: Vec<PathBuf>) {
        if self.busy || self.scanning() {
            self.pending_scan = Some(roots);
            return;
        }
        let previous = self
            .library
            .items
            .iter()
            .map(|a| (a.file.path.clone(), a.clone()))
            .collect();
        let (tx, rx) = mpsc::channel();
        self.scan_rx = Some(rx);
        thread::spawn(move || {
            let _ = tx.send(scan(&roots, &previous));
        });
    }
    pub fn tick(&mut self) -> Option<Report> {
        if let Some(mut library) = self.scan_rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            // Keep previews that arrived during the background folder scan.
            let current: HashMap<_, _> = self
                .library
                .items
                .iter()
                .map(|a| (&a.file.path, &a.file))
                .collect();
            for asset in &mut library.items {
                if let Some(current) = current
                    .get(&asset.file.path)
                    .filter(|p| p.revision == asset.file.revision)
                {
                    asset.file.preview = current.preview.clone();
                    asset.file.preview_note = current.preview_note;
                }
            }
            let revisions: HashMap<_, _> = library
                .items
                .iter()
                .map(|a| (&a.file.path, a.file.revision))
                .collect();
            self.textures.retain(|p, _| {
                self.library.items.iter().any(|a| {
                    &a.file.path == p && revisions.get(p).is_some_and(|r| *r == a.file.revision)
                })
            });
            self.selected.retain(|p| revisions.contains_key(p));
            self.preview_attempts
                .retain(|(p, r)| revisions.get(p).is_some_and(|revision| revision == r));
            self.library = library;
            self.scan_rx = None;
        }
        if let Some((path, revision, preview)) =
            self.preview_rx.as_ref().and_then(|rx| rx.try_recv().ok())
        {
            self.preview_rx = None;
            if let Some(asset) = self
                .library
                .items
                .iter_mut()
                .find(|a| a.file.path == path && a.file.revision == revision)
            {
                if let Some((image, note)) = preview {
                    asset.file.preview = Some(Arc::new(image));
                    asset.file.preview_note = note;
                }
                if let Some(request) = &mut self.use_asset {
                    if request.asset.file.path == path {
                        request.asset = asset.clone();
                    }
                }
            }
        }
        let events: Vec<_> = self
            .rx
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        let mut report = None;
        for event in events {
            match event {
                Progress::Update(message, progress) => {
                    self.message = message;
                    self.progress = progress;
                }
                Progress::Done(result) => {
                    let result = result.unwrap_or_else(|message| Report {
                        message,
                        failed: true,
                        remaps: Vec::new(),
                    });
                    self.message = result.message.clone();
                    self.failed = result.failed;
                    self.progress = 1.0;
                    self.busy = false;
                    self.rx = None;
                    report = Some(result);
                }
            }
        }
        if !self.busy && !self.scanning() {
            if let Some(roots) = self.pending_scan.take() {
                self.refresh(roots);
            }
        }
        report
    }
    fn start(
        &mut self,
        job: impl FnOnce(&Sender<Progress>) -> Result<Report, String> + Send + 'static,
    ) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.failed = false;
        self.progress = 0.0;
        self.message = "Preparing files…".into();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        thread::spawn(move || {
            let result = job(&tx);
            let _ = tx.send(Progress::Done(result));
        });
    }
    pub fn import_files(
        &mut self,
        root: PathBuf,
        app: AppInfo,
        paths: Vec<PathBuf>,
        presets: bool,
    ) {
        self.start(move |tx| {
            let base = workspace_bridge::prepare(&root, &app)?.join("Assets");
            let dest = if presets { base.join("Presets") } else { base };
            import_paths(&paths, &dest, tx)
        });
    }
    pub fn export_files(&mut self, paths: Vec<PathBuf>, destination: PathBuf) {
        self.start(move |tx| import_paths(&paths, &destination, tx));
    }
    pub fn rename_file(&mut self, asset: Asset, name: String) {
        self.start(move |_| {
            checked_asset(&asset)?;
            let name = name.trim();
            if !safe_name(name) {
                return Err(
                    "Enter a file name without path separators or reserved characters.".into(),
                );
            }
            if extension(Path::new(name)) != extension(&asset.file.path) {
                return Err("Keep the existing file extension when renaming an asset.".into());
            }
            let dest = asset.file.path.with_file_name(name);
            if dest == asset.file.path {
                return Ok(Report {
                    message: "The name is unchanged.".into(),
                    failed: false,
                    remaps: Vec::new(),
                });
            }
            if dest.exists() {
                return Err("An asset with that name already exists.".into());
            }
            fs::rename(&asset.file.path, &dest).map_err(|e| e.to_string())?;
            Ok(Report {
                message: "Asset renamed.".into(),
                failed: false,
                remaps: vec![(asset.file.path, Some(dest))],
            })
        });
    }
    pub fn remove_files(&mut self, assets: Vec<Asset>) {
        self.start(move |tx| {
            let mut report = Report {
                message: String::new(),
                failed: false,
                remaps: Vec::new(),
            };
            let mut failures = Vec::new();
            for (i, asset) in assets.iter().enumerate() {
                let result = (|| {
                    checked_asset(asset)?;
                    let trash = asset.root.join(".Trash");
                    fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
                    let dest = unique(&trash, &asset.file.title);
                    fs::rename(&asset.file.path, dest).map_err(|e| e.to_string())
                })();
                match result {
                    Ok(()) => report.remaps.push((asset.file.path.clone(), None)),
                    Err(e) => failures.push(format!("{}: {e}", asset.file.title)),
                }
                let _ = tx.send(Progress::Update(
                    "Moving assets to library trash…".into(),
                    (i + 1) as f32 / assets.len().max(1) as f32,
                ));
            }
            report.message = format!(
                "Moved {} asset(s) to the .Trash folder in their asset library.",
                report.remaps.len()
            );
            if !failures.is_empty() {
                report.failed = true;
                report.message.push_str(&format!(
                    " {} could not be moved: {}",
                    failures.len(),
                    failures
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
            Ok(report)
        });
    }
}

pub fn folders(roots: &[PathBuf]) -> Vec<(&'static AppInfo, PathBuf)> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for root in roots {
        for app in APPS {
            let mut paths = vec![root.join(app.name).join("Assets")];
            if root
                .file_name()
                .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(app.name))
            {
                paths.push(root.join("Assets"));
            }
            if root
                .file_name()
                .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case("Assets"))
                && root
                    .parent()
                    .and_then(Path::file_name)
                    .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(app.name))
            {
                paths.push(root.clone());
            }
            for path in paths {
                if let Ok(canonical) = fs::canonicalize(&path) {
                    if seen.insert(canonical) {
                        result.push((app, path));
                    }
                }
            }
        }
    }
    result
}
fn scan(roots: &[PathBuf], previous: &HashMap<PathBuf, Asset>) -> Library {
    let mut library = Library::default();
    let mut unreadable = 0;
    let mut visited = 0;
    let mut seen = HashSet::new();
    'folders: for (app, root) in folders(roots) {
        for item in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                !e.file_type().is_symlink() && !e.file_name().to_string_lossy().starts_with('.')
            })
        {
            visited += 1;
            if library.items.len() >= 10_000 || visited > 100_000 {
                library.note = "Showing the first 10,000 assets. Use smaller connected folders for larger libraries.".into();
                break 'folders;
            }
            let item = match item {
                Ok(e) => e,
                Err(_) => {
                    unreadable += 1;
                    continue;
                }
            };
            if !item.file_type().is_file() || !seen.insert(item.path().to_path_buf()) {
                continue;
            }
            let Ok(meta) = item.metadata() else {
                unreadable += 1;
                continue;
            };
            let path = item.into_path();
            let revision = project_revision(&path);
            let asset =
                if let Some(old) = previous.get(&path).filter(|a| a.file.revision == revision) {
                    old.clone()
                } else {
                    let asset_kind = if path.strip_prefix(&root).ok().is_some_and(|p| {
                        p.components().any(|c| {
                            c.as_os_str()
                                .to_string_lossy()
                                .eq_ignore_ascii_case("Presets")
                        })
                    }) {
                        "Presets"
                    } else {
                        kind(&path)
                    };
                    Asset {
                        root: root.clone(),
                        kind: asset_kind,
                        file: Project {
                            title: path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                            app,
                            modified: meta.modified().ok().map(DateTime::<Local>::from),
                            size_bytes: meta.len(),
                            preview: None,
                            preview_note: "No preview available",
                            path,
                            revision,
                        },
                    }
                };
            library.items.push(asset);
        }
    }
    if unreadable > 0 {
        library.note.push_str(&format!(
            " {unreadable} files or folders could not be read."
        ));
    }
    library
        .items
        .sort_by_key(|a| std::cmp::Reverse(a.file.modified));
    library
}
fn preview(path: &Path, size: u64) -> Option<(image::RgbaImage, &'static str)> {
    let ext = extension(path);
    if size <= 32 * 1024 * 1024
        && matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "tif" | "tiff" | "webp"
        )
    {
        let mut reader = image::ImageReader::open(path)
            .ok()?
            .with_guessed_format()
            .ok()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(30_000);
        limits.max_image_height = Some(30_000);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        return reader
            .decode()
            .ok()
            .map(|i| (i.thumbnail(480, 300).to_rgba8(), "Image preview"));
    }
    if size <= 2 * 1024 * 1024 * 1024 && matches!(ext.as_str(), "psd" | "psb") {
        return project_preview::psd_canvas(path)
            .or_else(|| psd_thumbnail(path))
            .map(|i| (i, "Saved canvas preview"));
    }
    preview_formats::load(path, &ext).map(|(i, n)| {
        (
            image::DynamicImage::ImageRgba8(i)
                .thumbnail(480, 300)
                .to_rgba8(),
            n,
        )
    })
}
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.ends_with([' ', '.'])
        && !name.contains(['/', '\\', ':', '\0', '<', '>', '|', '?', '*'])
        && Path::new(name).components().count() == 1
}
fn unique(folder: &Path, name: &str) -> PathBuf {
    let candidate = folder.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = Path::new(name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let ext = Path::new(name)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for i in 2.. {
        let path = folder.join(format!("{stem} ({i}){ext}"));
        if !path.exists() {
            return path;
        }
    }
    unreachable!()
}
fn checked_asset(asset: &Asset) -> Result<(), String> {
    let root = fs::canonicalize(&asset.root).map_err(|e| e.to_string())?;
    let path = fs::canonicalize(&asset.file.path).map_err(|e| e.to_string())?;
    if !path.starts_with(&root)
        || !fs::symlink_metadata(&asset.file.path)
            .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
    {
        return Err("The asset is no longer a regular file inside its library.".into());
    }
    Ok(())
}
fn import_paths(
    paths: &[PathBuf],
    destination: &Path,
    tx: &Sender<Progress>,
) -> Result<Report, String> {
    fs::create_dir_all(destination)
        .map_err(|e| format!("Cannot create {}: {e}", destination.display()))?;
    let canonical_dest = fs::canonicalize(destination).map_err(|e| e.to_string())?;
    let mut jobs = Vec::new();
    let mut warnings = Vec::new();
    for source in paths {
        let meta =
            fs::symlink_metadata(source).map_err(|e| format!("{}: {e}", source.display()))?;
        if meta.file_type().is_symlink() {
            return Err("Import the original file or folder instead of a symbolic link.".into());
        }
        let name = source
            .file_name()
            .ok_or("Choose a file or a folder, not an entire drive.")?
            .to_string_lossy();
        if !safe_name(&name) {
            return Err(format!("Unsupported asset name: {name}"));
        }
        if meta.is_dir() {
            let canonical_source = fs::canonicalize(source).map_err(|e| e.to_string())?;
            if canonical_dest.starts_with(&canonical_source) {
                return Err("Choose a source folder outside the destination library to avoid copying it into itself.".into());
            }
            let folder = unique(destination, &name);
            // Reserve a folder so separate inputs with the same name never merge.
            fs::create_dir(&folder).map_err(|e| e.to_string())?;
            for entry in WalkDir::new(source)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
            {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.depth() > 32 || jobs.len() >= 10_000 {
                    return Err(
                        "Import fewer than 10,000 files with folders no more than 32 levels deep."
                            .into(),
                    );
                }
                if entry.file_type().is_symlink() {
                    warnings.push(format!("Skipped link {}", entry.path().display()));
                    continue;
                }
                if entry.file_type().is_file() {
                    jobs.push((
                        entry.path().to_path_buf(),
                        folder.join(
                            entry
                                .path()
                                .strip_prefix(source)
                                .map_err(|e| e.to_string())?,
                        ),
                    ));
                }
            }
        } else if meta.is_file() {
            jobs.push((source.clone(), destination.join(name.as_ref())));
        }
    }
    let total: u64 = jobs
        .iter()
        .filter_map(|(s, _)| fs::metadata(s).ok())
        .map(|m| m.len())
        .sum();
    let mut done = 0_u64;
    let mut count = 0;
    let mut failures = Vec::new();
    for (source, requested) in jobs {
        let result = (|| {
            let parent = requested.parent().ok_or("Invalid asset folder")?;
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            let dest = unique(
                parent,
                &requested.file_name().unwrap_or_default().to_string_lossy(),
            );
            let mut input = fs::File::open(&source).map_err(|e| e.to_string())?;
            let before = input.metadata().map_err(|e| e.to_string())?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&dest)
                .map_err(|e| e.to_string())?;
            let copied = (|| {
                let mut buffer = vec![0_u8; 1024 * 1024];
                let mut length = 0;
                loop {
                    let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    if length + n as u64 > before.len() {
                        return Err(
                            "The source grew while it was being copied. Save it and try again."
                                .into(),
                        );
                    }
                    output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
                    length += n as u64;
                    done += n as u64;
                    let _ = tx.send(Progress::Update(
                        format!(
                            "Copying {}",
                            source.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        (done as f64 / total.max(1) as f64).min(1.0) as f32,
                    ));
                }
                output.sync_all().map_err(|e| e.to_string())?;
                let after = input.metadata().map_err(|e| e.to_string())?;
                if length != before.len()
                    || after.len() != before.len()
                    || after.modified().ok() != before.modified().ok()
                {
                    return Err(
                        "The source changed while it was being copied. Save it and try again."
                            .into(),
                    );
                }
                Ok::<(), String>(())
            })();
            drop(output);
            if copied.is_err() {
                let _ = fs::remove_file(&dest);
            }
            copied
        })();
        match result {
            Ok(()) => count += 1,
            Err(e) => failures.push(format!("{}: {e}", source.display())),
        }
    }
    failures.extend(warnings);
    let mut message = format!(
        "Saved {count} asset(s) to {}. Originals were kept.",
        destination.display()
    );
    if !failures.is_empty() {
        message.push_str(&format!(
            " {} item(s) need attention: {}",
            failures.len(),
            failures
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    Ok(Report {
        message,
        failed: !failures.is_empty(),
        remaps: Vec::new(),
    })
}

/// Exact file-open routes are used only where the official desktop frontend supports them.
pub fn direct_open(app: &AppInfo, path: &Path) -> bool {
    let ext = extension(path);
    let common_image = matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "tif" | "tiff" | "webp" | "bmp" | "gif"
    );
    match app.id {
        "photocraft" => {
            common_image
                || app.filetypes.contains(&ext.as_str())
                || matches!(
                    ext.as_str(),
                    "abr"
                        | "grd"
                        | "aco"
                        | "ase"
                        | "kys"
                        | "psp"
                        | "svg"
                        | "tga"
                        | "ico"
                        | "exr"
                        | "hdr"
                        | "heic"
                        | "heif"
                )
        }
        "vectorcraft" => app.filetypes.contains(&ext.as_str()),
        "lightcraft" => common_image || app.filetypes.contains(&ext.as_str()),
        "filmcraft" | "effectcraft" => {
            common_image
                || matches!(
                    ext.as_str(),
                    "mp4"
                        | "mov"
                        | "mkv"
                        | "avi"
                        | "webm"
                        | "wav"
                        | "mp3"
                        | "flac"
                        | "aif"
                        | "aiff"
                )
                || app.filetypes.contains(&ext.as_str())
        }
        "soundcraft" => matches!(
            ext.as_str(),
            "wav" | "aiff" | "aif" | "flac" | "mp3" | "ogg" | "mid" | "midi" | "smf" | "scraft"
        ),
        _ => app.filetypes.contains(&ext.as_str()),
    }
}
pub fn usage(app: &AppInfo, path: &Path) -> &'static str {
    let ext = extension(path);
    match (app.id, ext.as_str()) {
        ("photocraft", "abr" | "grd" | "aco" | "ase" | "kys" | "psp") => {
            "Open this preset in PhotoCraft to import it into the matching preset panel. PhotoCraft reports any format or version errors."
        }
        ("photocraft", "cube" | "3dl" | "look") => {
            "In PhotoCraft, add or edit a Color Lookup adjustment and load this LUT. Paste the copied file path in the file picker."
        }
        ("photocraft", "pat") => {
            "In PhotoCraft, import this file through the Patterns library. Paste the copied file path in its import picker."
        }
        ("photocraft", "pcbrushes") => {
            "This is an internal brush-store file and may depend on companion tips. Export a portable brush preset from PhotoCraft before sharing or importing it into another library."
        }
        ("lightcraft", "lcpreset" | "xmp" | "lrtemplate" | "lmp" | "mplumpack") => {
            "In LightCraft's Presets panel, choose Import Presets and paste the copied file path. Use Export User Presets to save presets back to your asset library."
        }
        (_, "ttf" | "otf" | "woff" | "woff2") => {
            "Use your operating system's font manager to install a supported desktop font, then restart the app. Web fonts may need conversion by their publisher. The asset library keeps your original font file."
        }
        ("vectorcraft", "ase" | "aco") => {
            "Open VectorCraft's Swatches library import command and choose this palette using the copied path."
        }
        _ if direct_open(app, path) => {
            "Opens this library file in the selected app. For use inside an existing document, use that app's Import or Place command with the copied file path. Save As to keep the library original unchanged."
        }
        _ if kind(path) == "Presets" => {
            "Open the matching preset panel in this app and use its Load or Import command with the copied path. Preset formats belong to their original app; check that this app supports the format."
        }
        _ => {
            "Use this app's Import, Place or Insert command and paste the copied file path. The app checks whether this format is supported; this file is not opened automatically."
        }
    }
}
pub fn preset_help(app: &AppInfo) -> &'static str {
    match app.id {
        "photocraft" => {
            "Save or export a preset from the appropriate PhotoCraft panel into this folder, or import an existing preset file. Brushes (.abr), gradients (.grd), and swatches (.aco/.ase) can be sent back through Use in app. LUTs (.cube/.3dl/.look) and patterns (.pat) use their own import panels. Internal .pcbrushes files need their companion tip files; use a portable export when available."
        }
        "lightcraft" => {
            "In LightCraft's Presets panel, choose Export User Presets or Export Group and save the .lcpreset file here. To use it later, choose Use in app and then Import Presets in LightCraft."
        }
        _ => {
            "Export a preset from this app's preset or effects panel and save it here, or use Import presets to keep an existing preset file. To load it again, select Use in app for the import instructions. Keep companion files together by importing their entire folder."
        }
    }
}
