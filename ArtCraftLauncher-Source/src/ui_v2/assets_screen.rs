use super::*;

impl Launcher {
    pub(crate) fn asset_roots(&self) -> Vec<PathBuf> {
        let mut roots = self.prefs.roots.clone();
        if let Some(root) = &self.prefs.default_project_root {
            if !roots.contains(root) {
                roots.push(root.clone());
            }
        }
        roots
    }
    fn asset_import_dialog(&mut self, app: Option<AppInfo>, presets: bool, folder: bool) {
        let dialog = rfd::FileDialog::new().set_title(if folder {
            "Import an asset folder"
        } else if presets {
            "Import presets"
        } else {
            "Import assets"
        });
        let paths = if folder {
            dialog.pick_folder().map(|p| vec![p])
        } else if presets {
            dialog
                .add_filter(
                    "Presets and palettes",
                    &[
                        "abr",
                        "grd",
                        "pat",
                        "aco",
                        "ase",
                        "asl",
                        "atn",
                        "kys",
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
                        "json",
                        "zip",
                    ],
                )
                .add_filter("All files", &["*"])
                .pick_files()
        } else {
            dialog.pick_files()
        };
        if let Some(paths) = paths {
            self.assets.import = Some(assets::Import {
                paths,
                presets,
                app: app.map(|a| a.id).unwrap_or("photocraft").into(),
            });
        }
    }
    pub(crate) fn v2_assets(&mut self, ui: &mut egui::Ui, app: Option<AppInfo>) {
        if app.is_none() {
            heading(
                ui,
                "Assets",
                "Images, media and reusable presets from your app libraries.",
            );
        }
        if self
            .folder_move
            .as_ref()
            .is_some_and(|operation| operation.running())
        {
            ui.label(tr(
                "Your library folder is moving. Assets will be available when the move finishes.",
            ));
            return;
        }
        let Some(root) = self.prefs.default_project_root.clone() else {
            ui.label(tr(
                "Choose a project folder in Settings to create your asset libraries.",
            ));
            if ui.add(primary_button("Choose library folder")).clicked() {
                self.settings_open = true;
                self.settings_tab = 2;
            }
            return;
        };
        ui.horizontal(|ui| {
            if tab(ui, !self.assets.folders_tab, "Library").clicked() {
                self.assets.folders_tab = false;
            }
            if tab(ui, self.assets.folders_tab, "Folders").clicked() {
                self.assets.folders_tab = true;
            }
        });
        if self.assets.folders_tab {
            scroll(ui, ("asset-folders", app.map(|a| a.id)), |ui| {
                ui.label(tr("Files saved into these folders appear in Assets after refresh. You can also save directly here from an app."));
                ui.add_space(14.0);
                for (owner, folder) in assets::folders(&self.asset_roots())
                    .into_iter()
                    .filter(|(owner, _)| app.is_none_or(|a| a.id == owner.id))
                {
                    egui::Frame::new()
                        .fill(panel())
                        .stroke(egui::Stroke::new(1.0_f32, border()))
                        .corner_radius(UI_RADIUS)
                        .inner_margin(16)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(RichText::new(owner.name).strong());
                            ui.add(
                                egui::Label::new(
                                    RichText::new(folder.display().to_string()).color(muted()),
                                )
                                .wrap(),
                            );
                            ui.horizontal_wrapped(|ui| {
                                if ui.add(outline_button("Open folder")).clicked() {
                                    reveal_project_path(&folder, false);
                                }
                                if ui.add(outline_button("Copy path")).clicked() {
                                    ui.ctx().copy_text(folder.display().to_string());
                                }
                                let trash = folder.join(".Trash");
                                if trash.is_dir()
                                    && ui.add(outline_button("Open library trash")).clicked()
                                {
                                    reveal_project_path(&trash, false);
                                }
                            });
                        });
                    ui.add_space(10.0);
                }
                if ui.add(outline_button("Manage library location")).clicked() {
                    self.settings_open = true;
                    self.settings_tab = 2;
                }
            });
            return;
        }
        if self.active_theme == UiTheme::Legacy {
            search(
                ui,
                "asset-search",
                &mut self.assets.search,
                "Search assets",
                400.0_f32.min(ui.available_width()),
            );
        }
        ui.horizontal_wrapped(|ui| {
            if app.is_none() {
                egui::ComboBox::from_id_salt("asset-app-filter")
                    .selected_text(&self.assets.app_filter)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.assets.app_filter,
                            "All apps".into(),
                            tr("All apps"),
                        );
                        for candidate in APPS {
                            ui.selectable_value(
                                &mut self.assets.app_filter,
                                candidate.name.into(),
                                candidate.name,
                            );
                        }
                    });
            }
            egui::ComboBox::from_id_salt("asset-type")
                .selected_text(tr(&self.assets.type_filter))
                .show_ui(ui, |ui| {
                    for kind in assets::KINDS {
                        ui.selectable_value(
                            &mut self.assets.type_filter,
                            (*kind).into(),
                            tr(*kind),
                        );
                    }
                });
            egui::ComboBox::from_id_salt("asset-scope")
                .selected_text(tr(&self.assets.scope))
                .show_ui(ui, |ui| {
                    for scope in ["All assets", "Favorites", "Last 7 days"] {
                        ui.selectable_value(&mut self.assets.scope, scope.into(), tr(scope));
                    }
                });
            egui::ComboBox::from_id_salt("asset-sort")
                .selected_text(tr(&self.assets.sort))
                .show_ui(ui, |ui| {
                    for sort in ["Recently modified", "Name A-Z", "Largest first", "By app"] {
                        ui.selectable_value(&mut self.assets.sort, sort.into(), tr(sort));
                    }
                });
            if view_options(ui, &mut self.prefs.asset_view) {
                save_preferences(&self.prefs);
            }
            if ui
                .add_enabled(
                    !self.assets.busy && !self.assets.scanning(),
                    outline_button("Refresh"),
                )
                .clicked()
            {
                self.assets.refresh(self.asset_roots());
            }
            if ui
                .add_enabled(!self.assets.busy, primary_button("Import files"))
                .clicked()
            {
                self.asset_import_dialog(app, false, false);
            }
            if ui
                .add_enabled(!self.assets.busy, outline_button("Import folder"))
                .clicked()
            {
                self.asset_import_dialog(app, false, true);
            }
            if ui
                .add_enabled(!self.assets.busy, outline_button("Import presets"))
                .clicked()
            {
                self.asset_import_dialog(app, true, false);
            }
            if ui.add(outline_button("Save presets from app")).clicked() {
                self.assets.preset_help = Some(app.map(|a| a.id).unwrap_or("photocraft").into());
            }
        });
        ui.add_space(10.0);
        if !self.assets.message.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(&self.assets.message).color(if self.assets.failed {
                        ui.visuals().error_fg_color
                    } else {
                        muted()
                    }),
                );
                if !self.assets.busy && ui.small_button(tr("Dismiss")).clicked() {
                    self.assets.message.clear();
                }
            });
            if self.assets.busy {
                ui.add(
                    egui::ProgressBar::new(self.assets.progress)
                        .desired_width(ui.available_width())
                        .animate(true),
                );
            }
        }
        if !self.assets.library.note.is_empty() {
            ui.label(
                RichText::new(&self.assets.library.note)
                    .small()
                    .color(muted()),
            );
        }
        let query = self.assets.search.trim().to_lowercase();
        let cutoff = Local::now() - chrono::Duration::days(7);
        let mut visible: Vec<_> = self
            .assets
            .library
            .items
            .iter()
            .filter(|a| {
                app.map_or(
                    self.assets.app_filter == "All apps"
                        || self.assets.app_filter == a.file.app.name,
                    |app| a.file.app.id == app.id,
                ) && (self.assets.type_filter == "All types" || self.assets.type_filter == a.kind)
                    && (a
                        .file
                        .path
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&query)
                        || a.kind.to_lowercase().contains(&query)
                        || a.file.app.name.to_lowercase().contains(&query))
                    && match self.assets.scope.as_str() {
                        "Favorites" => self.prefs.favorite_assets.contains(&a.file.path),
                        "Last 7 days" => a.file.modified.is_some_and(|d| d >= cutoff),
                        _ => true,
                    }
            })
            .cloned()
            .collect();
        match self.assets.sort.as_str() {
            "Name A-Z" => visible.sort_by_key(|a| a.file.title.to_lowercase()),
            "Largest first" => visible.sort_by_key(|a| std::cmp::Reverse(a.file.size_bytes)),
            "By app" => visible.sort_by_key(|a| (a.file.app.name, a.file.title.to_lowercase())),
            _ => visible.sort_by_key(|a| std::cmp::Reverse(a.file.modified)),
        }
        let selected: Vec<_> = visible
            .iter()
            .filter(|a| self.assets.selected.contains(&a.file.path))
            .cloned()
            .collect();
        ui.horizontal_wrapped(|ui| {
            let mut all = !visible.is_empty() && selected.len() == visible.len();
            if ui.checkbox(&mut all, tr("Select all")).changed() {
                for a in &visible {
                    if all {
                        self.assets.selected.insert(a.file.path.clone());
                    } else {
                        self.assets.selected.remove(&a.file.path);
                    }
                }
            }
            ui.label(
                RichText::new(format!(
                    "{} assets · {} selected",
                    visible.len(),
                    selected.len()
                ))
                .small()
                .color(muted()),
            );
            if self.assets.scanning() {
                ui.spinner();
            }
            if ui
                .add_enabled(
                    selected.len() == 1 && !self.assets.busy,
                    outline_button("Use in app"),
                )
                .clicked()
            {
                self.begin_asset_use(selected[0].clone());
            }
            if ui
                .add_enabled(
                    !selected.is_empty() && !self.assets.busy,
                    outline_button("Save copies…"),
                )
                .clicked()
            {
                if let Some(folder) = rfd::FileDialog::new()
                    .set_title("Save copies of selected assets")
                    .pick_folder()
                {
                    self.assets.export_files(
                        selected.iter().map(|a| a.file.path.clone()).collect(),
                        folder,
                    );
                }
            }
            if ui
                .add_enabled(
                    !selected.is_empty() && !self.assets.busy,
                    outline_button("Remove"),
                )
                .clicked()
            {
                self.assets.remove = selected.clone();
            }
        });
        ui.separator();
        let view = self.prefs.asset_view;
        scroll(ui, ("asset-results", app.map(|a| a.id)), |ui| {
            if visible.is_empty() {
                ui.add_space(24.0);
                ui.label(RichText::new(tr("No assets to show")).size(text_size(18.0)));
                ui.label(tr("Import files or folders, drop files here, or save from an app into its Assets folder. Change the filters to see other assets."));
            } else if view == ProjectView::List {
                self.asset_list(ui, &visible);
            } else {
                self.asset_grid(ui, &visible, view == ProjectView::Waterfall);
            }
        });
        if !self.settings_open && !self.assets.busy && self.assets.import.is_none() {
            let drops: Vec<_> = ui.ctx().input(|i| {
                i.raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect()
            });
            if !drops.is_empty() {
                self.assets.import = Some(assets::Import {
                    paths: drops,
                    app: app.map(|a| a.id).unwrap_or("photocraft").into(),
                    presets: false,
                });
            }
        }
        self.asset_dialogs(ui, &root);
    }
    fn begin_asset_use(&mut self, asset: assets::Asset) {
        let app = asset.file.app.id.into();
        self.assets.use_asset = Some(assets::Use { asset, app });
    }
    fn asset_favorite(&mut self, path: &Path) {
        if self.prefs.favorite_assets.iter().any(|p| p == path) {
            self.prefs.favorite_assets.retain(|p| p != path);
        } else {
            self.prefs.favorite_assets.push(path.to_path_buf());
        }
        save_preferences(&self.prefs);
    }
    fn asset_menu(&mut self, ui: &mut egui::Ui, asset: &assets::Asset) {
        if ui.button(tr("Use in app…")).clicked() {
            self.begin_asset_use(asset.clone());
            ui.close_menu();
        }
        if ui
            .button(tr(
                if self.prefs.favorite_assets.contains(&asset.file.path) {
                    "Remove favorite"
                } else {
                    "Add favorite"
                },
            ))
            .clicked()
        {
            self.asset_favorite(&asset.file.path);
            ui.close_menu();
        }
        if ui.button(tr(platform::reveal_label())).clicked() {
            reveal_project_path(&asset.file.path, true);
            ui.close_menu();
        }
        if ui.button(tr("Copy file path")).clicked() {
            ui.ctx().copy_text(asset.file.path.display().to_string());
            ui.close_menu();
        }
        if ui
            .add_enabled(!self.assets.busy, egui::Button::new(tr("Save a copy…")))
            .clicked()
        {
            if let Some(folder) = rfd::FileDialog::new()
                .set_title("Save a copy of this asset")
                .pick_folder()
            {
                self.assets
                    .export_files(vec![asset.file.path.clone()], folder);
            }
            ui.close_menu();
        }
        ui.separator();
        if ui
            .add_enabled(!self.assets.busy, egui::Button::new(tr("Rename…")))
            .clicked()
        {
            self.assets.rename = Some(assets::Rename {
                asset: asset.clone(),
                name: asset.file.title.clone(),
            });
            ui.close_menu();
        }
        if ui
            .add_enabled(
                !self.assets.busy,
                egui::Button::new(tr("Remove from library…")),
            )
            .clicked()
        {
            self.assets.remove = vec![asset.clone()];
            ui.close_menu();
        }
    }
    fn asset_list(&mut self, ui: &mut egui::Ui, items: &[assets::Asset]) {
        let widths = [
            26.0,
            (ui.available_width() - 493.0).max(150.0),
            94.0,
            112.0,
            112.0,
            72.0,
            28.0,
        ];
        let cell = |ui: &mut egui::Ui, width: f32, label: egui::Label, right: bool| {
            ui.allocate_ui_with_layout(
                Vec2::new(width, 36.0),
                if right {
                    egui::Layout::right_to_left(egui::Align::Center)
                } else {
                    egui::Layout::left_to_right(egui::Align::Center)
                },
                |ui| {
                    ui.set_min_size(Vec2::new(width, 36.0));
                    ui.add(label.truncate())
                },
            )
            .inner
        };
        egui::ScrollArea::horizontal()
            .id_salt("asset-list-horizontal")
            .show(ui, |ui| {
                egui::Grid::new("asset-table")
                    .striped(true)
                    .spacing(Vec2::new(8.0, 2.0))
                    .min_row_height(36.0)
                    .show(ui, |ui| {
                        for (i, label) in
                            ["", "Name", "Type", "Application", "Modified", "Size", ""]
                                .iter()
                                .enumerate()
                        {
                            cell(
                                ui,
                                widths[i],
                                egui::Label::new(
                                    RichText::new(tr(*label)).small().strong().color(muted()),
                                ),
                                i == 5,
                            );
                        }
                        ui.end_row();
                        for asset in items {
                            let mut selected = self.assets.selected.contains(&asset.file.path);
                            if ui.checkbox(&mut selected, "").changed() {
                                if selected {
                                    self.assets.selected.insert(asset.file.path.clone());
                                } else {
                                    self.assets.selected.remove(&asset.file.path);
                                }
                            }
                            let favorite = if self.prefs.favorite_assets.contains(&asset.file.path)
                            {
                                "★ "
                            } else {
                                ""
                            };
                            let response = cell(
                                ui,
                                widths[1],
                                egui::Label::new(format!("{favorite}{}", asset.file.title))
                                    .sense(egui::Sense::click()),
                                false,
                            )
                            .on_hover_text(asset.file.path.display().to_string());
                            if response.double_clicked() {
                                self.begin_asset_use(asset.clone());
                            }
                            response.context_menu(|ui| self.asset_menu(ui, asset));
                            cell(ui, widths[2], egui::Label::new(tr(asset.kind)), false);
                            cell(ui, widths[3], egui::Label::new(asset.file.app.name), false);
                            cell(
                                ui,
                                widths[4],
                                egui::Label::new(
                                    asset
                                        .file
                                        .modified
                                        .map(|d| d.format("%d %b %Y").to_string())
                                        .unwrap_or_else(|| "—".into()),
                                ),
                                false,
                            );
                            cell(
                                ui,
                                widths[5],
                                egui::Label::new(format_file_size(asset.file.size_bytes)),
                                true,
                            );
                            more_menu(ui, |ui| self.asset_menu(ui, asset));
                            ui.end_row();
                        }
                    });
            });
    }
    fn asset_grid(&mut self, ui: &mut egui::Ui, items: &[assets::Asset], waterfall: bool) {
        let width = ui.available_width().min(266.0);
        let gap = 16.0;
        let columns = (((ui.available_width() + gap) / (width + gap)).floor() as usize)
            .max(1)
            .min(items.len());
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.set_width(columns as f32 * width + (columns - 1) as f32 * gap);
            ui.columns(columns, |uis| {
                for (i, asset) in items.iter().enumerate() {
                    let ui = &mut uis[i % columns];
                    ui.push_id(&asset.file.path, |ui| {
                        let width = ui.available_width();
                        let height = if waterfall {
                            asset
                                .file
                                .preview
                                .as_ref()
                                .map(|p| width * p.height() as f32 / p.width().max(1) as f32)
                                .unwrap_or(width * 9.0 / 16.0)
                                .clamp(100.0, 300.0)
                        } else {
                            width * 9.0 / 16.0
                        };
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
                        if ui.is_rect_visible(rect) {
                            self.assets.request_preview(asset);
                        }
                        ui.painter().rect_filled(rect, UI_RADIUS, panel());
                        if let Some(image) = &asset.file.preview {
                            let texture = self
                                .assets
                                .textures
                                .entry(asset.file.path.clone())
                                .or_insert_with(|| {
                                    ui.ctx().load_texture(
                                        format!(
                                            "asset-{}-{}",
                                            asset.file.path.display(),
                                            asset.file.revision
                                        ),
                                        egui::ColorImage::from_rgba_unmultiplied(
                                            [image.width() as usize, image.height() as usize],
                                            image.as_raw(),
                                        ),
                                        egui::TextureOptions::LINEAR,
                                    )
                                });
                            let size = texture.size_vec2();
                            let scale = (rect.width() / size.x).min(rect.height() / size.y);
                            egui::Image::new((texture.id(), size * scale))
                                .corner_radius(UI_RADIUS)
                                .paint_at(
                                    ui,
                                    egui::Rect::from_center_size(rect.center(), size * scale),
                                );
                        } else {
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                assets::extension(&asset.file.path).to_uppercase(),
                                egui::FontId::proportional(18.0),
                                muted(),
                            );
                        }
                        let selected = self.assets.selected.contains(&asset.file.path);
                        ui.painter().rect_stroke(
                            rect,
                            UI_RADIUS,
                            egui::Stroke::new(
                                if selected { 2.0_f32 } else { 1.0_f32 },
                                if selected || response.hovered() {
                                    accent()
                                } else {
                                    border()
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        let check = egui::Rect::from_min_size(
                            rect.min + Vec2::splat(8.0),
                            Vec2::splat(22.0),
                        );
                        ui.painter().rect_filled(check, UI_RADIUS, chrome());
                        let mut checked = selected;
                        let check_response =
                            ui.put(check, egui::Checkbox::without_text(&mut checked));
                        if check_response.changed() {
                            if checked {
                                self.assets.selected.insert(asset.file.path.clone());
                            } else {
                                self.assets.selected.remove(&asset.file.path);
                            }
                        }
                        if response.double_clicked() {
                            self.begin_asset_use(asset.clone());
                        }
                        response.context_menu(|ui| self.asset_menu(ui, asset));
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [(width - 38.0).max(60.0), 28.0],
                                egui::Label::new(&asset.file.title).truncate(),
                            );
                            more_menu(ui, |ui| self.asset_menu(ui, asset));
                        });
                        ui.label(
                            RichText::new(format!(
                                "{} · {} · {}",
                                asset.file.app.name,
                                asset.kind,
                                format_file_size(asset.file.size_bytes)
                            ))
                            .small()
                            .color(muted()),
                        );
                        ui.add_space(20.0);
                    });
                }
            });
        });
    }
    fn asset_dialogs(&mut self, ui: &mut egui::Ui, root: &Path) {
        let dialog_frame = || {
            egui::Frame::new()
                .fill(panel())
                .stroke(egui::Stroke::new(1.0_f32, border()))
                .corner_radius(UI_RADIUS)
                .inner_margin(24)
        };
        if let Some(mut import) = self.assets.import.take() {
            let mut commit = false;
            let mut cancel = false;
            let modal=egui::Modal::new(egui::Id::new("import-assets")).frame(dialog_frame()).show(ui.ctx(),|ui| {
                ui.set_width(480.0_f32.min(ui.ctx().screen_rect().width()-60.0));ui.heading(tr("Import to asset library"));
                ui.label(tr(format!("{} selected file(s) or folder(s). Originals will be kept and matching names saved as separate copies.",import.paths.len())));
                egui::ComboBox::from_id_salt("import-asset-owner").selected_text(app_by_id(&import.app).map(|a|a.name).unwrap_or("PhotoCraft")).show_ui(ui,|ui|{for app in APPS{ui.selectable_value(&mut import.app,app.id.into(),app.name);}});
                let owner=app_by_id(&import.app).unwrap_or(&APPS[0]);let folder=root.join(owner.name).join("Assets");let folder=if import.presets{folder.join("Presets")}else{folder};
                ui.label(RichText::new(folder.display().to_string()).small().color(muted()));
                ui.add_space(12.0);ui.horizontal(|ui|{commit=ui.add(primary_button("Import")).clicked();cancel=ui.add(outline_button("Cancel")).clicked();});
            });
            if commit {
                let owner = *app_by_id(&import.app).unwrap_or(&APPS[0]);
                self.assets
                    .import_files(root.to_path_buf(), owner, import.paths, import.presets);
            } else if !cancel && !modal.should_close() {
                self.assets.import = Some(import);
            }
        }
        if let Some(mut request) = self.assets.use_asset.take() {
            self.assets.request_preview(&request.asset);
            let mut close = false;
            let mut launch = None;
            let modal = egui::Modal::new(egui::Id::new("use-asset"))
                .frame(dialog_frame())
                .show(ui.ctx(), |ui| {
                    ui.set_width(540.0_f32.min(ui.ctx().screen_rect().width() - 60.0));
                    ui.heading(tr("Use asset"));
                    ui.label(RichText::new(&request.asset.file.title).strong());
                    if let Some(image) = &request.asset.file.preview {
                        let texture = self
                            .assets
                            .textures
                            .entry(request.asset.file.path.clone())
                            .or_insert_with(|| {
                                ui.ctx().load_texture(
                                    format!("asset-dialog-{}", request.asset.file.path.display()),
                                    egui::ColorImage::from_rgba_unmultiplied(
                                        [image.width() as usize, image.height() as usize],
                                        image.as_raw(),
                                    ),
                                    egui::TextureOptions::LINEAR,
                                )
                            });
                        ui.add(
                            egui::Image::new((texture.id(), texture.size_vec2()))
                                .max_size(Vec2::new(ui.available_width(), 220.0))
                                .corner_radius(UI_RADIUS),
                        );
                    }
                    egui::ComboBox::from_id_salt("use-asset-app")
                        .selected_text(
                            app_by_id(&request.app)
                                .map(|a| a.name)
                                .unwrap_or("Choose app"),
                        )
                        .show_ui(ui, |ui| {
                            for app in APPS {
                                let installed = self
                                    .states
                                    .get(app.id)
                                    .is_some_and(|s| s.installed.is_some());
                                ui.selectable_value(
                                    &mut request.app,
                                    app.id.into(),
                                    format!(
                                        "{}{}",
                                        app.name,
                                        if installed { "" } else { " (not installed)" }
                                    ),
                                );
                            }
                        });
                    let app = *app_by_id(&request.app).unwrap_or(&APPS[0]);
                    let direct = assets::direct_open(&app, &request.asset.file.path);
                    ui.add_space(10.0);
                    ui.label(tr(assets::usage(&app, &request.asset.file.path)));
                    ui.add_space(12.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.add(outline_button("Copy file path")).clicked() {
                            ui.ctx()
                                .copy_text(request.asset.file.path.display().to_string());
                        }
                        if ui.add(outline_button(platform::reveal_label())).clicked() {
                            reveal_project_path(&request.asset.file.path, true);
                        }
                        let installed = self
                            .states
                            .get(app.id)
                            .is_some_and(|s| s.installed.is_some());
                        if ui
                            .add_enabled(
                                installed && request.asset.file.path.is_file(),
                                primary_button(if direct {
                                    "Open in app"
                                } else {
                                    "Open app and copy path"
                                }),
                            )
                            .clicked()
                        {
                            ui.ctx()
                                .copy_text(request.asset.file.path.display().to_string());
                            launch = Some((app, direct));
                        }
                        close = ui.add(outline_button("Done")).clicked();
                    });
                    if !self
                        .states
                        .get(app.id)
                        .is_some_and(|s| s.installed.is_some())
                    {
                        ui.label(tr("Install this app from All Apps to use this action."));
                    }
                });
            if let Some((app, direct)) = launch {
                self.launch(
                    app,
                    if direct {
                        Some(request.asset.file.path.as_path())
                    } else {
                        None
                    },
                );
            }
            if !close && !modal.should_close() {
                self.assets.use_asset = Some(request);
            }
        }
        if let Some(mut rename) = self.assets.rename.take() {
            let mut commit = false;
            let mut cancel = false;
            let modal=egui::Modal::new(egui::Id::new("rename-asset")).frame(dialog_frame()).show(ui.ctx(),|ui|{
                ui.set_width(400.0);ui.heading(tr("Rename asset"));ui.text_edit_singleline(&mut rename.name);ui.label(tr("Keep the file extension. Files already placed in a project may need relinking after a rename."));
                ui.horizontal(|ui|{commit=ui.add(primary_button("Rename")).clicked();cancel=ui.add(outline_button("Cancel")).clicked();});
            });
            if commit {
                self.assets.rename_file(rename.asset, rename.name);
            } else if !cancel && !modal.should_close() {
                self.assets.rename = Some(rename);
            }
        }
        if !self.assets.remove.is_empty() {
            let mut commit = false;
            let mut cancel = false;
            let modal=egui::Modal::new(egui::Id::new("remove-assets")).frame(dialog_frame()).show(ui.ctx(),|ui|{
                ui.set_width(430.0);ui.heading(tr(format!("Remove {} asset(s)?",self.assets.remove.len())));
                ui.label(tr("Moves the library copies to the library's .Trash folder. Original imports remain where they were. Projects referencing these library files may need relinking. You can restore files from Folders → Open library trash."));
                ui.horizontal(|ui|{commit=ui.add(primary_button("Move to library trash")).clicked();cancel=ui.add(outline_button("Cancel")).clicked();});
            });
            if commit {
                let assets = std::mem::take(&mut self.assets.remove);
                self.assets.remove_files(assets);
            } else if cancel || modal.should_close() {
                self.assets.remove.clear();
            }
        }
        if let Some(mut id) = self.assets.preset_help.take() {
            let mut close = false;
            let modal=egui::Modal::new(egui::Id::new("save-asset-presets")).frame(dialog_frame()).show(ui.ctx(),|ui|{
                ui.set_width(530.0_f32.min(ui.ctx().screen_rect().width()-60.0));ui.heading(tr("Save presets from an app"));
                egui::ComboBox::from_id_salt("preset-source-app").selected_text(app_by_id(&id).map(|a|a.name).unwrap_or("PhotoCraft")).show_ui(ui,|ui|{for app in APPS{ui.selectable_value(&mut id,app.id.into(),app.name);}});
                let app=*app_by_id(&id).unwrap_or(&APPS[0]);let folder=root.join(app.name).join("Assets/Presets");
                ui.label(tr(assets::preset_help(&app)));ui.add_space(12.0);ui.label(RichText::new(folder.display().to_string()).small().color(muted()));
                ui.label(tr("Save into this folder, then refresh Assets. Preset files remain in their original format."));
                ui.horizontal_wrapped(|ui|{
                    if ui.add(outline_button("Open save folder")).clicked(){match fs::create_dir_all(&folder){Ok(())=>reveal_project_path(&folder,false),Err(e)=>self.toast=Some(e.to_string())}}
                    if ui.add(outline_button("Copy folder path")).clicked(){ui.ctx().copy_text(folder.display().to_string());}
                    if ui.add_enabled(self.states.get(app.id).is_some_and(|s|s.installed.is_some()),primary_button("Open app")).clicked(){self.launch(app,None);}
                    close=ui.add(outline_button("Done")).clicked();
                });
            });
            if !close && !modal.should_close() {
                self.assets.preset_help = Some(id);
            }
        }
    }
}
