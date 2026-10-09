use super::*;

impl Launcher {
    pub(super) fn onboarding(&mut self, ctx: &egui::Context) {
        if self.active_theme == UiTheme::V2 { self.v2_onboarding(ctx); return; }
        let titles = [
            "Welcome to ArtCraft",
            "Your project folder",
            "Make it yours",
            "Cloud backup",
        ];
        let mut finish = false;
        egui::Modal::new(egui::Id::new("first-run-setup"))
            .backdrop_color(Color32::from_black_alpha(185))
            .frame(egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(UI_RADIUS).inner_margin(32))
            .show(ctx, |ui| {
                ui.set_width(540.0_f32.min(ctx.screen_rect().width() - 100.0));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ARTCRAFT MASTER SUITE").size(text_size(11.0)).color(ACCENT));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} / 4", self.onboarding_step + 1)).size(text_size(12.0)).color(muted()));
                    });
                });
                ui.add_space(12.0);
                ui.heading(RichText::new(tr(titles[self.onboarding_step])).size(text_size(28.0)));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    for step in 0..4 {
                        let width = (ui.available_width() / (4 - step) as f32 - 6.0).max(10.0);
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 3.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, UI_RADIUS, if step <= self.onboarding_step { ACCENT } else { border() });
                    }
                });
                ui.add_space(22.0);
                egui::ScrollArea::vertical().id_salt("setup-content").max_height((ctx.screen_rect().height() - 270.0).max(180.0)).show_scoped(ui, |ui| {
                    ui.set_min_height(260.0);
                    match self.onboarding_step {
                        0 => {
                            ui.label(tr("A few preferences, then you're ready to create."));
                            ui.add_space(24.0);
                            ui.label(RichText::new(tr("Language")).strong());
                            ui.add_space(8.0);
                            egui::ComboBox::from_id_salt("setup-language").width(300.0)
                                .selected_text(localization::name(&self.prefs.language)).show_ui(ui, |ui| {
                                    for (code, name) in localization::LANGUAGES {
                                        ui.selectable_value(&mut self.prefs.language, code.to_owned(), name);
                                    }
                                });
                            localization::select(&self.prefs.language);
                            ui.add_space(24.0);
                            ui.label(RichText::new(tr("You can change these preferences in Settings at any time.")).color(muted()));
                        }
                        1 => {
                            ui.label(tr("Choose where to keep your creative projects."));
                            ui.add_space(16.0);
                            egui::Frame::new().fill(panel()).corner_radius(UI_RADIUS).inner_margin(16).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new(tr("Project folder")).strong());
                                let path = self.prefs.default_project_root.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| tr("No folder selected"));
                                ui.add(egui::Label::new(RichText::new(&path).color(muted())).wrap());
                                ui.add_space(12.0);
                                if secondary_button(ui, "Choose folder").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        self.onboarding_error = None;
                                        // Validate the location before changing the saved default.
                                        let result = APPS.iter().try_for_each(|app| workspace_bridge::prepare(&path, app).map(|_| ()));
                                        match result {
                                            Ok(()) => {
                                                if !self.prefs.roots.contains(&path) { self.prefs.roots.push(path.clone()); }
                                                self.prefs.default_project_root = Some(path);
                                                self.scan_projects();
                                            }
                                            Err(e) => self.onboarding_error = Some(format!("Could not prepare the project folder: {e}")),
                                        }
                                    }
                                }
                            });
                            ui.add_space(16.0);
                            ui.label(RichText::new(tr("A subfolder is created for each app. Existing files stay where they are.")).color(muted()));
                            ui.label(RichText::new(tr("You can also choose a folder later from Projects.")).color(muted()));
                        }
                        2 => {
                            ui.horizontal(|ui| {
                                ui.label(tr("Theme"));
                                ui.selectable_value(&mut self.prefs.light_mode, false, tr("Dark"));
                                ui.selectable_value(&mut self.prefs.light_mode, true, tr("Light"));
                            });
                            ui.add_space(18.0);
                            ui.checkbox(&mut self.prefs.reduce_motion, tr("Reduce motion"));
                            ui.add_space(10.0);
                            ui.checkbox(&mut self.prefs.minimize_to_tray, tr("Minimize to system tray"));
                            ui.add_space(10.0);
                            ui.checkbox(&mut self.prefs.start_with_windows, tr("Start at login in the system tray"));
                            ui.add_space(10.0);
                            ui.checkbox(&mut self.prefs.automatic_suite_updates, tr("Update Master Suite automatically"));
                            ui.add_space(18.0);
                            ui.label(RichText::new(tr("Login startup and minimize-to-tray are off by default.")).color(muted()));
                        }
                        _ => {
                            ui.label(tr("Use your provider’s desktop app to sign in, then choose its sync folder. This step is optional."));
                            ui.add_space(14.0);
                            for provider in cloud::PROVIDERS {
                                egui::Frame::new().fill(panel()).corner_radius(UI_RADIUS).inner_margin(12).show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(provider.name()).strong());
                                            if let Some(account) = self.cloud.settings.folders.get(&provider) {
                                                ui.label(RichText::new(account.path.display().to_string()).size(text_size(12.0)).color(muted()));
                                            } else {
                                                ui.label(RichText::new(tr("Select the folder synced by the desktop app")).size(text_size(12.0)).color(muted()));
                                            }
                                        });
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let connected = self.cloud.settings.folders.contains_key(&provider);
                                            if ui.add_enabled_ui(!self.cloud.busy && !connected, |ui| secondary_button(ui, if connected {"Folder selected"} else {"Choose folder"})).inner.clicked() {
                                                self.cloud.connect(provider);
                                            }
                                        });
                                    });
                                });
                                ui.add_space(8.0);
                            }
                            if self.cloud.busy {
                                ui.horizontal(|ui| {ui.spinner(); ui.label(tr("Preparing backup folder…")); if secondary_button(ui,"Cancel").clicked() {self.cloud.cancel();}});
                            }
                            if !self.cloud.message.is_empty() { ui.label(RichText::new(tr(&self.cloud.message)).size(text_size(12.0)).color(muted())); }
                            ui.add_space(8.0);
                            ui.label(RichText::new(tr("Selecting a folder does not copy any projects. Choose files later in Cloud.")).size(text_size(12.0)).color(muted()));
                        }
                    }
                });
                if let Some(error) = &self.onboarding_error { ui.colored_label(Color32::from_rgb(235,105,112), error); }
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if self.onboarding_step > 0 && secondary_button(ui, "Back").clicked() { self.onboarding_step -= 1; self.onboarding_error = None; }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.onboarding_step == 3 {
                            if ui.add_enabled_ui(!self.cloud.busy, |ui| primary_button(ui, "Get started")).inner.clicked() { finish = true; }
                        } else if primary_button(ui, "Continue").clicked() { self.onboarding_step += 1; self.onboarding_error = None; }
                    });
                });
            });
        if finish {
            #[cfg(target_os = "windows")]
            if let Err(error) = windows_tray::set_startup(self.prefs.start_with_windows) {
                self.onboarding_error = Some(error);
                self.onboarding_step = 2;
                return;
            }
            #[cfg(target_os = "macos")]
            if let Err(error) = macos::set_startup(self.prefs.start_with_windows) {
                self.onboarding_error = Some(error);
                self.onboarding_step = 2;
                return;
            }
            #[cfg(target_os = "linux")]
            if let Err(error) = linux_tray::set_startup(self.prefs.start_with_windows) {
                self.onboarding_error = Some(error);
                self.onboarding_step = 2;
                return;
            }
            self.prefs.onboarding_complete = true;
            // Unlike routine preference saves, completion must report persistence errors.
            let saved = (|| -> Result<(), String> {
                let path = preferences_path().ok_or("Could not locate your settings folder.")?;
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let bytes = serde_json::to_vec_pretty(&self.prefs).map_err(|e| e.to_string())?;
                fs::write(path, bytes).map_err(|e| e.to_string())
            })();
            if let Err(error) = saved {
                self.prefs.onboarding_complete = false;
                self.onboarding_error = Some(format!("Could not save setup: {error}"));
            }
            self.sidebar_collapsed = self.prefs.compact_sidebar;
        }
    }
}
