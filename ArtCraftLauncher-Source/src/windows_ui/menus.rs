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
    egui::Frame::new()
        .fill(card())
        .corner_radius(10)
        .inner_margin(12)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(tr(label))
                    .font(toolbar::font(11.0, true))
                    .color(muted()),
            );
            ui.add(
                egui::Label::new(RichText::new(value).font(toolbar::font(13.0, false)))
                    .wrap()
                    .selectable(true),
            );
        });
    ui.add_space(4.0);
}

impl Launcher {
    pub(crate) fn app_menu_shortcuts(&mut self, ui: &mut egui::Ui, app: AppInfo) {
        let directory = installed_dir(app.id).filter(|path| path.is_dir());
        let file = directory
            .as_ref()
            .and_then(|path| platform::executable(path, app.id));
        let shortcuts = [
            (platform::reveal_label(), file.is_some()),
            ("Copy file path", file.is_some()),
            ("Open install folder", directory.is_some()),
            ("Open workspace", true),
        ];
        let width = shortcuts
            .iter()
            .map(|(label, _)| {
                ui.painter()
                    .layout_no_wrap(tr(*label), toolbar::font(13.0, false), foreground())
                    .size()
                    .x
                    + 24.0
            })
            .fold(184.0_f32, f32::max);
        ui.set_width(width);
        for (index, (label, enabled)) in shortcuts.into_iter().enumerate() {
            if item(ui, label, enabled, false).clicked() {
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
                        self.detail_parent = Page::Apps;
                        self.page = Page::App(app.id);
                    }
                }
                ui.close_menu();
            }
        }
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
                let background = ui.painter().add(egui::Shape::Noop);
                let identity =
                    egui::Frame::new()
                        .corner_radius(14)
                        .inner_margin(18)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let name = properties
                                .project
                                .as_ref()
                                .map(|p| p.title.as_str())
                                .unwrap_or(app.name);
                            let copy_width = (ui.available_width() - 92.0).max(80.0);
                            let title = ui.painter().layout(
                                name.into(),
                                toolbar::font(26.0, true),
                                foreground(),
                                copy_width,
                            );
                            let eyebrow = ui.painter().layout(
                                tr("Properties").to_uppercase(),
                                toolbar::font(10.0, true),
                                readable_app_color(app.tint),
                                copy_width,
                            );
                            let category = ui.painter().layout(
                                tr(app.category),
                                toolbar::font(11.0, false),
                                muted(),
                                copy_width,
                            );
                            let height =
                                (eyebrow.size().y + 8.0 + title.size().y + 8.0 + category.size().y)
                                    .max(72.0);
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), height),
                                egui::Sense::hover(),
                            );
                            let logo = egui::Rect::from_center_size(
                                rect.left_center() + Vec2::new(36.0, 0.0),
                                Vec2::splat(72.0),
                            );
                            if let Some(texture) = &state.icon {
                                egui::Image::new((texture.id(), texture.size_vec2()))
                                    .corner_radius(14)
                                    .paint_at(ui, logo);
                            } else {
                                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(logo));
                                self.app_logo(&mut child, &app, 72.0);
                            }
                            let start = rect.min + Vec2::new(92.0, 0.0);
                            let title_pos = start + Vec2::new(0.0, eyebrow.size().y + 8.0);
                            let category_pos = title_pos + Vec2::new(0.0, title.size().y + 8.0);
                            ui.painter()
                                .galley(start, eyebrow, readable_app_color(app.tint));
                            ui.painter().galley(title_pos, title, foreground());
                            ui.painter().galley(category_pos, category, muted());
                        });
                ui.painter().set(
                    background,
                    egui::Shape::mesh(identity_mesh(identity.response.rect, app.tint, false)),
                );
                ui.add_space(16.0);
                egui::ScrollArea::vertical()
                    .id_salt("properties-body")
                    .max_height(
                        (ctx.screen_rect().height() - identity.response.rect.height() - 180.0)
                            .max(100.0),
                    )
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
