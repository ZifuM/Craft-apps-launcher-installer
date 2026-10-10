use super::*;

fn plugin_panel() -> egui::Frame {
    egui::Frame::new()
        .fill(panel())
        .stroke(egui::Stroke::new(1.0_f32, border()))
        .corner_radius(UI_RADIUS)
        .inner_margin(20)
}
impl Launcher {
    pub(crate) fn v2_plugins(&mut self, ui: &mut egui::Ui, app: AppInfo) {
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(tr(format!("Plugins for {}", app.name)))
                    .size(text_size(18.0))
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.hyperlink_to(tr("Plugin documentation"), plugins::documentation(&app));
            });
        });
        ui.label(
            RichText::new(tr("Install and manage extensions for this workspace.")).color(muted()),
        );
        ui.add_space(20.0);
        if !plugins::supported(app.id) {
            plugin_panel().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(tr("Plugin installation is not available for this app yet.")).strong());
                ui.add_space(6.0);
                ui.label(RichText::new(tr("A documented compatible plugin loader is required. Check the official app documentation for supported extensions.")).color(muted()));
            });
            return;
        }
        let Some(root) = self.prefs.default_project_root.clone() else {
            ui.label(tr(
                "Choose your project folder in Settings before installing plugins.",
            ));
            if ui.add(primary_button("Open Settings")).clicked() {
                self.settings_open = true;
                self.settings_tab = 2;
            }
            return;
        };
        let installed = self
            .states
            .get(app.id)
            .is_some_and(|s| s.installed.is_some());
        plugin_panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(tr("Add a plugin")).strong());
            ui.add_space(6.0);
            ui.label(RichText::new(tr(plugins::formats(app.id))).color(muted()));
            ui.add_space(16.0);
            ui.label(tr("GitHub link"));
            ui.add_space(4.0);
            ui.add_enabled(!self.plugins.busy, egui::TextEdit::singleline(&mut self.plugins.link)
                .id_salt(("plugin-link", app.id)).hint_text("https://github.com/owner/plugin")
                .desired_width(ui.available_width()).margin(Vec2::new(10.0, 7.0)));
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                let enabled = installed && !self.plugins.busy && self.plugins.choices.is_empty();
                if ui.add_enabled(enabled && !self.plugins.link.trim().is_empty(), primary_button("Install from GitHub")).clicked() {
                    self.plugins.install(app, root.clone(), self.plugins.link.trim().into(), false);
                }
                if ui.add_enabled(enabled, outline_button("Install from file")).clicked() {
                    let types: &[&str] = if app.id == "soundcraft" { &["clap", "vst3", "zip"] } else { &["wasm", "zip"] };
                    if let Some(file) = rfd::FileDialog::new().set_title(format!("Install a plugin for {}", app.name)).add_filter("Plugin packages", types).pick_file() {
                        self.plugins.install(app, root.clone(), file.display().to_string(), true);
                    }
                }
                if app.id == "soundcraft" && ui.add_enabled(enabled, outline_button("Install bundle folder")).clicked() {
                    if let Some(folder) = rfd::FileDialog::new().set_title("Choose a .vst3, .clap or .component bundle").pick_folder() {
                        self.plugins.install(app, root.clone(), folder.display().to_string(), true);
                    }
                }
            });
            ui.add_space(12.0);
            ui.label(RichText::new(tr(if !installed { "Install this app from All Apps first." }
                else if app.id == "soundcraft" { "Choose plugins for this computer. Audio plugins are shared with other audio apps. Close SoundCraft before making changes." }
                else { "Use a compiled plugin made for this app. Close the app before installing, enabling or removing plugins." })).small().color(muted()));
        });
        if !self.plugins.message.is_empty() {
            ui.add_space(14.0);
            egui::Frame::new()
                .fill(panel())
                .corner_radius(UI_RADIUS)
                .inner_margin(12)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        if self.plugins.busy {
                            ui.spinner();
                            ui.ctx().request_repaint_after(Duration::from_millis(100));
                        }
                        let color = if self.plugins.failed {
                            ui.visuals().error_fg_color
                        } else {
                            foreground()
                        };
                        ui.label(RichText::new(tr(&self.plugins.message)).color(color));
                        if !self.plugins.busy && ui.add(outline_button("Dismiss")).clicked() {
                            self.plugins.message.clear();
                        }
                    });
                });
        }
        ui.add_space(14.0);
        egui::CollapsingHeader::new(tr("Installation folders")).id_salt(("plugin-folders", app.id)).show(ui, |ui| {
            match plugins::locations(&app, &root) {
                Ok(locations) => for (kind, folder) in locations {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(kind).strong());
                        ui.label(RichText::new(folder.display().to_string()).small().color(muted()));
                        if ui.add_enabled(folder.exists(), outline_button("Open folder")).clicked() { reveal_project_path(&folder, false); }
                    });
                },
                Err(error) => { ui.label(RichText::new(error).color(ui.visuals().error_fg_color)); }
            }
            ui.add_space(8.0);
            ui.label(RichText::new(tr("Master Suite keeps a copy in this app's workspace Plugins folder. Existing custom plugin locations are preserved.")).small().color(muted()));
        });
        section(ui, "Installed plugins");
        search(
            ui,
            "plugin-search",
            &mut self.plugins.search,
            "Search installed plugins",
            ui.available_width().min(400.0),
        );
        ui.add_space(12.0);
        match plugins::entries(&app) {
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            Ok(entries) => {
                let filter = self.plugins.search.trim().to_lowercase();
                let visible: Vec<_> = entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.name.to_lowercase().contains(&filter))
                    .collect();
                if visible.is_empty() {
                    plugin_panel().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(tr(if entries.is_empty() {
                            "No plugins installed by Master Suite yet."
                        } else {
                            "No plugins match your search."
                        }));
                        if entries.is_empty() {
                            ui.label(
                                RichText::new(tr(
                                    "Paste a GitHub link or choose a plugin file to get started.",
                                ))
                                .color(muted()),
                            );
                        }
                    });
                }
                for (index, entry) in visible {
                    ui.push_id((app.id, index), |ui| {
                        plugin_panel().show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let exists = plugins::active_path(entry).exists();
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(&entry.name).strong());
                                ui.label(RichText::new(tr(if !exists { "Missing files" } else if entry.enabled { "Enabled" } else { "Disabled" })).small().color(muted()));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add_enabled(!self.plugins.busy, outline_button("Remove")).clicked() { self.plugins.remove = Some((app.id.into(), index)); }
                                    if ui.add_enabled(exists && !self.plugins.busy, outline_button(if entry.enabled { "Disable" } else { "Enable" })).clicked() {
                                        match plugins::toggle(&app, index) {
                                            Ok(()) => { self.plugins.failed = false; self.plugins.message = tr("Plugin updated. Restart the app to apply the change."); }
                                            Err(error) => { self.plugins.failed = true; self.plugins.message = error; }
                                        }
                                    }
                                });
                            });
                            ui.add_space(8.0);
                            ui.label(RichText::new(format!("{} · {}", if entry.format.is_empty() { "WASM".into() } else { entry.format.to_uppercase() }, entry.deployed.display())).small().color(muted()));
                            ui.horizontal_wrapped(|ui| {
                                if entry.source.starts_with("https://github.com/") || entry.source.starts_with("https://raw.githubusercontent.com/") { ui.hyperlink_to(tr("Source on GitHub"), &entry.source); }
                                if ui.add_enabled(exists, outline_button("Show file")).clicked() { reveal_project_path(&plugins::active_path(entry), true); }
                            });
                        });
                    });
                    ui.add_space(10.0);
                }
            }
        }
        self.plugin_dialogs(ui, app);
    }
    fn plugin_dialogs(&mut self, ui: &mut egui::Ui, app: AppInfo) {
        if !self.plugins.choices.is_empty() {
            let mut chosen = None;
            let mut cancel = false;
            let modal =
                egui::Modal::new(egui::Id::new("plugin-download-picker")).show(ui.ctx(), |ui| {
                    ui.set_width(620.0_f32.min(ui.ctx().screen_rect().width() - 60.0));
                    ui.heading(tr("Choose a plugin download"));
                    ui.label(tr(format!(
                        "Select the compiled package for {} on {} / {}.",
                        self.plugins
                            .target
                            .as_ref()
                            .map(|(app, _)| app.name)
                            .unwrap_or(app.name),
                        std::env::consts::OS,
                        std::env::consts::ARCH
                    )));
                    ui.add_space(12.0);
                    egui::ScrollArea::vertical()
                        .max_height(320.0)
                        .show(ui, |ui| {
                            for download in &self.plugins.choices {
                                ui.horizontal(|ui| {
                                    ui.allocate_ui_with_layout(
                                        Vec2::new((ui.available_width() - 92.0).max(100.0), 40.0),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.add(egui::Label::new(&download.name).wrap());
                                            ui.label(
                                                RichText::new(format!(
                                                    "{:.1} MB",
                                                    download.bytes as f64 / 1048576.0
                                                ))
                                                .small()
                                                .color(muted()),
                                            );
                                        },
                                    );
                                    if ui.add(outline_button("Install")).clicked() {
                                        chosen = Some(download.clone());
                                    }
                                });
                                ui.separator();
                            }
                        });
                    ui.add_space(12.0);
                    cancel = ui.add(outline_button("Cancel")).clicked();
                });
            if let Some(download) = chosen {
                self.plugins.choose(download);
            } else if cancel || modal.should_close() {
                self.plugins.choices.clear();
                self.plugins.message.clear();
            }
        }
        if let Some((id, index)) = self.plugins.remove.clone() {
            if id != app.id {
                return;
            }
            let mut remove = false;
            let modal = egui::Modal::new(egui::Id::new("remove-managed-plugin")).show(ui.ctx(), |ui| {
                ui.set_width(400.0);
                ui.heading(tr("Remove plugin?"));
                ui.label(tr("Remove the installed plugin and its managed workspace copy. Your projects and original download are kept."));
                if app.id == "soundcraft" { ui.label(tr("Other audio apps using this plugin will also lose access to it.")); }
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    remove = ui.add(primary_button("Remove plugin")).clicked();
                    if ui.add(outline_button("Cancel")).clicked() { self.plugins.remove = None; }
                });
            });
            if remove {
                let result = plugins::uninstall(&app, index);
                self.plugins.failed = result.is_err();
                self.plugins.message = result.map(|()| tr("Plugin removed.")).unwrap_or_else(|e| e);
                self.plugins.remove = None;
            } else if modal.should_close() {
                self.plugins.remove = None;
            }
        }
    }
}
