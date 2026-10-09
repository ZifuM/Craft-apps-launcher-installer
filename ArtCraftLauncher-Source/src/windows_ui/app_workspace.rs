use super::*;
use crate::app_screens::{action, ink_for};
use crate::windows_ui::toolbar;
#[path = "profile.rs"]
mod profile;

impl Launcher {
    fn workspace_identity(&mut self, ui: &mut egui::Ui, app: AppInfo, state: &AppState) {
        toolbar::style(ui);
        let profile = profile::for_app(app.id);
        let background = ui.painter().add(egui::Shape::Noop);
        let artwork = ui.painter().add(egui::Shape::Noop);
        let wide = ui.available_width() >= 900.0;
        let frame = egui::Frame::new()
            .corner_radius(14)
            .stroke(egui::Stroke::new(1.0_f32, border()))
            .inner_margin(24)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let full_width = ui.available_width();
                let copy_width = full_width - if wide { 260.0 } else { 0.0 };
                ui.scope(|ui| {
                    ui.set_width(copy_width);
                    let logo_size = if copy_width >= 600.0 { 112.0 } else { 80.0 };
                    let gap = 24.0;
                    let text_width = (copy_width - logo_size - gap).max(100.0);
                    let category = ui.painter().layout(
                        tr(app.category),
                        toolbar::font(11.0, true),
                        readable_app_color(app.tint),
                        text_width,
                    );
                    let title = ui.painter().layout(
                        app.name.into(),
                        toolbar::font(if copy_width >= 600.0 { 48.0 } else { 34.0 }, true),
                        foreground(),
                        text_width,
                    );
                    let copy_height = category.size().y + 12.0 + title.size().y;
                    let (identity, _) = ui.allocate_exact_size(
                        Vec2::new(copy_width, logo_size.max(copy_height)),
                        egui::Sense::hover(),
                    );
                    let logo = egui::Rect::from_center_size(
                        identity.left_center() + Vec2::new(logo_size * 0.5, 0.0),
                        Vec2::splat(logo_size),
                    );
                    if let Some(texture) = &state.icon {
                        egui::Image::new((texture.id(), texture.size_vec2()))
                            .corner_radius(20)
                            .paint_at(ui, logo);
                    } else {
                        let mut logo_ui = ui.new_child(egui::UiBuilder::new().max_rect(logo));
                        self.app_logo(&mut logo_ui, &app, logo_size);
                    }
                    let text_pos = egui::pos2(
                        identity.left() + logo_size + gap,
                        identity.center().y - copy_height * 0.5,
                    );
                    let title_pos = text_pos + Vec2::new(0.0, category.size().y + 12.0);
                    ui.painter()
                        .galley(text_pos, category, readable_app_color(app.tint));
                    ui.painter().galley(title_pos, title, foreground());
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().interact_size.y = 20.0;
                        let mut badges = Vec::new();
                        if state.installed.is_none() {
                            badges.push((tr("Not installed"), muted()));
                        }
                        if let Some(version) = &state.installed {
                            badges.push((
                                format!("{} v{}", tr("Installed version"), version),
                                muted(),
                            ));
                        }
                        if let Some(version) = &state.latest {
                            badges.push((
                                format!("{} v{}", tr("Latest version"), version),
                                readable_app_color(app.tint),
                            ));
                        }
                        badges.push((profile.platforms.into(), muted()));
                        for (label, color) in badges {
                            egui::Frame::new()
                                .fill(mix_color(panel(), app.tint, 0.06))
                                .corner_radius(6)
                                .inner_margin(egui::Margin::symmetric(9, 5))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(label)
                                            .font(toolbar::font(11.0, true))
                                            .color(color),
                                    );
                                });
                        }
                    });
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(tr(app.blurb)).font(toolbar::font(20.0, true)),
                        )
                        .wrap(),
                    );
                    ui.add_space(2.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(tr(profile.summary))
                                .font(toolbar::font(14.0, false))
                                .color(muted()),
                        )
                        .wrap(),
                    );
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    let label = if state.busy.is_some() {
                        "Working…"
                    } else if state.installed.is_some() {
                        "Open app"
                    } else if app.has_release {
                        "Install app"
                    } else {
                        "Coming soon"
                    };
                    let width = ui
                        .painter()
                        .layout_no_wrap(tr(label), toolbar::font(13.0, true), foreground())
                        .size()
                        .x
                        .max(74.0)
                        + 32.0;
                    let (button, _) =
                        ui.allocate_exact_size(Vec2::new(width, 40.0), egui::Sense::hover());
                    if action(
                        ui,
                        button,
                        "workspace-primary",
                        label,
                        app.tint,
                        ink_for(app.tint),
                        state.busy.is_none() && (state.installed.is_some() || app.has_release),
                    )
                    .clicked()
                    {
                        if state.installed.is_some() {
                            self.launch(app, None);
                        } else {
                            self.install(app);
                        }
                    }
                    if state.installed.is_some()
                        && state.latest.is_some()
                        && state.latest != state.installed
                    {
                        let update = format!(
                            "{} v{}",
                            tr("Update"),
                            state.latest.as_deref().unwrap_or_default()
                        );
                        if ui
                            .add_enabled(
                                state.busy.is_none(),
                                egui::Button::new(
                                    RichText::new(update).font(toolbar::font(13.0, true)),
                                )
                                .min_size(Vec2::new(120.0, 40.0)),
                            )
                            .clicked()
                        {
                            self.install(app);
                        }
                    }
                    crate::windows_ui::menus::more(ui, &tr("Details and actions"), |ui| {
                        self.app_action_menu(ui, app)
                    });
                    if profile.website.starts_with("https://getartcraft.com/")
                        && toolbar::standard(ui, "App website", false).clicked()
                    {
                        open_url(profile.website);
                    }
                    if toolbar::standard(ui, "Source repository", false).clicked() {
                        open_url(&format!("{REPO}/{}", release_slug(app.id)));
                    }
                    if toolbar::standard(ui, "Release notes", false).clicked() {
                        open_url(&format!("{REPO}/{}/releases", release_slug(app.id)));
                    }
                });
                if let Some(progress) = &state.busy {
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spinner();
                        ui.label(RichText::new(tr(progress)).color(muted()));
                    });
                }
                if let Some(error) = &state.error {
                    ui.add_space(10.0);
                    ui.label(RichText::new(tr(error)).color(theme_rgb(225, 112, 114)));
                }
            });
        let rect = frame.response.rect;
        ui.painter().set(
            background,
            egui::Shape::mesh(crate::windows_ui::identity_mesh(rect, app.tint, false)),
        );
        if wide {
            if let Some(texture) = &state.icon {
                let bounds = rect.shrink(1.0);
                let width = (bounds.width() * 0.36).clamp(300.0, 520.0);
                let art = egui::Rect::from_min_max(
                    egui::pos2(bounds.right() - width, bounds.top()),
                    bounds.max,
                );
                ui.painter()
                    .set(artwork, egui::Shape::mesh(workspace_art(art, texture)));
            }
        }
        ui.add_space(20.0);
    }

    pub(super) fn app_detail(&mut self, ui: &mut egui::Ui, id: &'static str) {
        if self.prefs.classic_app_screens {
            self.classic_app_detail(ui, id);
            return;
        }
        let Some(app) = app_by_id(id).copied() else {
            self.page = Page::Apps;
            return;
        };
        let state = self.states.get(id).cloned().unwrap_or_default();
        ui.push_id(id, |ui| {
            ui.horizontal(|ui| {
                if ui.link(format!("← {}", tr("App Manager"))).clicked() {
                    self.page = Page::Apps;
                }
            });
            ui.add_space(18.0);
            self.workspace_identity(ui, app, &state);
            let count = self.projects.iter().filter(|p| p.app.id == id).count();
            let mut tab = *self.workspace_tabs.get(id).unwrap_or(&0);
            let heading = match tab {
                1 => "Assets",
                2 => "Plugins",
                _ => "Project library",
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr(heading)).font(toolbar::font(24.0, true)));
                if tab == 0 {
                    let query = self
                        .workspace_search
                        .get(id)
                        .map(|s| s.trim().to_lowercase())
                        .unwrap_or_default();
                    let shown = self
                        .projects
                        .iter()
                        .filter(|p| {
                            p.app.id == id
                                && (p.title.to_lowercase().contains(&query)
                                    || p.path.to_string_lossy().to_lowercase().contains(&query))
                        })
                        .count();
                    ui.label(
                        RichText::new(if query.is_empty() {
                            count.to_string()
                        } else {
                            format!("{shown} / {count}")
                        })
                        .font(toolbar::font(12.0, false))
                        .color(muted()),
                    );
                }
            });
            ui.add_space(8.0);
            let available_width = ui.available_width();
            egui::ScrollArea::horizontal()
                .id_salt("workspace-library-toolbar")
                .show(ui, |ui| {
                    let tab_width = text_size(330.0);
                    let browse_width = ui
                        .painter()
                        .layout_no_wrap(
                            tr("Browse projects"),
                            toolbar::font(13.0, true),
                            foreground(),
                        )
                        .size()
                        .x
                        .max(116.0)
                        + 56.0;
                    let views_width = 116.0;
                    let gap = 8.0;
                    let minimum =
                        tab_width + browse_width + views_width + text_size(200.0) + gap * 3.0;
                    ui.set_width(available_width.max(minimum));
                    let row = toolbar::row(ui);
                    if let Some(index) = toolbar::tabs(
                        ui,
                        egui::Rect::from_min_size(row.min, Vec2::new(tab_width, toolbar::HEIGHT)),
                        "workspace-tabs",
                        &[
                            ("Projects", Some(count)),
                            ("Assets", None),
                            ("Plugins", None),
                        ],
                        tab as usize,
                    ) {
                        tab = index as u8;
                    }
                    if tab == 0 {
                        let search_width =
                            (row.width() - tab_width - browse_width - views_width - gap * 3.0)
                                .min(420.0);
                        toolbar::search(
                            ui,
                            egui::Rect::from_min_size(
                                row.min + Vec2::new(tab_width + gap, 0.0),
                                Vec2::new(search_width, toolbar::HEIGHT),
                            ),
                            "workspace-project-search",
                            self.workspace_search.entry(app.id.into()).or_default(),
                            "Search projects by name or format",
                        );
                        if toolbar::views(
                            ui,
                            egui::Rect::from_min_size(
                                row.min + Vec2::new(tab_width + search_width + gap * 2.0, 0.0),
                                Vec2::new(views_width, toolbar::HEIGHT),
                            ),
                            &mut self.prefs.workspace_project_view,
                        ) {
                            save_preferences(&self.prefs);
                        }
                    }
                    if toolbar::button(
                        ui,
                        egui::Rect::from_min_size(
                            row.right_top() - Vec2::new(browse_width, 0.0),
                            Vec2::new(browse_width, toolbar::HEIGHT),
                        ),
                        "browse-projects",
                        "Browse projects",
                        toolbar::Glyph::Arrow,
                        false,
                        true,
                    )
                    .clicked()
                    {
                        self.project_filter = app.name.into();
                        self.projects_tab = false;
                        self.project_scope = "All projects".into();
                        self.search.clear();
                        self.page = Page::Projects;
                    }
                });
            self.workspace_tabs.insert(id.into(), tab);
            ui.add_space(12.0);
            match tab {
                1 => self.workspace_placeholder(ui, &app, false),
                2 => self.plugins_page(ui, &app),
                _ => self.workspace_projects(ui, &app),
            }
        });
    }

    fn workspace_projects(&mut self, ui: &mut egui::Ui, app: &AppInfo) {
        let query = self
            .workspace_search
            .get(app.id)
            .map(|s| s.trim().to_lowercase())
            .unwrap_or_default();
        let projects: Vec<_> = self
            .projects
            .iter()
            .filter(|p| {
                p.app.id == app.id
                    && (p.title.to_lowercase().contains(&query)
                        || p.path.to_string_lossy().to_lowercase().contains(&query))
            })
            .cloned()
            .collect();
        if projects.is_empty() {
            egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32,border())).corner_radius(14).inner_margin(28).show(ui,|ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(tr(if query.is_empty(){"Your next project starts here"}else{"No matching projects"})).size(text_size(19.0)));
                ui.add_space(6.0);
                ui.label(RichText::new(tr(if query.is_empty(){"Connect a project folder to see previews and open your work from this workspace."}else{"Try another name or file format."})).color(muted()));
                ui.add_space(14.0);
                if !query.is_empty() && secondary_button(ui,"Clear search").clicked(){self.workspace_search.remove(app.id);}
                if query.is_empty() && secondary_button(ui,"Manage project folders").clicked(){self.projects_tab=true;self.page=Page::Projects;}
            });
        } else {
            self.workspace_project_gallery(ui, &projects);
        }
        ui.add_space(22.0);
        ui.collapsing(tr("Supported file formats"), |ui| {
            ui.label(
                RichText::new(tr(app
                    .filetypes
                    .iter()
                    .map(|e| format!(".{e}"))
                    .collect::<Vec<_>>()
                    .join("   ")))
                .color(muted()),
            );
        });
    }

    fn workspace_project_gallery(&mut self, ui: &mut egui::Ui, projects: &[Project]) {
        let view = self.prefs.workspace_project_view;
        if view == ProjectView::List {
            for project in projects {
                self.project_list_row(ui, project);
                ui.add_space(6.0);
            }
            return;
        }
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            let columns = ((ui.available_width() + 14.0) / text_size(280.0))
                .floor()
                .clamp(1.0, 3.0) as usize;
            if view == ProjectView::Grid {
                for row in projects.chunks(columns) {
                    ui.columns(columns, |cells| {
                        for (index, project) in row.iter().enumerate() {
                            self.workspace_project_card(&mut cells[index], project, false);
                        }
                    });
                    ui.add_space(6.0);
                }
            } else {
                let width = (ui.available_width() - 14.0 * (columns - 1) as f32) / columns as f32;
                let mut buckets: Vec<Vec<&Project>> = vec![Vec::new(); columns];
                let mut heights = vec![0.0_f32; columns];
                for project in projects {
                    let index = heights
                        .iter()
                        .enumerate()
                        .min_by(|a, b| a.1.total_cmp(b.1))
                        .map(|entry| entry.0)
                        .unwrap_or(0);
                    heights[index] += Self::workspace_project_height(project, width, true) + 14.0;
                    buckets[index].push(project);
                }
                ui.columns(columns, |cells| {
                    for (index, bucket) in buckets.iter().enumerate() {
                        for project in bucket {
                            self.workspace_project_card(&mut cells[index], project, true);
                            cells[index].add_space(6.0);
                        }
                    }
                });
            }
        });
    }

    fn workspace_project_height(project: &Project, width: f32, waterfall: bool) -> f32 {
        if waterfall {
            let aspect = project
                .preview
                .as_ref()
                .map(|p| p.width() as f32 / p.height().max(1) as f32)
                .unwrap_or(1.5);
            ((width - 36.0) / aspect).clamp(100.0, 220.0) + text_size(150.0)
        } else {
            text_size(166.0)
        }
    }

    fn workspace_project_card(&mut self, ui: &mut egui::Ui, project: &Project, waterfall: bool) {
        ui.push_id((&project.path, "workspace-project-card"), |ui| {
            let width = ui.available_width();
            let height = Self::workspace_project_height(project, width, waterfall);
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &project.title)
            });
            ui.painter().rect_filled(
                rect,
                12.0,
                if response.hovered() {
                    mix_color(panel(), project.app.tint, 0.035)
                } else {
                    panel()
                },
            );
            ui.painter().rect_stroke(
                rect,
                12.0,
                egui::Stroke::new(
                    1.0_f32,
                    if response.has_focus() {
                        project.app.tint
                    } else {
                        border()
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let preview_size = if waterfall {
                Vec2::new(width - 36.0, height - text_size(150.0))
            } else {
                Vec2::splat(48.0)
            };
            let thumb = egui::Rect::from_min_size(rect.min + Vec2::splat(18.0), preview_size);
            let mut thumb_ui = toolbar::cell(ui, thumb);
            self.project_thumbnail_at(&mut thumb_ui, project, preview_size);
            let controls_y = if waterfall {
                thumb.bottom() + 8.0
            } else {
                rect.top() + 24.0
            };
            let actions_rect = egui::Rect::from_min_size(
                egui::pos2(rect.right() - 48.0, controls_y),
                Vec2::splat(32.0),
            );
            let cloud_rect = egui::Rect::from_min_size(
                actions_rect.min + Vec2::new(-34.0, 2.0),
                Vec2::splat(28.0),
            );
            let mut actions_ui = toolbar::cell(ui, actions_rect);
            self.project_more_menu(&mut actions_ui, project);
            let mut cloud_ui = toolbar::cell(ui, cloud_rect);
            self.project_cloud_badge(&mut cloud_ui, project);
            let extension = project
                .path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("FILE")
                .to_uppercase();
            let favorite = self.prefs.favorite_projects.contains(&project.path);
            let eyebrow = if favorite {
                format!("★  {extension}")
            } else {
                extension.clone()
            };
            let eyebrow_rect = egui::Rect::from_min_max(
                egui::pos2(
                    if waterfall {
                        rect.left() + 18.0
                    } else {
                        thumb.right() + 12.0
                    },
                    controls_y,
                ),
                egui::pos2(cloud_rect.left() - 6.0, controls_y + 32.0),
            );
            let mut eyebrow_ui = toolbar::cell(ui, eyebrow_rect);
            eyebrow_ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(eyebrow)
                            .font(toolbar::font(10.0, true))
                            .color(readable_app_color(project.app.tint)),
                    )
                    .truncate(),
                );
            });
            let details_top = if waterfall {
                controls_y + 42.0
            } else {
                thumb.bottom() + 14.0
            };
            let details_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left() + 18.0, details_top),
                rect.right_bottom() - Vec2::splat(18.0),
            );
            let mut details = toolbar::cell(ui, details_rect);
            details.set_clip_rect(details_rect.intersect(ui.clip_rect()));
            details.spacing_mut().item_spacing.y = 6.0;
            let title = project
                .path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            details
                .add(
                    egui::Label::new(RichText::new(title).font(toolbar::font(16.0, true)))
                        .truncate(),
                )
                .on_hover_text(project.path.display().to_string());
            let modified = project
                .modified
                .map(|date| date.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| tr("Unknown date"));
            details
                .add(
                    egui::Label::new(
                        RichText::new(format!(
                            "{}  /  {}",
                            format_file_size(project.size_bytes),
                            modified
                        ))
                        .font(toolbar::font(12.0, false))
                        .color(muted()),
                    )
                    .truncate(),
                )
                .on_hover_text(format!("{}: {}", tr("Modified"), modified));
            let folder = project
                .path
                .parent()
                .unwrap_or(Path::new(""))
                .display()
                .to_string();
            details
                .add(
                    egui::Label::new(
                        RichText::new(&folder)
                            .font(toolbar::font(11.0, false))
                            .color(muted()),
                    )
                    .truncate(),
                )
                .on_hover_text(folder);
            self.project_context_menu(&response, project);
            let on_action = ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|point| actions_rect.contains(point) || cloud_rect.contains(point));
            if response.double_clicked() && !on_action {
                self.launch(*project.app, Some(&project.path));
            }
        });
    }

    fn workspace_placeholder(&mut self, ui: &mut egui::Ui, app: &AppInfo, plugins: bool) {
        app_screens::experimental_banner(ui);
        egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32,border())).corner_radius(12).inner_margin(32).show(ui,|ui|{
            ui.set_width(ui.available_width());
            let (r,_)=ui.allocate_exact_size(Vec2::splat(56.0),egui::Sense::hover());
            ui.painter().rect_filled(r,14.0,mix_color(panel(),app.tint,0.12));
            paint_navigation_icon(ui.painter(),r.shrink(16.0),Page::Projects,readable_app_color(app.tint));
            ui.add_space(16.0);
            ui.label(RichText::new(tr(if plugins{"Plugin management"}else{"Asset management"})).size(text_size(23.0)).strong());
            ui.add_space(8.0);
            ui.label(RichText::new(tr("Coming soon")).size(text_size(12.0)).color(readable_app_color(app.tint)));
            ui.add_space(12.0);
            ui.add(egui::Label::new(RichText::new(tr("Asset management is planned for a future release. Importing and organizing assets is not available yet.")).color(muted())).wrap());
            ui.add_space(24.0);
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui|{for label in ["Your asset library","Collections","Asset details"]{ui.label(RichText::new(tr(label)).size(text_size(13.0)).color(muted()));ui.add_space(20.0);}});
        });
    }
}

/// Cover-cropped app artwork with a transparent left edge and rounded outer corners.
fn workspace_art(rect: egui::Rect, texture: &egui::TextureHandle) -> egui::Mesh {
    let mut mesh = egui::Mesh::with_texture(texture.id());
    let image_size = texture.size_vec2();
    let scale = (rect.width() / image_size.x).max(rect.height() / image_size.y);
    let painted_size = image_size * scale;
    let image_origin = rect.center() - painted_size * 0.5;
    let radius = 13.0_f32;
    const STEPS: usize = 64;
    for y in 0..=STEPS {
        let vertical = y as f32 / STEPS as f32;
        let py = rect.top() + vertical * rect.height();
        let edge_distance = (py - rect.top()).min(rect.bottom() - py);
        let corner_offset = if edge_distance < radius {
            radius
                - (radius * radius - (radius - edge_distance).powi(2))
                    .max(0.0)
                    .sqrt()
        } else {
            0.0
        };
        for x in 0..=STEPS {
            let horizontal = x as f32 / STEPS as f32;
            let pos = egui::pos2(
                rect.left() + horizontal * (rect.width() - corner_offset),
                py,
            );
            let progress = horizontal;
            let fade = (progress * progress * (3.0 - 2.0 * progress)).powf(1.5);
            let opacity = fade * if light_theme() { 0.22 } else { 0.48 };
            mesh.vertices.push(egui::epaint::Vertex {
                pos,
                uv: egui::pos2(
                    (pos.x - image_origin.x) / painted_size.x,
                    (pos.y - image_origin.y) / painted_size.y,
                ),
                color: Color32::from_white_alpha((opacity * 255.0).round() as u8),
            });
        }
    }
    for y in 0..STEPS {
        for x in 0..STEPS {
            let a = (y * (STEPS + 1) + x) as u32;
            let b = a + (STEPS + 1) as u32;
            mesh.add_triangle(a, a + 1, b);
            mesh.add_triangle(a + 1, b + 1, b);
        }
    }
    mesh
}
