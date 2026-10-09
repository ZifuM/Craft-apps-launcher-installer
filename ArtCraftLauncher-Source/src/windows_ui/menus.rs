//! Shared Windows action menus and read-only properties.
use super::*;

pub(crate) fn style(ui: &mut egui::Ui) {
    toolbar::style(ui);
    ui.set_min_width(184.0);
    ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
    ui.spacing_mut().interact_size.y = 32.0;
    ui.spacing_mut().item_spacing.y = 3.0;
}

pub(crate) fn item(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    destructive: bool,
) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::SelectableLabel::new(
            false,
            RichText::new(tr(label))
                .font(toolbar::font(13.0, false))
                .color(if destructive {
                    theme_rgb(225, 112, 114)
                } else {
                    foreground()
                }),
        ),
    )
}

pub(crate) fn more(ui: &mut egui::Ui, label: &str, contents: impl FnOnce(&mut egui::Ui)) {
    let menu = egui::menu::menu_custom_button(
        ui,
        egui::Button::new("")
            .min_size(Vec2::splat(32.0))
            .corner_radius(6)
            .frame(false),
        |ui| {
            style(ui);
            contents(ui);
        },
    );
    for y in [-5.0, 0.0, 5.0] {
        ui.painter().circle_filled(
            menu.response.rect.center() + Vec2::new(0.0, y),
            1.5,
            if menu.inner.is_some() {
                foreground()
            } else {
                muted()
            },
        );
    }
    if menu.response.has_focus() {
        ui.painter().rect_stroke(
            menu.response.rect,
            6,
            egui::Stroke::new(1.0_f32, ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    menu.response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    menu.response.on_hover_text(label);
}

pub(crate) struct Properties {
    app: AppInfo,
    project: Option<Project>,
    directory: Option<PathBuf>,
    file: Option<PathBuf>,
    metadata: Vec<(String, String)>,
}
impl Properties {
    pub(crate) fn app(app: AppInfo) -> Self {
        let directory = installed_dir(app.id);
        let file = directory
            .as_ref()
            .and_then(|root| platform::executable(root, app.id));
        let metadata = file_metadata(file.as_deref());
        Self {
            app,
            project: None,
            directory,
            file,
            metadata,
        }
    }
    fn project(project: &Project) -> Self {
        Self {
            app: *project.app,
            project: Some(project.clone()),
            directory: project.path.parent().map(Path::to_path_buf),
            file: Some(project.path.clone()),
            metadata: file_metadata(Some(&project.path)),
        }
    }
}
fn file_metadata(path: Option<&Path>) -> Vec<(String, String)> {
    let Some(path) = path else {
        return vec![];
    };
    let mut rows = vec![(
        tr("File name"),
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    )];
    match std::fs::metadata(path) {
        Ok(meta) => {
            rows.push((tr("File size"), format_file_size(meta.len())));
            for (label, date) in [("Created", meta.created()), ("Modified", meta.modified())] {
                if let Ok(date) = date {
                    rows.push((
                        tr(label),
                        DateTime::<Local>::from(date)
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string(),
                    ));
                }
            }
            rows.push((
                tr("Read-only"),
                tr(if meta.permissions().readonly() {
                    "Yes"
                } else {
                    "No"
                }),
            ));
        }
        Err(_) => rows.push((tr("File status"), tr("File unavailable"))),
    }
    rows
}
fn detail(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(
        RichText::new(tr(label))
            .font(toolbar::font(11.0, false))
            .color(muted()),
    );
    ui.add(
        egui::Label::new(RichText::new(value).font(toolbar::font(13.0, false)))
            .wrap()
            .selectable(true),
    );
    ui.add_space(8.0);
}

impl Launcher {
    pub(crate) fn app_menu_shortcuts(&mut self, ui: &mut egui::Ui, app: AppInfo) {
        let directory = installed_dir(app.id).filter(|path| path.is_dir());
        let file = directory
            .as_ref()
            .and_then(|path| platform::executable(path, app.id));
        // Do not size shortcuts from the popup's default available width: doing
        // so permanently expands the popup before its text can determine its size.
        let menu_width = ["Check for updates", "Properties", "Uninstall…"]
            .iter()
            .map(|label| {
                ui.painter()
                    .layout_no_wrap(tr(*label), toolbar::font(13.0, false), foreground())
                    .size()
                    .x
                    + 24.0
            })
            .fold(184.0_f32, f32::max);
        ui.set_width(menu_width);
        let (row, _) = ui.allocate_exact_size(Vec2::new(menu_width, 36.0), egui::Sense::hover());
        let width = 36.0;
        let start = (menu_width - (4.0 * width + 3.0 * 6.0)) * 0.5;
        for (index, (label, enabled)) in [
            (platform::reveal_label(), file.is_some()),
            ("Copy file path", file.is_some()),
            ("Open install folder", directory.is_some()),
            ("Open workspace", true),
        ]
        .into_iter()
        .enumerate()
        {
            let rect = egui::Rect::from_min_size(
                row.min + Vec2::new(start + index as f32 * (width + 6.0), 0.0),
                Vec2::new(width, 36.0),
            );
            let mut button_ui = toolbar::cell(ui, rect);
            button_ui.spacing_mut().interact_size = Vec2::splat(36.0);
            button_ui.spacing_mut().button_padding = Vec2::splat(4.0);
            let response = button_ui.add_enabled(
                enabled,
                egui::Button::new("")
                    .min_size(rect.size())
                    .fill(card())
                    .corner_radius(6),
            );
            let color = if enabled {
                foreground()
            } else {
                muted().gamma_multiply(0.4)
            };
            paint_shortcut(ui.painter(), response.rect.center(), index, color);
            if response.has_focus() {
                ui.painter().rect_stroke(
                    response.rect,
                    6,
                    egui::Stroke::new(1.0_f32, ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tr(label))
            });
            if response
                .on_hover_text(tr(label))
                .on_disabled_hover_text(tr(label))
                .clicked()
            {
                match index {
                    0 => {
                        if let Some(path) = &file {
                            reveal_project_path(path, true);
                        }
                    }
                    1 => {
                        if let Some(path) = &file {
                            ui.ctx().copy_text(path.display().to_string());
                        }
                    }
                    2 => {
                        if let Some(path) = &directory {
                            reveal_project_path(path, false);
                        }
                    }
                    _ => {
                        self.detail_parent = if self.page == Page::YourApps {
                            Page::YourApps
                        } else {
                            Page::Apps
                        };
                        self.page = Page::App(app.id);
                    }
                }
                ui.close_menu();
            }
        }
        ui.add_space(3.0);
        ui.separator();
    }
    pub(crate) fn project_more_menu(&mut self, ui: &mut egui::Ui, project: &Project) {
        ui.push_id(("project-actions", &project.path), |ui| {
            more(ui, &tr("Project actions"), |ui| {
                self.project_menu_items(ui, project)
            });
        });
    }
    pub(crate) fn project_context_menu(&mut self, response: &egui::Response, project: &Project) {
        response.context_menu(|ui| {
            style(ui);
            self.project_menu_items(ui, project);
        });
    }
    fn project_menu_items(&mut self, ui: &mut egui::Ui, project: &Project) {
        if item(ui, &format!("Open in {}", project.app.name), true, false).clicked() {
            self.launch(*project.app, Some(&project.path));
            ui.close_menu();
        }
        if item(ui, "Rename…", true, false).clicked() {
            self.show_project_rename = Some(ProjectRename {
                focused: false,
                path: project.path.clone(),
                name: project
                    .path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            });
            ui.close_menu();
        }
        if item(ui, platform::reveal_label(), true, false).clicked() {
            reveal_project_path(&project.path, true);
            ui.close_menu();
        }
        if item(ui, "Copy file path", true, false).clicked() {
            ui.ctx().copy_text(project.path.display().to_string());
            ui.close_menu();
        }
        if item(
            ui,
            if self.prefs.favorite_projects.contains(&project.path) {
                "Remove from favorites"
            } else {
                "Add to favorites"
            },
            true,
            false,
        )
        .clicked()
        {
            self.toggle_project_favorite(&project.path);
            ui.close_menu();
        }
        if item(ui, "Cloud backup", true, false).clicked() {
            self.cloud.tab = 0;
            self.cloud.search = project.title.clone();
            self.page = Page::Cloud;
            ui.close_menu();
        }
        if item(ui, "Properties", true, false).clicked() {
            self.properties = Some(Properties::project(project));
            ui.close_menu();
        }
        ui.separator();
        if item(ui, "Delete…", true, true).clicked() {
            self.show_project_delete = Some(project.clone());
            ui.close_menu();
        }
    }
    pub(crate) fn properties_dialog(&mut self, ctx: &egui::Context) {
        let Some(properties) = self.properties.take() else {
            return;
        };
        let app = properties.app;
        let state = self.states.get(app.id).cloned().unwrap_or_default();
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("item-properties"))
            .backdrop_color(Color32::from_black_alpha(155))
            .frame(
                egui::Frame::new()
                    .fill(panel())
                    .stroke(egui::Stroke::new(1.0_f32, border()))
                    .corner_radius(14)
                    .inner_margin(24),
            )
            .show(ctx, |ui| {
                toolbar::style(ui);
                ui.set_width((ctx.screen_rect().width() - 80.0).clamp(280.0, 520.0));
                ui.horizontal(|ui| {
                    self.app_logo(ui, &app, 40.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tr("Properties")).font(toolbar::font(22.0, true)));
                        ui.add(
                            egui::Label::new(
                                RichText::new(
                                    properties
                                        .project
                                        .as_ref()
                                        .map(|p| p.title.as_str())
                                        .unwrap_or(app.name),
                                )
                                .color(muted()),
                            )
                            .truncate(),
                        );
                    });
                });
                ui.add_space(16.0);
                egui::ScrollArea::vertical()
                    .id_salt("properties-body")
                    .max_height((ctx.screen_rect().height() - 240.0).max(160.0))
                    .show(ui, |ui| {
                        if properties.project.is_none() {
                            detail(ui, "Description", &tr(app.blurb));
                            detail(
                                ui,
                                "Projects",
                                &self
                                    .projects
                                    .iter()
                                    .filter(|p| p.app.id == app.id)
                                    .count()
                                    .to_string(),
                            );
                            ui.columns(2, |cols| {
                                detail(
                                    &mut cols[0],
                                    "Installed version",
                                    &state
                                        .installed
                                        .clone()
                                        .unwrap_or_else(|| tr("Not installed")),
                                );
                                detail(
                                    &mut cols[1],
                                    "Latest version",
                                    &state.latest.clone().unwrap_or_else(|| tr("Not checked")),
                                );
                            });
                            if let Some(message) = state.error.as_ref().or(state.busy.as_ref()) {
                                detail(ui, "Status", &tr(message));
                            }
                        } else {
                            detail(ui, "Application", app.name);
                        }
                        if let Some(directory) = &properties.directory {
                            detail(
                                ui,
                                if properties.project.is_none() {
                                    "Install directory"
                                } else {
                                    "Folder"
                                },
                                &directory.display().to_string(),
                            );
                        }
                        if let Some(file) = &properties.file {
                            detail(ui, "File location", &file.display().to_string());
                        } else {
                            detail(ui, "File location", &tr("No installed executable found"));
                        }
                        for (key, value) in &properties.metadata {
                            detail(ui, key, value);
                        }
                        if properties.project.is_none() {
                            detail(
                                ui,
                                "Supported file formats",
                                &app.filetypes
                                    .iter()
                                    .map(|ext| format!(".{ext}"))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            );
                        }
                        if properties.project.is_some() {
                            ui.horizontal_wrapped(|ui| {
                                if let Some(file) = &properties.file {
                                    if toolbar::standard(ui, platform::reveal_label(), false)
                                        .clicked()
                                    {
                                        reveal_project_path(file, true);
                                    }
                                    if toolbar::standard(ui, "Copy file path", false).clicked() {
                                        ctx.copy_text(file.display().to_string());
                                    }
                                }
                            });
                        }
                        if properties.project.is_none() {
                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                if state.installed.is_some()
                                    && state.latest.is_some()
                                    && state.latest != state.installed
                                {
                                    if ui
                                        .add_enabled_ui(state.busy.is_none(), |ui| {
                                            toolbar::standard(ui, "Install update", true)
                                        })
                                        .inner
                                        .clicked()
                                    {
                                        self.install(app);
                                        close = true;
                                    }
                                }
                            });
                        }
                    });
                ui.add_space(16.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close |= toolbar::standard(ui, "Close", false).clicked();
                });
            });
        if !close && !modal.should_close() {
            self.properties = Some(properties);
        }
    }
}

/// Conventional folder-search, copy, open-folder and window-layout symbols.
fn paint_shortcut(painter: &egui::Painter, center: egui::Pos2, index: usize, color: Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
    let point = |x, y| center + Vec2::new(x, y);
    match index {
        0 => {
            // Folder and magnifying glass: locate the executable in File Explorer.
            painter.add(egui::Shape::line(
                vec![
                    point(-2.0, 6.0),
                    point(-8.0, 6.0),
                    point(-8.0, -6.0),
                    point(-3.0, -6.0),
                    point(-1.0, -3.0),
                    point(7.0, -3.0),
                    point(7.0, -1.0),
                ],
                stroke,
            ));
            let lens = point(3.0, 3.0);
            painter.circle_stroke(lens, 3.5, stroke);
            painter.line_segment([lens + Vec2::splat(2.5), lens + Vec2::splat(5.0)], stroke);
        }
        1 => {
            // Two overlapping sheets: copy the file path.
            painter.add(egui::Shape::line(
                vec![
                    point(-4.0, 3.0),
                    point(-7.0, 3.0),
                    point(-7.0, -7.0),
                    point(3.0, -7.0),
                    point(3.0, -4.0),
                ],
                stroke,
            ));
            painter.rect_stroke(
                egui::Rect::from_min_max(point(-3.0, -3.0), point(7.0, 7.0)),
                2,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        2 => {
            // Open folder with a slanted front flap.
            painter.add(egui::Shape::line(
                vec![
                    point(-8.0, 6.0),
                    point(-8.0, -6.0),
                    point(-3.0, -6.0),
                    point(-1.0, -3.0),
                    point(6.0, -3.0),
                    point(6.0, -1.0),
                ],
                stroke,
            ));
            painter.add(egui::Shape::closed_line(
                vec![
                    point(-8.0, 6.0),
                    point(-5.0, -1.0),
                    point(9.0, -1.0),
                    point(6.0, 6.0),
                ],
                stroke,
            ));
        }
        _ => {
            // Window with a title bar and sidebar: the app's workspace.
            painter.rect_stroke(
                egui::Rect::from_min_max(point(-8.0, -7.0), point(8.0, 7.0)),
                2,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment([point(-8.0, -2.0), point(8.0, -2.0)], stroke);
            painter.line_segment([point(-2.0, -2.0), point(-2.0, 7.0)], stroke);
        }
    }
}
