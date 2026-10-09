use super::*;
use crate::app_screens::{action, ink_for, text_at};
use crate::windows_ui::toolbar;

impl Launcher {
    pub(super) fn app_detail(&mut self, ui: &mut egui::Ui, id: &'static str) {
        if self.prefs.classic_app_screens {
            self.classic_app_detail(ui, id);
            return;
        }
        let Some(app) = app_by_id(id).copied() else {
            self.page = Page::YourApps;
            return;
        };
        let state = self.states.get(id).cloned().unwrap_or_default();
        ui.push_id(id, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .link(tr(if self.detail_parent == Page::YourApps {
                        "Your apps"
                    } else {
                        "App Manager"
                    }))
                    .clicked()
                {
                    self.page = self.detail_parent;
                }
                ui.label(RichText::new(tr("/  Workspace")).color(muted()));
            });
            ui.add_space(18.0);
            let (hero, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), text_size(210.0)),
                egui::Sense::hover(),
            );
            ui.painter()
                .rect_filled(hero, 16.0, mix_color(panel(), app.tint, 0.055));
            ui.painter().rect_stroke(
                hero,
                16.0,
                egui::Stroke::new(1.0_f32, mix_color(border(), app.tint, 0.38)),
                egui::StrokeKind::Inside,
            );
            let logo =
                egui::Rect::from_min_size(hero.min + Vec2::new(22.0, 26.0), Vec2::splat(64.0));
            let mut logo_ui = ui.new_child(egui::UiBuilder::new().max_rect(logo));
            self.app_logo(&mut logo_ui, &app, 64.0);
            let text_width = (hero.width() - 132.0).max(1.0);
            text_at(
                ui,
                egui::Rect::from_min_size(
                    hero.min + Vec2::new(108.0, text_size(24.0)),
                    Vec2::new(text_width, text_size(18.0)),
                ),
                app.category,
                10.0,
                readable_app_color(app.tint),
            );
            text_at(
                ui,
                egui::Rect::from_min_size(
                    hero.min + Vec2::new(108.0, text_size(46.0)),
                    Vec2::new(text_width, text_size(38.0)),
                ),
                app.name,
                29.0,
                foreground(),
            );
            text_at(
                ui,
                egui::Rect::from_min_size(
                    hero.min + Vec2::new(108.0, text_size(88.0)),
                    Vec2::new(text_width, text_size(22.0)),
                ),
                app.blurb,
                13.0,
                muted(),
            );
            let installed = state
                .installed
                .as_deref()
                .map(|v| format!("Installed  v{v}"))
                .unwrap_or_else(|| "Not installed".into());
            let latest = state
                .latest
                .as_deref()
                .map(|v| format!("Latest  v{v}"))
                .unwrap_or_else(|| "Release not checked".into());
            text_at(
                ui,
                egui::Rect::from_min_size(
                    hero.min + Vec2::new(22.0, text_size(120.0)),
                    Vec2::new(hero.width() - 44.0, text_size(20.0)),
                ),
                format!("{installed}   /   {latest}"),
                12.0,
                muted(),
            );
            let button = egui::Rect::from_min_size(
                hero.min + Vec2::new(22.0, text_size(154.0)),
                Vec2::new(132.0, 36.0),
            );
            let enabled = state.busy.is_none() && (state.installed.is_some() || app.has_release);
            let label = if state.busy.is_some() {
                "Working…"
            } else if state.installed.is_some() {
                "Open app"
            } else if app.has_release {
                "Install app"
            } else {
                "Coming soon"
            };
            if action(
                ui,
                button,
                "workspace-open",
                label,
                app.tint,
                ink_for(app.tint),
                enabled,
            )
            .clicked()
            {
                if state.installed.is_some() {
                    self.launch(app, None);
                } else {
                    self.install(app);
                }
            }
            let controls = egui::Rect::from_min_size(
                hero.min + Vec2::new(168.0, text_size(154.0)),
                Vec2::new(hero.width() - 190.0, 36.0),
            );
            let mut controls_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(controls)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            controls_ui.add_enabled_ui(state.busy.is_none(), |ui| {
                let update_available = state.installed.is_some()
                    && state.latest.is_some()
                    && state.latest != state.installed;
                if icon_button(
                    ui,
                    ButtonIcon::UpdateArrow,
                    if update_available {
                        "Install available update"
                    } else {
                        "Check for updates"
                    },
                    readable_app_color(app.tint),
                )
                .clicked()
                {
                    if update_available {
                        self.install(app);
                    } else {
                        self.check_app_release(app);
                    }
                }
                if state.installed.is_some()
                    && icon_button(
                        ui,
                        ButtonIcon::Delete,
                        "Uninstall app",
                        theme_rgb(213, 83, 93),
                    )
                    .clicked()
                {
                    self.show_remove = Some(id.into());
                }
            });
            if icon_button(
                &mut controls_ui,
                ButtonIcon::Github,
                "Source repository",
                muted(),
            )
            .clicked()
            {
                open_url(&format!("{REPO}/{}", release_slug(id)));
            }
            if icon_button(&mut controls_ui, ButtonIcon::Globe, "App website", muted()).clicked() {
                open_url("https://getartcraft.com/apps");
            }
            if let Some(progress) = &state.busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(progress);
                });
            }
            if let Some(error) = &state.error {
                ui.label(RichText::new(tr(error)).color(theme_rgb(255, 156, 135)));
            }
            ui.add_space(22.0);
            let count = self.projects.iter().filter(|p| p.app.id == id).count();
            let mut tab = *self.workspace_tabs.get(id).unwrap_or(&0);
            let tabs = toolbar::row(ui);
            let tab_width = (tabs.width() - 192.0).min(420.0);
            if let Some(index) = toolbar::tabs(
                ui,
                egui::Rect::from_min_size(tabs.min, Vec2::new(tab_width, toolbar::HEIGHT)),
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
            self.workspace_tabs.insert(id.into(), tab);
            if toolbar::button(
                ui,
                egui::Rect::from_min_size(
                    egui::pos2(tabs.right() - 172.0, tabs.top()),
                    Vec2::new(172.0, toolbar::HEIGHT),
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
            ui.add_space(8.0);
            if tab != 0 {
                toolbar::divider(ui);
            }
            match tab {
                1 => self.workspace_placeholder(ui, &app, false),
                2 => self.plugins_page(ui, &app),
                _ => self.workspace_projects(ui, &app, count),
            }
        });
    }

    fn workspace_projects(&mut self, ui: &mut egui::Ui, app: &AppInfo, count: usize) {
        let row = toolbar::row(ui);
        let heading_width = ui
            .painter()
            .layout_no_wrap(
                tr("Project library"),
                toolbar::font(15.0, true),
                foreground(),
            )
            .size()
            .x
            + 72.0;
        // Keep a compact search next to the view controls; narrow windows wrap this group.
        let controls = if row.width() >= heading_width + 360.0 {
            row
        } else {
            toolbar::row(ui)
        };
        let search_width =
            (controls.width() - if controls == row { heading_width } else { 0.0 } - 128.0)
                .min(360.0)
                .max(120.0);
        toolbar::search(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(controls.right() - 128.0 - search_width, controls.top()),
                Vec2::new(search_width, toolbar::HEIGHT),
            ),
            "workspace-project-search",
            self.workspace_search.entry(app.id.into()).or_default(),
            "Search projects by name or format",
        );
        if toolbar::views(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(controls.right() - 116.0, controls.top()),
                Vec2::new(116.0, toolbar::HEIGHT),
            ),
            &mut self.prefs.project_view,
        ) {
            save_preferences(&self.prefs);
        }
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
        let mut heading_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(
                    row.min,
                    Vec2::new(heading_width.min(row.width()), toolbar::HEIGHT),
                ))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        heading_ui.label(
            RichText::new(tr("Project library"))
                .font(toolbar::font(15.0, true))
                .color(foreground()),
        );
        heading_ui.label(
            RichText::new(if query.is_empty() {
                count.to_string()
            } else {
                format!("{} / {count}", projects.len())
            })
            .font(toolbar::font(12.0, false))
            .color(muted()),
        );
        ui.add_space(12.0);
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
            self.project_gallery(ui, projects);
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
