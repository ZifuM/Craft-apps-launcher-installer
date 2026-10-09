use super::*;
use crate::windows_ui::toolbar;
fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    toolbar::standard(ui, label, false)
}

pub(super) fn paint_cloud(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let c = r.center();
    let stroke = egui::Stroke::new(1.5_f32, color);
    let points = [
        (-7.0, 5.0),
        (-9.0, 2.0),
        (-8.0, -1.0),
        (-5.0, -2.0),
        (-4.0, -5.0),
        (0.0, -7.0),
        (4.0, -5.0),
        (5.0, -2.0),
        (8.0, -1.0),
        (9.0, 2.0),
        (7.0, 5.0),
        (-7.0, 5.0),
    ];
    p.add(egui::Shape::line(
        points.iter().map(|(x, y)| c + Vec2::new(*x, *y)).collect(),
        stroke,
    ));
}
// Logos are bundled assets, decoded once per egui context; no network requests at runtime.
fn paint_provider_icon(ui: &egui::Ui, rect: egui::Rect, provider: cloud::Provider) {
    let key = egui::Id::new(("cloud-provider-logo", provider));
    let cached = ui
        .ctx()
        .data(|data| data.get_temp::<egui::TextureHandle>(key));
    let texture = cached.or_else(|| {
        let color_image = if provider == cloud::Provider::Google {
            let image =
                image::load_from_memory(include_bytes!("../../assets/providers/google-drive.png"))
                    .ok()?
                    .to_rgba8();
            egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            )
        } else {
            let bytes: &[u8] = if provider == cloud::Provider::Dropbox {
                include_bytes!("../../assets/providers/dropbox.svg")
            } else {
                include_bytes!("../../assets/providers/onedrive.svg")
            };
            let tree =
                resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).ok()?;
            let mut image = resvg::tiny_skia::Pixmap::new(128, 128)?;
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(
                    128.0 / tree.size().width(),
                    128.0 / tree.size().height(),
                ),
                &mut image.as_mut(),
            );
            egui::ColorImage::from_rgba_premultiplied([128, 128], image.data())
        };
        let texture = ui.ctx().load_texture(
            format!("provider-{}", provider.name()),
            color_image,
            egui::TextureOptions::LINEAR,
        );
        ui.ctx()
            .data_mut(|data| data.insert_temp(key, texture.clone()));
        Some(texture)
    });
    if let Some(texture) = texture {
        ui.painter().image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

fn surface() -> egui::Frame {
    egui::Frame::new()
        .fill(panel())
        .stroke(egui::Stroke::new(1.0_f32, border()))
        .corner_radius(12)
        .inner_margin(16)
}
use crate::cloud::windows_backup::{State, Summary};
fn status_color(state: State) -> Color32 {
    match state {
        State::Confirmed => theme_rgb(88, 198, 151),
        State::Unavailable | State::Error => theme_rgb(235, 105, 112),
        State::Changed | State::Partial => theme_rgb(223, 173, 78),
        State::Copied | State::Uploading | State::Pending | State::Checking => {
            readable_app_color(ACCENT)
        }
        State::Local => muted(),
    }
}
pub(super) fn paint_backup(p: &egui::Painter, rect: egui::Rect, state: State) {
    let color = status_color(state);
    if state == State::Confirmed {
        p.rect_filled(rect, 5.0, mix_color(panel(), color, 0.17));
    }
    paint_cloud(p, rect, color);
    if state == State::Local {
        return;
    }
    let c = rect.center() + Vec2::new(6.0, 4.0);
    p.circle_filled(c, 5.5, panel());
    let stroke = egui::Stroke::new(1.5_f32, color);
    match state {
        State::Confirmed => {
            p.add(egui::Shape::line(
                vec![
                    c + Vec2::new(-3.0, 0.0),
                    c + Vec2::new(-1.0, 2.0),
                    c + Vec2::new(3.5, -2.5),
                ],
                stroke,
            ));
        }
        State::Copied => {
            p.circle_filled(c, 2.5, color);
        }
        State::Changed | State::Partial | State::Error | State::Unavailable => {
            p.line_segment([c - Vec2::new(0.0, 3.0), c + Vec2::new(0.0, 0.5)], stroke);
            p.circle_filled(c + Vec2::new(0.0, 3.0), 0.8, color);
        }
        _ => {
            p.circle_stroke(c, 4.0, stroke);
            p.add(egui::Shape::line(
                vec![c - Vec2::new(0.0, 2.5), c, c + Vec2::new(2.0, 1.0)],
                stroke,
            ));
        }
    }
}
fn backup_status_at(ui: &mut egui::Ui, rect: egui::Rect, summary: &Summary) {
    let icon =
        egui::Rect::from_center_size(rect.left_center() + Vec2::new(12.0, 0.0), Vec2::splat(24.0));
    paint_backup(ui.painter(), icon, summary.state);
    let text = egui::Rect::from_min_max(rect.min + Vec2::new(32.0, 0.0), rect.max);
    let height = rect.height() * 0.5;
    app_screens::text_at(
        ui,
        egui::Rect::from_min_size(text.min, Vec2::new(text.width(), height)),
        summary.state.label(),
        12.0,
        status_color(summary.state),
    );
    app_screens::text_at(
        ui,
        egui::Rect::from_min_size(
            text.min + Vec2::new(0.0, height),
            Vec2::new(text.width(), height),
        ),
        summary.date_label(),
        11.0,
        muted(),
    );
    ui.interact(
        rect,
        ui.id().with(("backup-info", rect.min.y.to_bits())),
        egui::Sense::hover(),
    )
    .on_hover_text(&summary.tooltip);
}
impl Launcher {
    pub(super) fn project_cloud_badge(&mut self, ui: &mut egui::Ui, project: &Project) {
        let summary = self.cloud.backup_status(project);
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::click());
        paint_backup(ui.painter(), rect, summary.state);
        if response.on_hover_text(summary.tooltip).clicked() {
            self.page = Page::Cloud;
        }
    }
    pub(super) fn cloud_page(&mut self, ui: &mut egui::Ui) {
        toolbar::style(ui);
        if toolbar::header(
            ui,
            "Cloud",
            "Keep a versioned backup of your work in your own sync folders.",
            Page::Cloud,
            "Sync folders",
            false,
        ) {
            self.cloud.tab = 3;
        }
        let nav = toolbar::row(ui);
        let order = [0, 1, 3, 2];
        let selected = order
            .iter()
            .position(|tab| *tab == self.cloud.tab)
            .unwrap_or(0);
        if let Some(index) = toolbar::tabs(
            ui,
            egui::Rect::from_min_size(nav.min, Vec2::new(nav.width().min(600.0), toolbar::HEIGHT)),
            "cloud-sections",
            &[
                ("Project files", None),
                ("Saved versions", None),
                ("Sync folders", Some(self.cloud.settings.folders.len())),
                ("Assets", None),
            ],
            selected,
        ) {
            let tab = order[index];
            if self.cloud.tab != tab {
                self.cloud.tab = tab;
                if tab == 1 && !self.cloud.busy && !self.cloud.settings.folders.is_empty() {
                    self.cloud.refresh();
                }
            }
        }
        toolbar::divider(ui);
        app_screens::experimental_banner(ui);
        if self.cloud.busy || !self.cloud.message.is_empty() {
            ui.scope(|ui| {
                ui.spacing_mut().interact_size.y = 24.0;
                ui.spacing_mut().button_padding = Vec2::new(8.0, 3.0);
                ui.horizontal_wrapped(|ui| {
                    if self.cloud.busy {
                        ui.spinner();
                    }
                    ui.label(
                        RichText::new(tr(&self.cloud.message))
                            .font(toolbar::font(12.0, false))
                            .color(muted()),
                    );
                    if self.cloud.busy {
                        if ui.small_button(tr("Cancel")).clicked() {
                            self.cloud.cancel();
                        }
                    } else if ui.small_button(tr("Dismiss")).clicked() {
                        self.cloud.message.clear();
                    }
                });
            });
            ui.add_space(8.0);
        }

        match self.cloud.tab {
            0 => {
                if ui.available_width() >= 880.0 {
                    let available = ui.available_width();
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(available - 296.0, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                surface().inner_margin(20).show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    self.cloud_projects(ui);
                                });
                            },
                        );
                        ui.add_space(10.0);
                        ui.allocate_ui_with_layout(
                            Vec2::new(276.0, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| self.cloud_backup_summary(ui),
                        );
                    });
                } else {
                    self.cloud_backup_summary(ui);
                    ui.add_space(18.0);
                    surface().inner_margin(20).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.cloud_projects(ui);
                    });
                }
            }
            1 => {
                surface().inner_margin(24).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    self.cloud_history(ui);
                });
            }
            3 => self.cloud_accounts_page(ui),
            _ => {
                surface().inner_margin(28).show(ui,|ui|{
                ui.set_width(ui.available_width());ui.label(RichText::new(tr("Asset backup")).size(text_size(22.0)));ui.add_space(10.0);
                ui.label(RichText::new(tr("Asset backup will arrive with asset management. Your project backups are available in Files.")).color(muted()));
                ui.add_space(20.0);if secondary_button(ui,"View files").clicked(){self.cloud.tab=0;}
            });
            }
        }
        self.cloud_dialogs(ui.ctx());
    }
    fn cloud_backup_summary(&mut self, ui: &mut egui::Ui) {
        surface().inner_margin(20).show(ui,|ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(tr("Backup overview")).font(toolbar::font(17.0,true)));
            ui.add_space(12.0);
            let saved=self.projects.iter().filter(|p|self.cloud.backup_status(p).state==State::Confirmed).count();
            let pending=self.projects.iter().filter(|p|self.cloud.settings.selected.contains(&p.path)&&!self.cloud.backup_status(p).state.has_copy()).count();
            let metrics=[("Selected files",self.cloud.settings.selected.len()),("Backed up to cloud",saved),("Awaiting backup",pending)];
            let wide=ui.available_width()>500.0;
            if wide {ui.columns(3,|cols|{for(i,(label,value))in metrics.iter().enumerate(){cols[i].label(RichText::new(value.to_string()).font(toolbar::font(24.0,true)));cols[i].label(RichText::new(tr(*label)).font(toolbar::font(12.0,false)).color(muted()));}});} else {
                for(label,value)in metrics {ui.horizontal(|ui|{ui.label(RichText::new(tr(label)).color(muted()));ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{ui.label(RichText::new(value.to_string()).font(toolbar::font(16.0,true)));});});}
            }
            ui.add_space(16.0);
            let before=self.cloud.settings.automatic;
            setting_toggle(ui,"Automatic backup","Back up selected files after saving.",&mut self.cloud.settings.automatic);
            if before!=self.cloud.settings.automatic{self.cloud.save();}
            ui.add_space(8.0);
            let can_sync=!self.cloud.busy&&!self.cloud.settings.selected.is_empty()&&self.cloud.settings.targets.iter().any(|p|self.cloud.settings.folders.contains_key(p));
            let actions=toolbar::row(ui);
            let first_width=if wide {(actions.width()-12.0)*0.5}else{actions.width()};
            if toolbar::button(ui,egui::Rect::from_min_size(actions.min,Vec2::new(first_width,toolbar::HEIGHT)),"backup-now",if self.cloud.busy{"Working…"}else{"Back up now"},toolbar::Glyph::Refresh,true,can_sync).clicked(){self.cloud.sync(&self.projects);}
            let second=if wide {egui::Rect::from_min_max(actions.min+Vec2::new(first_width+12.0,0.0),actions.max)}else{ui.add_space(8.0);toolbar::row(ui)};
            if toolbar::button(ui,second,"manage-sync-folders","Manage sync folders",toolbar::Glyph::Arrow,false,true).clicked(){self.cloud.tab=3;}
            ui.add_space(12.0);
            ui.label(RichText::new(tr("Backup dates show when the version was saved. Cloud confirmation comes from your sync app.")).font(toolbar::font(12.0,false)).color(muted()));
        });
    }
    fn cloud_accounts_page(&mut self, ui: &mut egui::Ui) {
        use crate::windows_ui::menus;
        ui.label(
            RichText::new(tr(
                "Sign in through each provider’s desktop app, then select its synced folder below.",
            ))
            .font(toolbar::font(12.0, false))
            .color(muted()),
        );
        ui.add_space(10.0);
        for provider in cloud::PROVIDERS {
            ui.push_id(provider, |ui| {
                surface().inner_margin(12).show(ui, |ui| {
                    let connected = self.cloud.settings.folders.contains_key(&provider);
                    let wide = ui.available_width() > 710.0 * crate::windows_ui::text_scale();
                    let (row,_) = ui.allocate_exact_size(Vec2::new(ui.available_width(),48.0),egui::Sense::hover());
                    let icon = egui::Rect::from_center_size(row.left_center()+Vec2::new(20.0,0.0),Vec2::splat(32.0));
                    paint_provider_icon(ui,icon,provider);
                    let control_width = 330.0 * crate::windows_ui::text_scale();
                    let text_width = (row.width()-54.0-if wide {control_width+12.0}else{0.0}).max(1.0);
                    app_screens::text_at(ui,egui::Rect::from_min_size(row.min+Vec2::new(54.0,1.0),Vec2::new(text_width,23.0)),provider.name(),15.0,foreground());
                    let folder = self.cloud.settings.folders.get(&provider).map(|f|f.path.display().to_string()).unwrap_or_else(||tr("Choose a folder managed by the desktop app"));
                    app_screens::text_at(ui,egui::Rect::from_min_size(row.min+Vec2::new(54.0,26.0),Vec2::new(text_width,22.0)),&folder,11.0,muted()).on_hover_text(folder);
                    let controls = if wide {
                        egui::Rect::from_min_size(egui::pos2(row.right()-control_width,row.center().y-20.0),Vec2::new(control_width,40.0))
                    } else { ui.add_space(4.0);toolbar::row(ui) };
                    let mut actions = ui.new_child(egui::UiBuilder::new().max_rect(controls).layout(egui::Layout::right_to_left(egui::Align::Center)));
                    actions.spacing_mut().button_padding=Vec2::new(10.0,6.0);
                    actions.spacing_mut().interact_size.y=32.0;
                    menus::more(&mut actions,&format!("{} · {}",provider.name(),tr("Sync folders")),|ui| {
                        if menus::item(ui,"Open folder",connected,false).clicked(){self.cloud.open_folder(provider);ui.close_menu();}
                        if menus::item(ui,"Open desktop app",true,false).clicked(){
                            self.cloud.message=match provider.open_desktop(){Ok(())=>format!("Opening {}. Check its system tray icon if no window appears.",provider.name()),Err(e)=>e};ui.close_menu();
                        }
                        if menus::item(ui,"Download desktop app",true,false).clicked(){open_url(provider.website());ui.close_menu();}
                        if connected {
                            ui.separator();
                            if menus::item(ui,"Disconnect",!self.cloud.busy,true).clicked(){self.cloud.disconnect_provider=Some(provider);ui.close_menu();}
                        }
                    });
                    let button = egui::Button::new(RichText::new(tr(if connected{"Change folder"}else{"Choose folder"})).font(toolbar::font(12.0,true)))
                        .min_size(Vec2::new(116.0,32.0)).fill(if connected{panel()}else{mix_color(panel(),ACCENT,0.18)}).corner_radius(6);
                    if actions.add_enabled(!self.cloud.busy,button).clicked(){self.cloud.connect(provider);}
                    if connected {
                        let mut enabled=self.cloud.settings.targets.contains(&provider);
                        if actions.add_enabled(!self.cloud.busy,egui::Checkbox::new(&mut enabled,tr("Back up"))).on_hover_text(tr("Use for project backups")).changed(){
                            self.cloud.settings.targets.retain(|p|*p!=provider);
                            if enabled{self.cloud.settings.targets.push(provider);} self.cloud.save();
                        }
                    }
                });
            });
            ui.add_space(6.0);
        }
        ui.add_space(6.0);
        ui.label(RichText::new(tr("Backup dates show when the version was saved. Cloud confirmation comes from your sync app.")).font(toolbar::font(11.0,false)).color(muted()));
    }
    fn cloud_projects(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(tr("Project files"))
                .size(text_size(18.0))
                .strong(),
        );
        ui.label(
            RichText::new(tr("Choose the projects you want to back up."))
                .size(text_size(12.0))
                .color(muted()),
        );
        ui.add_space(16.0);
        let search = toolbar::row(ui);
        let view_rect = egui::Rect::from_min_size(
            search.right_top() - Vec2::new(124.0, 0.0),
            Vec2::new(124.0, toolbar::HEIGHT),
        );
        toolbar::search(
            ui,
            egui::Rect::from_min_max(search.min, search.max - Vec2::new(136.0, 0.0)),
            "cloud-file-search",
            &mut self.cloud.search,
            "Search files or folders",
        );
        if toolbar::views(ui, view_rect, &mut self.prefs.cloud_project_view) {
            save_preferences(&self.prefs);
        }
        ui.add_space(8.0);
        let filters = toolbar::row(ui);
        if let Some(index) = toolbar::tabs(
            ui,
            egui::Rect::from_min_size(
                filters.min,
                Vec2::new(filters.width().min(460.0), toolbar::HEIGHT),
            ),
            "backup-filters",
            &[
                ("All files", None),
                ("Selected", None),
                ("Needs attention", None),
            ],
            self.cloud.view_filter as usize,
        ) {
            self.cloud.view_filter = index as u8;
        }
        toolbar::divider(ui);
        let query = self.cloud.search.trim().to_lowercase();
        let projects: Vec<_> = self
            .projects
            .iter()
            .filter(|p| {
                (p.title.to_lowercase().contains(&query)
                    || p.path.to_string_lossy().to_lowercase().contains(&query))
                    && match self.cloud.view_filter {
                        1 => self.cloud.settings.selected.contains(&p.path),
                        2 => matches!(
                            self.cloud.backup_status(p).state,
                            State::Error | State::Unavailable | State::Changed | State::Partial
                        ),
                        _ => true,
                    }
            })
            .cloned()
            .collect();
        if projects.is_empty() {
            surface().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add_space(18.0);
                ui.label(RichText::new(tr("No files here yet")).size(text_size(18.0)));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(tr(
                        "Add a project folder, or change your search and filters.",
                    ))
                    .color(muted()),
                );
                ui.add_space(16.0);
                if secondary_button(ui, "Manage project folders").clicked() {
                    self.projects_tab = true;
                    self.page = Page::Projects;
                }
                ui.add_space(10.0);
            });
            return;
        }
        let all = projects
            .iter()
            .all(|p| self.cloud.settings.selected.contains(&p.path));
        let (head, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), egui::Sense::hover());
        let mut select = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
            head.min + Vec2::new(15.0, 8.0),
            Vec2::splat(22.0),
        )));
        let mut select_all = all;
        select.spacing_mut().interact_size = Vec2::splat(20.0);
        if select
            .add_enabled(
                !self.cloud.busy,
                egui::Checkbox::without_text(&mut select_all),
            )
            .on_hover_text(tr("Select all visible files"))
            .changed()
        {
            for project in &projects {
                self.cloud.settings.selected.retain(|p| p != &project.path);
                if select_all {
                    self.cloud.settings.selected.push(project.path.clone());
                }
            }
            self.cloud.save();
        }
        app_screens::text_at(
            ui,
            egui::Rect::from_min_size(
                head.min + Vec2::new(54.0, 8.0),
                Vec2::new(head.width() - 64.0, 20.0),
            ),
            "Select all visible files",
            11.0,
            muted(),
        );
        if self.prefs.cloud_project_view != ProjectView::List {
            let waterfall = self.prefs.cloud_project_view == ProjectView::Waterfall;
            let count = ((ui.available_width() + 12.0) / 240.0).floor().max(1.0) as usize;
            let width = (ui.available_width() - (count - 1) as f32 * ui.spacing().item_spacing.x)
                / count as f32;
            let mut heights = vec![0.0_f32; count];
            let mut buckets = vec![Vec::new(); count];
            for (i, project) in projects.iter().enumerate() {
                let col = if waterfall {
                    heights
                        .iter()
                        .enumerate()
                        .min_by(|a, b| a.1.total_cmp(b.1))
                        .map(|v| v.0)
                        .unwrap_or(0)
                } else {
                    i % count
                };
                let aspect = project
                    .preview
                    .as_ref()
                    .map(|p| p.width() as f32 / p.height().max(1) as f32)
                    .unwrap_or(1.5);
                heights[col] += if waterfall {
                    (width / aspect).clamp(100.0, 260.0) + 122.0
                } else {
                    282.0
                };
                buckets[col].push(project);
            }
            ui.columns(count, |cols| {
                for (col, bucket) in buckets.iter().enumerate() {
                    for project in bucket {
                        self.cloud_project_card(&mut cols[col], project, waterfall);
                        cols[col].add_space(12.0);
                    }
                }
            });
            return;
        }
        for project in projects {
            let selected = self.cloud.settings.selected.contains(&project.path);
            let summary = self.cloud.backup_status(&project);
            let (r, response) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 70.0), egui::Sense::hover());
            let fill = if selected {
                mix_color(panel(), ACCENT, 0.05)
            } else if response.hovered() {
                card()
            } else {
                panel()
            };
            ui.painter().rect_filled(r, 8.0, fill);

            let mut check = ui.new_child(egui::UiBuilder::new().max_rect(
                egui::Rect::from_min_size(r.min + Vec2::new(15.0, 25.0), Vec2::splat(22.0)),
            ));
            let mut enabled = selected;
            check.spacing_mut().interact_size = Vec2::splat(20.0);
            if check
                .add_enabled(!self.cloud.busy, egui::Checkbox::without_text(&mut enabled))
                .on_hover_text(format!("{}: {}", tr("Back up"), project.title))
                .changed()
            {
                self.cloud.settings.selected.retain(|p| p != &project.path);
                if enabled {
                    self.cloud.settings.selected.push(project.path.clone());
                }
                self.cloud.save();
            }
            let thumb = egui::Rect::from_min_size(r.min + Vec2::new(50.0, 15.0), Vec2::splat(40.0));
            let mut image = ui.new_child(egui::UiBuilder::new().max_rect(thumb));
            self.project_thumbnail_at(&mut image, &project, thumb.size());
            let title = egui::Rect::from_min_max(
                r.min + Vec2::new(104.0, 15.0),
                egui::pos2(r.right() - 300.0, r.top() + 36.0),
            );
            app_screens::text_at(ui, title, &project.title, 14.0, foreground())
                .on_hover_text(project.path.display().to_string());
            app_screens::text_at(
                ui,
                title.translate(Vec2::new(0.0, 24.0)),
                format!(
                    "{}  ·  {}",
                    project.app.name,
                    format_file_size(project.size_bytes)
                ),
                11.0,
                muted(),
            );
            backup_status_at(
                ui,
                egui::Rect::from_min_size(
                    egui::pos2(r.right() - 280.0, r.center().y - 22.0),
                    Vec2::new(236.0, 44.0),
                ),
                &summary,
            );
            let menu_rect = egui::Rect::from_min_size(
                egui::pos2(r.right() - 36.0, r.center().y - 16.0),
                Vec2::splat(32.0),
            );
            let mut menu_ui = toolbar::cell(ui, menu_rect);
            self.project_more_menu(&mut menu_ui, &project);
            self.project_context_menu(&response, &project);
        }
    }
    fn cloud_project_card(&mut self, ui: &mut egui::Ui, project: &Project, waterfall: bool) {
        let selected = self.cloud.settings.selected.contains(&project.path);
        let summary = self.cloud.backup_status(project);
        let frame = egui::Frame::new()
            .fill(if selected {
                mix_color(card(), ACCENT, 0.08)
            } else {
                card()
            })
            .stroke(egui::Stroke::new(
                1.0_f32,
                if selected { ACCENT } else { border() },
            ))
            .corner_radius(10)
            .inner_margin(12)
            .show(ui, |ui| {
                let width = ui.available_width();
                let aspect = project
                    .preview
                    .as_ref()
                    .map(|p| p.width() as f32 / p.height().max(1) as f32)
                    .unwrap_or(1.5);
                let height = if waterfall {
                    (width / aspect).clamp(100.0, 260.0)
                } else {
                    150.0
                };
                self.project_thumbnail_at(ui, project, Vec2::new(width, height));
                ui.add_space(8.0);
                let (row, _) = ui.allocate_exact_size(Vec2::new(width, 32.0), egui::Sense::hover());
                let mut checkbox_ui =
                    toolbar::cell(ui, egui::Rect::from_min_size(row.min, Vec2::splat(28.0)));
                checkbox_ui.spacing_mut().interact_size = Vec2::splat(20.0);
                let mut checked = selected;
                if checkbox_ui
                    .add_enabled(!self.cloud.busy, egui::Checkbox::without_text(&mut checked))
                    .on_hover_text(format!("{}: {}", tr("Back up"), project.title))
                    .changed()
                {
                    self.cloud.settings.selected.retain(|p| p != &project.path);
                    if checked {
                        self.cloud.settings.selected.push(project.path.clone());
                    }
                    self.cloud.save();
                }
                app_screens::text_at(
                    ui,
                    egui::Rect::from_min_max(
                        row.min + Vec2::new(30.0, 4.0),
                        row.max - Vec2::new(36.0, 0.0),
                    ),
                    &project.title,
                    13.0,
                    foreground(),
                )
                .on_hover_text(project.path.display().to_string());
                let mut actions = toolbar::cell(
                    ui,
                    egui::Rect::from_min_size(
                        row.right_top() - Vec2::new(32.0, 0.0),
                        Vec2::splat(32.0),
                    ),
                );
                self.project_more_menu(&mut actions, project);
                ui.add(
                    egui::Label::new(
                        RichText::new(format!(
                            "{} · {}",
                            project.app.name,
                            format_file_size(project.size_bytes)
                        ))
                        .font(toolbar::font(11.0, false))
                        .color(muted()),
                    )
                    .truncate(),
                );
                ui.add_space(4.0);
                let (status_rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 44.0),
                    egui::Sense::hover(),
                );
                backup_status_at(ui, status_rect, &summary);
            });
        self.project_context_menu(&frame.response, project);
    }
    fn cloud_history(&mut self, ui: &mut egui::Ui) {
        let heading = toolbar::row(ui);
        let mut title_ui = toolbar::cell(
            ui,
            egui::Rect::from_min_max(heading.min, heading.max - Vec2::new(170.0, 0.0)),
        );
        title_ui.label(RichText::new(tr("Saved versions")).font(toolbar::font(18.0, true)));
        if toolbar::button(
            ui,
            egui::Rect::from_min_size(
                heading.right_top() - Vec2::new(148.0, 0.0),
                Vec2::new(148.0, toolbar::HEIGHT),
            ),
            "refresh-history",
            "Refresh",
            toolbar::Glyph::Refresh,
            false,
            !self.cloud.busy && !self.cloud.settings.folders.is_empty(),
        )
        .clicked()
        {
            self.cloud.refresh();
        }
        ui.label(RichText::new(tr("Restore an earlier version as a separate file. Your current project stays untouched.")).color(muted()));
        ui.add_space(16.0);
        if self.cloud.history.is_empty() {
            surface().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add_space(18.0);
                ui.label(
                    RichText::new(tr("Your backup history will appear here")).size(text_size(18.0)),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(tr(
                        "Choose a sync folder and back up a project to save its first version.",
                    ))
                    .color(muted()),
                );
                ui.add_space(16.0);
                if secondary_button(ui, "View files").clicked() {
                    self.cloud.tab = 0;
                }
                ui.add_space(10.0);
            });
        }
        let mut history = self.cloud.history.clone();
        history.sort_by(|a, b| b.modified.cmp(&a.modified));
        for remote in history {
            let (r, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 76.0), egui::Sense::hover());
            ui.painter().rect_filled(r, 0.0, panel());

            let button = egui::Rect::from_min_size(
                egui::pos2(r.right() - 140.0, r.center().y - 16.0),
                Vec2::new(124.0, 32.0),
            );
            let title = egui::Rect::from_min_max(
                r.min + Vec2::new(16.0, 15.0),
                egui::pos2(button.left() - 18.0, r.top() + 36.0),
            );
            app_screens::text_at(ui, title, remote.original_name(), 14.0, foreground())
                .on_hover_text(remote.original_name());
            let date = DateTime::parse_from_rfc3339(&remote.modified)
                .map(|t| t.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|_| remote.modified.clone());
            app_screens::text_at(
                ui,
                title.translate(Vec2::new(0.0, 25.0)),
                format!(
                    "{}  ·  {}  ·  {}",
                    remote.provider.name(),
                    date,
                    format_file_size(remote.bytes)
                ),
                11.0,
                muted(),
            );
            if app_screens::action(
                ui,
                button,
                ("restore", remote.provider, &remote.id),
                "Restore copy",
                card(),
                foreground(),
                !self.cloud.busy,
            )
            .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(remote.original_name())
                    .save_file()
                {
                    self.cloud.restore(remote.clone(), path);
                }
            }
        }
    }
    fn cloud_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(provider) = self.cloud.disconnect_provider {
            let mut remove = false;
            let mut cancel = false;
            let response=egui::Modal::new(egui::Id::new("cloud-disconnect")).frame(surface().inner_margin(26)).show(ctx,|ui|{
                ui.set_width(420.0);ui.heading(format!("{} {}?",tr("Disconnect"),provider.name()));ui.add_space(14.0);ui.label(tr("Master Suite will stop copying backups to this folder. Existing backup files and projects are kept. Your provider remains signed in."));ui.add_space(22.0);ui.horizontal(|ui|{cancel=secondary_button(ui,"Cancel").clicked();remove=danger_button(ui,"Disconnect").clicked();});
            });
            if remove {
                self.cloud.disconnect(provider);
                self.cloud.disconnect_provider = None;
            } else if cancel || response.should_close() {
                self.cloud.disconnect_provider = None;
            }
        }
    }
}
