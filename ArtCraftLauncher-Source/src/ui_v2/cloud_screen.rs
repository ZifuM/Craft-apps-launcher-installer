use super::*;

impl Launcher {
    pub(crate) fn cloud_sidebar(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("cloud-navigation-scroll")
            .show_scoped(ui, |ui| {
                sidebar_caption(ui, "CLOUD", false);
                for (id, label, glyph) in [
                    (0, "Your files", Glyph::Folder),
                    (4, "Selected files", Glyph::Document),
                    (1, "Saved versions", Glyph::Refresh),
                    (3, "Connections", Glyph::Cloud),
                ] {
                    let response =
                        sidebar_item(ui, label, self.direct_cloud.tab == id, glyph, false, None);
                    if response.clicked() {
                        self.direct_cloud.tab = id;
                        if id == 3 {
                            self.direct_cloud.check_on_open();
                        }
                        if id == 1 && !self.direct_cloud.busy {
                            self.direct_cloud.refresh();
                        }
                    }
                }
                sidebar_divider(ui);
                sidebar_caption(ui, "STORAGE", false);
                for target in [
                    cloud_direct::BackupTarget::Google,
                    cloud_direct::BackupTarget::Nas,
                ] {
                    if sidebar_item(
                        ui,
                        target.label(),
                        self.direct_cloud.tab == 3 && self.direct_cloud.settings.target == target,
                        Glyph::Cloud,
                        false,
                        None,
                    )
                    .clicked()
                    {
                        self.direct_cloud.select_target(target);
                        self.direct_cloud.tab = 3;
                        self.direct_cloud.check_on_open();
                    }
                }
                ui.add_space(12.0);
                ui.label(
                    RichText::new(tr(
                        if self.direct_cloud.settings.target == cloud_direct::BackupTarget::Nas {
                            self.direct_cloud.nas_connection.label()
                        } else {
                            self.direct_cloud.connection.label()
                        },
                    ))
                    .small()
                    .color(muted()),
                );
                sidebar_divider(ui);
                if sidebar_item(ui, "Back to Home", false, Glyph::Home, false, None).clicked() {
                    self.page = Page::Home;
                }
            });
    }

    pub(crate) fn cloud_screen(&mut self, ui: &mut egui::Ui) {
        let bounds = ui.available_rect_before_wrap();
        let footer_height = 64.0;
        let body = egui::Rect::from_min_max(
            bounds.min,
            egui::pos2(
                bounds.right(),
                (bounds.bottom() - footer_height - 16.0).max(bounds.top()),
            ),
        );
        let footer =
            egui::Rect::from_min_max(egui::pos2(bounds.left(), body.bottom() + 16.0), bounds.max);
        let mut content = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("cloud-content")
                .max_rect(body),
        );
        content.set_clip_rect(body.intersect(ui.clip_rect()));
        egui::ScrollArea::vertical().id_salt(("cloud-content-scroll", self.direct_cloud.tab))
            .auto_shrink([false, false]).show_scoped(&mut content, |ui| {
                ui.set_width(ui.available_width());
                let ease = transition_progress(ui.ctx(), egui::Id::new("cloud-section-transition"), egui::Id::new(self.direct_cloud.tab), self.prefs.reduce_motion);
                ui.multiply_opacity(0.35 + ease * 0.65);
                let title = match self.direct_cloud.tab { 1 => "Saved versions", 3 => "Connections", 4 => "Selected files", _ => "Your files" };
                ui.heading(tr(title));
                ui.add_space(8.0);
                ui.label(RichText::new(tr(match self.direct_cloud.tab {
                    1 => "Saved versions on your selected backup destination. Restore any version as a separate file.",
                    3 => "Choose Google Drive or a local NAS folder for your backups.",
                    _ => "Select projects, assets and workspace files to back up in their original app folders.",
                })).color(muted()));
                ui.add_space(22.0);
                if !self.direct_cloud.message.is_empty() {
                    egui::Frame::new().fill(panel()).corner_radius(UI_RADIUS).inner_margin(12).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if self.direct_cloud.busy { ui.spinner(); }
                            ui.label(tr(&self.direct_cloud.message));
                            if self.direct_cloud.busy {
                                if ui.small_button(tr("Cancel")).clicked() { self.direct_cloud.cancel(); }
                            } else if ui.small_button(tr("Dismiss")).clicked() { self.direct_cloud.message.clear(); }
                        });
                    });
                    ui.add_space(16.0);
                }
                match self.direct_cloud.tab {
                    1 => self.cloud_versions(ui), 3 => self.cloud_connections(ui), _ => self.cloud_files(ui),
                }
            });

        let selected = self
            .direct_cloud
            .library
            .files
            .iter()
            .filter(|p| self.direct_cloud.settings.selected.contains(&p.path))
            .count();
        ui.painter().rect_filled(footer, 0, panel());
        ui.painter().line_segment(
            [footer.left_top(), footer.right_top()],
            egui::Stroke::new(1.0_f32, border()),
        );
        let mut bottom = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("cloud-sync-footer")
                .max_rect(footer.shrink2(Vec2::new(16.0, 10.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        bottom.vertical(|ui| {
            ui.label(
                RichText::new(tr(format!(
                    "{} file{} selected",
                    selected,
                    if selected == 1 { "" } else { "s" }
                )))
                .strong()
                .color(foreground()),
            );
            ui.label(
                RichText::new(tr(if self.direct_cloud.ready() {
                    format!(
                        "{} · {}",
                        self.direct_cloud.target_label(),
                        if self.direct_cloud.settings.target == cloud_direct::BackupTarget::Nas {
                            "Network backup"
                        } else {
                            "Direct upload"
                        }
                    )
                } else {
                    format!("Connect {} to sync", self.direct_cloud.target_label())
                }))
                .small()
                .color(muted()),
            );
        });
        bottom.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if self.direct_cloud.busy {
                if ui.add(outline_button("Cancel")).clicked() {
                    self.direct_cloud.cancel();
                }
            } else {
                let enabled = selected > 0 && self.direct_cloud.ready() && !self.prefs.scanning;
                if ui.add_enabled(enabled, primary_button("Sync")).clicked() {
                    self.direct_cloud.sync();
                }
            }
            if selected > 0
                && ui
                    .add_enabled(!self.direct_cloud.busy, outline_button("Clear selection"))
                    .clicked()
            {
                self.direct_cloud.settings.selected.clear();
                self.direct_cloud.save();
            }
        });
        if self.direct_cloud.disconnect_open {
            let modal =
                egui::Modal::new(egui::Id::new("disconnect-google-drive")).show(ui.ctx(), |ui| {
                    ui.set_width(360.0);
                    ui.heading(tr("Disconnect Google Drive?"));
                    ui.label(tr(
                        "Uploaded files will stay in your Google Drive. Automatic sync will stop.",
                    ));
                    ui.horizontal(|ui| {
                        if ui.button(tr("Disconnect")).clicked() {
                            self.direct_cloud.disconnect();
                            self.direct_cloud.disconnect_open = false;
                        }
                        if ui.button(tr("Cancel")).clicked() {
                            self.direct_cloud.disconnect_open = false;
                        }
                    });
                });
            if modal.should_close() {
                self.direct_cloud.disconnect_open = false;
            }
        }
    }
    fn cloud_connections(&mut self, ui: &mut egui::Ui) {
        for provider in cloud::PROVIDERS {
            egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(UI_RADIUS).inner_margin(20)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::hover());
                        cloud_ui::paint_provider_icon(ui, rect, provider);
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(provider.name()).strong());
                            let label = if provider != cloud::Provider::Google { tr("Not available yet") }
                                else { self.direct_cloud.settings.account.as_ref().map(|a| a.label.clone()).unwrap_or_else(|| tr("Sign in through your browser")) };
                            ui.label(RichText::new(label).color(muted()));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if provider != cloud::Provider::Google { ui.add_enabled(false, outline_button("Not available yet")); }
                            else { self.google_connection_controls(ui); }
                        });
                    });
                    if provider == cloud::Provider::Google {
                        if let cloud_direct::ConnectionState::Error(error) = &self.direct_cloud.connection {
                            ui.add_space(12.0);
                            ui.label(RichText::new(tr("Google Drive is not connected")).strong().color(ui.visuals().error_fg_color));
                            ui.add(egui::Label::new(RichText::new(tr(error)).color(ui.visuals().error_fg_color)).wrap());
                        }
                        ui.add_space(16.0); ui.separator(); ui.add_space(12.0);
                        ui.label(RichText::new(tr("Uploads mirror your app folders inside ArtCraft Master Suite in your Drive.")).color(muted()));
                        if ui.add_enabled(!self.direct_cloud.busy, egui::RadioButton::new(self.direct_cloud.settings.target == cloud_direct::BackupTarget::Google, tr("Use Google Drive for backups"))).clicked() {
                            self.direct_cloud.select_target(cloud_direct::BackupTarget::Google);
                        }
                    }
                });
            ui.add_space(16.0);
        }
        self.nas_connection_card(ui);
        ui.add_space(16.0);
        egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(UI_RADIUS).inner_margin(20).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(tr(format!("Backup destination: {}", self.direct_cloud.target_label()))).strong());
            ui.add_space(8.0);
            let previous = self.direct_cloud.settings.automatic;
            ui.add_enabled_ui(!self.direct_cloud.busy && self.direct_cloud.ready(), |ui| {
                preferences_toggle(ui, &mut self.direct_cloud.settings.automatic, "Automatically sync changes to selected files");
            });
            if previous != self.direct_cloud.settings.automatic { self.direct_cloud.save(); }
            ui.label(RichText::new(tr("Keep Master Suite running until backups finish. Changing destinations turns automatic sync off.")).small().color(muted()));
        });
    }
    fn nas_connection_card(&mut self, ui: &mut egui::Ui) {
        use cloud_direct::{BackupTarget, ConnectionState};
        egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(UI_RADIUS).inner_margin(20).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr("Local NAS")).strong());
                    ui.label(RichText::new(tr(self.direct_cloud.nas_connection.label())).color(muted()));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let configured = self.direct_cloud.settings.nas.root.is_some();
                    if ui.add_enabled(!self.direct_cloud.busy, outline_button(if configured { "Change folder" } else { "Connect NAS" })).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().set_title("Choose a connected NAS or network folder").pick_folder() {
                            self.direct_cloud.connect_nas(folder);
                        }
                    }
                    if configured {
                        if ui.add_enabled(!self.direct_cloud.busy, outline_button("Check connection")).clicked() { self.direct_cloud.check_nas(); }
                        if ui.add_enabled(!self.direct_cloud.busy, outline_button("Disconnect")).clicked() { self.direct_cloud.disconnect_nas(); }
                    }
                });
            });
            if let Some(root) = &self.direct_cloud.settings.nas.root {
                ui.add_space(10.0);
                ui.add(egui::Label::new(RichText::new(root.display().to_string()).small().color(muted())).wrap());
            }
            if let ConnectionState::Error(error) = &self.direct_cloud.nas_connection {
                ui.add_space(10.0);
                ui.add(egui::Label::new(RichText::new(error).color(ui.visuals().error_fg_color)).wrap());
            }
            ui.add_space(16.0); ui.separator(); ui.add_space(12.0);
            ui.label(RichText::new(tr("Choose a NAS share already connected in your file manager. Projects and assets keep their app folders, with older versions available to restore.")).color(muted()));
            if ui.add_enabled(!self.direct_cloud.busy && self.direct_cloud.settings.nas.root.is_some(), egui::RadioButton::new(self.direct_cloud.settings.target == BackupTarget::Nas, tr("Use Local NAS for backups"))).clicked() {
                self.direct_cloud.select_target(BackupTarget::Nas);
            }
        });
    }
    fn google_connection_controls(&mut self, ui: &mut egui::Ui) {
        use cloud_direct::ConnectionState;
        match self.direct_cloud.connection.clone() {
            ConnectionState::Connected => {
                let color = if light_theme() {
                    Color32::from_rgb(22, 122, 67)
                } else {
                    Color32::from_rgb(115, 212, 161)
                };
                ui.add(
                    egui::Button::new(RichText::new(tr("Connected")).color(color))
                        .fill(mix_color(panel(), color, 0.12))
                        .stroke(egui::Stroke::new(1.0_f32, color))
                        .corner_radius(UI_RADIUS)
                        .min_size(Vec2::new(94.0, 30.0))
                        .sense(egui::Sense::hover()),
                )
                .on_hover_text(tr("Google Drive access has been verified."));
                if ui
                    .add_enabled(!self.direct_cloud.busy, outline_button("Check connection"))
                    .clicked()
                {
                    self.direct_cloud.check_connection();
                }
            }
            state @ (ConnectionState::Connecting
            | ConnectionState::Checking
            | ConnectionState::Disconnecting) => {
                ui.add_enabled(false, primary_button(state.label()));
                ui.spinner();
            }
            ConnectionState::Disconnected | ConnectionState::Error(_) => {
                let retry = matches!(self.direct_cloud.connection, ConnectionState::Error(_));
                let label = if self.direct_cloud.settings.account.is_some() {
                    "Reconnect"
                } else if retry {
                    "Retry connection"
                } else {
                    "Connect"
                };
                if ui
                    .add_enabled(!self.direct_cloud.busy, primary_button(label))
                    .clicked()
                {
                    self.direct_cloud.connect();
                }
                if self.direct_cloud.settings.account.is_some()
                    && ui
                        .add_enabled(!self.direct_cloud.busy, outline_button("Check connection"))
                        .clicked()
                {
                    self.direct_cloud.check_connection();
                }
            }
        }
        if self.direct_cloud.settings.account.is_some()
            && ui
                .add_enabled(!self.direct_cloud.busy, outline_button("Disconnect"))
                .clicked()
        {
            self.direct_cloud.disconnect_open = true;
        }
    }
    fn cloud_files(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("cloud-app-filter")
                .width(150.0)
                .selected_text(if self.direct_cloud.app_filter.is_empty() {
                    tr("All apps")
                } else {
                    app_by_id(&self.direct_cloud.app_filter)
                        .map(|a| a.name.to_owned())
                        .unwrap_or_default()
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.direct_cloud.app_filter,
                        String::new(),
                        tr("All apps"),
                    );
                    for app in APPS {
                        if self
                            .direct_cloud
                            .library
                            .files
                            .iter()
                            .any(|p| p.app.id == app.id)
                        {
                            ui.selectable_value(
                                &mut self.direct_cloud.app_filter,
                                app.id.into(),
                                app.name,
                            );
                        }
                    }
                });
            egui::ComboBox::from_id_salt("cloud-kind-filter")
                .selected_text(if self.direct_cloud.kind_filter.is_empty() {
                    tr("All files")
                } else {
                    tr(&self.direct_cloud.kind_filter)
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.direct_cloud.kind_filter,
                        String::new(),
                        tr("All files"),
                    );
                    for kind in cloud_layout::KINDS {
                        ui.selectable_value(
                            &mut self.direct_cloud.kind_filter,
                            kind.to_owned(),
                            tr(kind),
                        );
                    }
                });
            if ui
                .add_enabled(!self.prefs.scanning, outline_button("Refresh files"))
                .clicked()
            {
                self.scan_projects();
            }
            if self.prefs.scanning {
                ui.spinner();
            }
            ui.selectable_value(&mut self.direct_cloud.list_view, false, tr("Grid"));
            ui.selectable_value(&mut self.direct_cloud.list_view, true, tr("List"));
        });
        ui.add_space(16.0);
        if !self.direct_cloud.library.note.is_empty() {
            ui.label(
                RichText::new(&self.direct_cloud.library.note).color(ui.visuals().warn_fg_color),
            );
        }
        let query = self.direct_cloud.search.trim().to_lowercase();
        let files: Vec<_> = self
            .direct_cloud
            .library
            .files
            .iter()
            .filter(|p| {
                (self.direct_cloud.app_filter.is_empty()
                    || p.app.id == self.direct_cloud.app_filter)
                    && (self.direct_cloud.tab != 4
                        || self.direct_cloud.settings.selected.contains(&p.path))
                    && (self.direct_cloud.kind_filter.is_empty()
                        || self
                            .direct_cloud
                            .library
                            .destinations
                            .get(&p.path)
                            .is_some_and(|d| d.kind() == self.direct_cloud.kind_filter))
                    && (p.title.to_lowercase().contains(&query)
                        || p.path.to_string_lossy().to_lowercase().contains(&query))
            })
            .cloned()
            .collect();
        let mut all = !files.is_empty()
            && files
                .iter()
                .all(|p| self.direct_cloud.settings.selected.contains(&p.path));
        if ui
            .add_enabled(
                !self.direct_cloud.busy && !files.is_empty(),
                egui::Checkbox::new(&mut all, tr("Select all visible files")),
            )
            .changed()
        {
            for p in &files {
                self.direct_cloud
                    .settings
                    .selected
                    .retain(|path| path != &p.path);
                if all {
                    self.direct_cloud.settings.selected.push(p.path.clone());
                }
            }
            self.direct_cloud.save();
        }
        ui.add_space(14.0);
        if files.is_empty() {
            ui.label(tr(
                "No matching files. Add your workspace folders on the Projects screen, then refresh.",
            ));
            return;
        }
        if self.direct_cloud.list_view {
            for p in &files {
                ui.push_id(&p.path, |ui| {
                    egui::Frame::new()
                        .fill(panel())
                        .corner_radius(UI_RADIUS)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let mut selected =
                                    self.direct_cloud.settings.selected.contains(&p.path);
                                if ui
                                    .add_enabled(
                                        !self.direct_cloud.busy,
                                        egui::Checkbox::without_text(&mut selected),
                                    )
                                    .changed()
                                {
                                    self.cloud_select(p, selected);
                                }
                                self.project_thumbnail_at(ui, p, Vec2::new(54.0, 36.0));
                                ui.vertical(|ui| {
                                    ui.label(&p.title);
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(self.direct_cloud.folder_label(p))
                                                .small()
                                                .color(muted()),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(self.direct_cloud.folder_label(p));
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {} · {}",
                                            p.app.name,
                                            format_file_size(p.size_bytes),
                                            tr(self.direct_cloud.status(p))
                                        ))
                                        .small()
                                        .color(muted()),
                                    );
                                    if let Some(error) = self.direct_cloud.errors.get(&p.path) {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(error)
                                                    .small()
                                                    .color(ui.visuals().error_fg_color),
                                            )
                                            .wrap(),
                                        );
                                    }
                                });
                            });
                        });
                });
                ui.add_space(8.0);
            }
        } else {
            let width = ui.available_width().min(250.0);
            let gap = 18.0;
            let count = (((ui.available_width() + gap) / (width + gap)).floor() as usize)
                .max(1)
                .min(files.len());
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.set_width(count as f32 * width + (count - 1) as f32 * gap);
                ui.columns(count, |columns| {
                    for (index, p) in files.iter().enumerate() {
                        let ui = &mut columns[index % count];
                        ui.push_id(&p.path, |ui| {
                            let selected = self.direct_cloud.settings.selected.contains(&p.path);
                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::new(width, width * 9.0 / 16.0),
                                if self.direct_cloud.busy {
                                    egui::Sense::hover()
                                } else {
                                    egui::Sense::click()
                                },
                            );
                            ui.painter().rect_filled(rect, UI_RADIUS, panel());
                            if let Some(preview) = &p.preview {
                                let texture = self
                                    .project_previews
                                    .entry(p.path.clone())
                                    .or_insert_with(|| {
                                        ui.ctx().load_texture(
                                            format!("cloud-preview-{}", p.path.display()),
                                            egui::ColorImage::from_rgba_unmultiplied(
                                                [
                                                    preview.width() as usize,
                                                    preview.height() as usize,
                                                ],
                                                preview.as_raw(),
                                            ),
                                            egui::TextureOptions::LINEAR,
                                        )
                                    });
                                let size = texture.size_vec2();
                                let scale = (rect.width() / size.x).min(rect.height() / size.y);
                                ui.painter().image(
                                    texture.id(),
                                    egui::Rect::from_center_size(rect.center(), size * scale),
                                    egui::Rect::from_min_max(
                                        egui::Pos2::ZERO,
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    Color32::WHITE,
                                );
                                mask_banner_corners(ui.painter(), rect, ink());
                            } else {
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    p.app.name,
                                    egui::FontId::proportional(16.0),
                                    muted(),
                                );
                            }
                            ui.painter().rect_stroke(
                                rect,
                                UI_RADIUS,
                                egui::Stroke::new(
                                    if selected { 2.0_f32 } else { 1.0_f32 },
                                    if selected { accent() } else { border() },
                                ),
                                egui::StrokeKind::Inside,
                            );
                            let check = egui::Rect::from_min_size(
                                rect.min + Vec2::splat(10.0),
                                Vec2::splat(22.0),
                            );
                            ui.painter().rect_filled(
                                check,
                                UI_RADIUS,
                                if selected { accent() } else { panel() },
                            );
                            ui.painter().rect_stroke(
                                check,
                                UI_RADIUS,
                                egui::Stroke::new(
                                    1.0_f32,
                                    if selected { accent() } else { muted() },
                                ),
                                egui::StrokeKind::Inside,
                            );
                            if selected {
                                ui.painter().add(egui::Shape::line(
                                    vec![
                                        check.min + Vec2::new(5.0, 11.0),
                                        check.min + Vec2::new(9.0, 15.0),
                                        check.min + Vec2::new(17.0, 7.0),
                                    ],
                                    egui::Stroke::new(2.0_f32, Color32::WHITE),
                                ));
                            }
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::Checkbox,
                                    !self.direct_cloud.busy,
                                    selected,
                                    &p.title,
                                )
                            });
                            if response
                                .on_hover_text(p.path.display().to_string())
                                .clicked()
                            {
                                self.cloud_select(p, !selected);
                            }
                            ui.add_space(8.0);
                            ui.add(egui::Label::new(RichText::new(&p.title).strong()).truncate())
                                .on_hover_text(p.path.display().to_string());
                            ui.add(
                                egui::Label::new(
                                    RichText::new(self.direct_cloud.folder_label(p))
                                        .small()
                                        .color(muted()),
                                )
                                .truncate(),
                            )
                            .on_hover_text(self.direct_cloud.folder_label(p));
                            ui.label(
                                RichText::new(format!(
                                    "{} · {}",
                                    p.app.name,
                                    format_file_size(p.size_bytes)
                                ))
                                .small()
                                .color(muted()),
                            );
                            let status = ui.label(
                                RichText::new(tr(self.direct_cloud.status(p)))
                                    .small()
                                    .color(if selected { accent() } else { muted() }),
                            );
                            if let Some(error) = self.direct_cloud.errors.get(&p.path) {
                                status.on_hover_text(error);
                            }
                            ui.add_space(24.0);
                        });
                    }
                });
            });
        }
    }
    fn cloud_select(&mut self, project: &Project, selected: bool) {
        self.direct_cloud
            .settings
            .selected
            .retain(|p| p != &project.path);
        if selected {
            self.direct_cloud
                .settings
                .selected
                .push(project.path.clone());
        }
        self.direct_cloud.save();
    }
    fn cloud_versions(&mut self, ui: &mut egui::Ui) {
        if ui
            .add_enabled(
                !self.direct_cloud.busy && self.direct_cloud.ready(),
                outline_button("Refresh saved versions"),
            )
            .clicked()
        {
            self.direct_cloud.refresh();
        }
        ui.add_space(18.0);
        let query = self.direct_cloud.search.trim().to_lowercase();
        let versions: Vec<_> = self
            .direct_cloud
            .saved_versions()
            .iter()
            .filter(|v| v.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        if versions.is_empty() {
            ui.label(tr("No saved versions to show."));
        }
        for saved in versions {
            ui.push_id(&saved.id, |ui| {
                egui::Frame::new()
                    .fill(panel())
                    .stroke(egui::Stroke::new(1.0_f32, border()))
                    .corner_radius(UI_RADIUS)
                    .inner_margin(16)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::Label::new(RichText::new(&saved.name).strong())
                                        .truncate(),
                                );
                                let date = DateTime::parse_from_rfc3339(&saved.modified)
                                    .map(|d| {
                                        d.with_timezone(&Local)
                                            .format("%d %b %Y · %H:%M")
                                            .to_string()
                                    })
                                    .unwrap_or_else(|_| saved.modified.clone());
                                ui.label(
                                    RichText::new(format!(
                                        "{} · {}",
                                        format_file_size(saved.bytes),
                                        date
                                    ))
                                    .small()
                                    .color(muted()),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add_enabled(
                                            !self.direct_cloud.busy && self.direct_cloud.ready(),
                                            outline_button("Restore copy"),
                                        )
                                        .clicked()
                                    {
                                        // A provider-controlled filename is only a suggestion, never a destination path.
                                        let name: String = saved
                                            .name
                                            .chars()
                                            .map(|c| {
                                                if c.is_control() || "/\\:<>\"|?*".contains(c) {
                                                    '_'
                                                } else {
                                                    c
                                                }
                                            })
                                            .collect();
                                        if let Some(path) =
                                            rfd::FileDialog::new().set_file_name(name).save_file()
                                        {
                                            self.direct_cloud.restore(saved.clone(), path);
                                        }
                                    }
                                },
                            );
                        });
                    });
            });
            ui.add_space(10.0);
        }
    }
    pub(crate) fn cloud_notifications(&mut self, ctx: &egui::Context) -> f32 {
        let mut offset = 14.0;
        let notices = self.direct_cloud.notices.clone();
        for notice in notices.iter().rev().take(3) {
            let shown = egui::Area::new(egui::Id::new(("cloud-sync-notice", notice.id)))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-14.0, -offset))
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(panel())
                        .stroke(egui::Stroke::new(1.0_f32, border()))
                        .corner_radius(UI_RADIUS)
                        .inner_margin(16)
                        .show(ui, |ui| {
                            ui.set_width((ctx.screen_rect().width() - 64.0).clamp(180.0, 350.0));
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(tr(&notice.title)).strong());
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if notice.finished.is_some() {
                                            if ui.small_button(tr("Dismiss")).clicked() {
                                                self.direct_cloud
                                                    .notices
                                                    .retain(|n| n.id != notice.id);
                                            }
                                        } else if ui.small_button(tr("Cancel")).clicked() {
                                            self.direct_cloud.cancel();
                                        }
                                    },
                                );
                            });
                            ui.add_space(6.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(tr(&notice.detail)).small().color(muted()),
                                )
                                .wrap(),
                            );
                            ui.add_space(10.0);
                            ui.add(
                                egui::ProgressBar::new(notice.progress)
                                    .fill(if notice.failed {
                                        Color32::from_rgb(196, 79, 73)
                                    } else {
                                        accent()
                                    })
                                    .show_percentage(),
                            );
                            if let Some(finished) = notice.finished {
                                let remaining =
                                    (5.0 - finished.elapsed().as_secs_f32()).max(0.0).ceil() as u32;
                                ui.label(
                                    RichText::new(tr(format!("Closing in {remaining}s")))
                                        .small()
                                        .color(muted()),
                                );
                            }
                        });
                });
            offset += shown.response.rect.height() + 12.0;
        }
        if !notices.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        offset - 14.0
    }
}
