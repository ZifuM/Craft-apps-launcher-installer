//! Shared desktop interface for Windows, Linux and macOS — Build 3.3.
use super::*;
pub(crate) mod menus;
pub(crate) mod toolbar;
static TEXT_SCALE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub(crate) fn set_text_scale(value: u8) {
    TEXT_SCALE.store(value.min(2), std::sync::atomic::Ordering::Relaxed);
}
pub(crate) fn text_scale() -> f32 {
    match TEXT_SCALE.load(std::sync::atomic::Ordering::Relaxed) {
        1 => 1.125,
        2 => 1.25,
        _ => 1.0,
    }
}

pub(super) fn page_heading(ui: &mut egui::Ui, eyebrow: &str, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(tr(eyebrow))
            .size(text_size(10.0))
            .strong()
            .color(muted()),
    );
    ui.add_space(5.0);
    ui.label(
        RichText::new(tr(title))
            .size(text_size(30.0))
            .strong()
            .color(foreground()),
    );
    ui.add_space(3.0);
    ui.label(
        RichText::new(tr(subtitle))
            .size(text_size(13.0))
            .color(muted()),
    );
    ui.add_space(22.0);
}
pub(super) fn refine_style(style: &mut egui::Style) {
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 9.0);
    style.spacing.interact_size = Vec2::new(36.0, 36.0);
    style.spacing.icon_width = 18.0;
    style.spacing.icon_spacing = 8.0;
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(text_size(14.0)),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(text_size(13.0)),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(text_size(12.0)),
    );
    style.visuals.hyperlink_color = readable_app_color(ACCENT);
    style.visuals.widgets.noninteractive.fg_stroke.color = foreground();
    style.visuals.widgets.inactive.fg_stroke.color = foreground();
    style.visuals.widgets.hovered.fg_stroke.color = foreground();
    style.visuals.widgets.active.fg_stroke.color = foreground();
    style.visuals.widgets.open.fg_stroke.color = foreground();
    style.visuals.widgets.inactive.weak_bg_fill = panel();
    style.visuals.widgets.hovered.weak_bg_fill = card();
    style.visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0_f32, mix_color(border(), ACCENT, 0.45));
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT);
    style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, foreground());
    style.visuals.window_corner_radius = 14.into();
    style.visuals.window_stroke = egui::Stroke::new(1.0_f32, border());
}

impl Launcher {
    fn sidebar_app(&mut self, ui: &mut egui::Ui, app: AppInfo) {
        let compact = self.sidebar_collapsed;
        let state = self.states.get(app.id).cloned().unwrap_or_default();
        let selected = self.page == Page::App(app.id);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), egui::Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, app.name)
        });
        if selected || response.hovered() {
            ui.painter().rect_filled(
                rect,
                6.0,
                mix_color(panel(), app.tint, if selected { 0.13 } else { 0.055 }),
            );
        }
        let icon = egui::Rect::from_center_size(
            if compact {
                rect.center()
            } else {
                rect.left_center() + Vec2::new(22.0, 0.0)
            },
            Vec2::splat(21.0),
        );
        if let Some(texture) = state.icon.as_ref() {
            ui.painter().image(
                texture.id(),
                icon,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            ui.painter().rect_filled(icon, 5.0, app.tint);
        }
        if !compact {
            let name = ui.painter().layout_no_wrap(
                app.name.into(),
                toolbar::font(12.0, selected),
                if selected { foreground() } else { muted() },
            );
            let clip = egui::Rect::from_min_max(
                rect.left_top() + Vec2::new(42.0, 0.0),
                rect.right_bottom() - Vec2::new(24.0, 0.0),
            );
            ui.painter().with_clip_rect(clip).galley(
                egui::pos2(clip.left(), rect.center().y - name.size().y * 0.5),
                name,
                if selected { foreground() } else { muted() },
            );
        }
        let update =
            state.installed.is_some() && state.latest.is_some() && state.latest != state.installed;
        if state.installed.is_some() || state.busy.is_some() {
            ui.painter().circle_filled(
                if compact {
                    icon.right_bottom() + Vec2::new(1.0, 1.0)
                } else {
                    rect.right_center() - Vec2::new(14.0, 0.0)
                },
                3.0,
                if update || state.busy.is_some() {
                    readable_app_color(app.tint)
                } else {
                    theme_rgb(102, 192, 154)
                },
            );
        }
        if selected {
            ui.painter().rect_filled(
                egui::Rect::from_center_size(
                    rect.left_center() + Vec2::new(2.0, 0.0),
                    Vec2::new(2.0, 16.0),
                ),
                1.0,
                app.tint,
            );
        }
        response.clone().on_hover_text(format!(
            "{}\n{}\n{}",
            app.name,
            tr(app.blurb),
            tr(if update {
                "Update available"
            } else if state.installed.is_some() {
                "Installed"
            } else {
                "Not installed"
            })
        ));
        response.context_menu(|ui| self.app_action_menu(ui, app));
        if response.clicked() {
            self.detail_parent = Page::Apps;
            self.page = Page::App(app.id);
        }
    }

    pub(super) fn modern_sidebar(&mut self, ctx: &egui::Context) {
        let compact = self.sidebar_collapsed;
        egui::SidePanel::left("modern-sidebar")
            .exact_width(if compact { 68.0 } else { 238.0 })
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(panel())
                    .inner_margin(egui::Margin::symmetric(10, 16)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                let (brand, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), if compact { 44.0 } else { 66.0 }),
                    egui::Sense::hover(),
                );
                let mark = egui::Rect::from_center_size(
                    if compact {
                        brand.center()
                    } else {
                        brand.left_center() + Vec2::new(27.0, 0.0)
                    },
                    Vec2::splat(30.0),
                );
                ui.painter().image(
                    self.brand_icon.id(),
                    mark,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                if !compact {
                    ui.painter().text(
                        brand.left_center() + Vec2::new(53.0, -10.0),
                        egui::Align2::LEFT_CENTER,
                        tr("ArtCraft"),
                        toolbar::font(19.0, true),
                        foreground(),
                    );
                    ui.painter().text(
                        brand.left_center() + Vec2::new(53.0, 12.0),
                        egui::Align2::LEFT_CENTER,
                        tr("MASTER SUITE"),
                        egui::FontId::proportional(text_size(10.0)),
                        muted(),
                    );
                }
                let (toggle, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 28.0),
                    egui::Sense::click(),
                );
                let response = response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(tr(if compact {
                        "Expand navigation"
                    } else {
                        "Collapse navigation"
                    }));
                if response.hovered() {
                    ui.painter().rect_filled(toggle, 7.0, card());
                }
                let icon = egui::Rect::from_center_size(
                    if compact {
                        toggle.center()
                    } else {
                        toggle.left_center() + Vec2::new(25.0, 0.0)
                    },
                    Vec2::new(15.0, 12.0),
                );
                ui.painter().rect_stroke(
                    icon,
                    2.0,
                    egui::Stroke::new(1.2_f32, muted()),
                    egui::StrokeKind::Inside,
                );
                ui.painter().line_segment(
                    [
                        icon.left_top() + Vec2::new(5.0, 0.0),
                        icon.left_bottom() + Vec2::new(5.0, 0.0),
                    ],
                    egui::Stroke::new(1.2_f32, muted()),
                );
                if !compact {
                    ui.painter().text(
                        toggle.left_center() + Vec2::new(44.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        tr("Collapse navigation"),
                        egui::FontId::proportional(text_size(11.0)),
                        muted(),
                    );
                }
                if response.clicked() {
                    self.sidebar_collapsed = !compact;
                    self.prefs.compact_sidebar = self.sidebar_collapsed;
                    save_preferences(&self.prefs);
                }
                ui.add_space(18.0);
                let installed = self
                    .states
                    .values()
                    .filter(|s| s.installed.is_some())
                    .count();
                let navigation_height =
                    (ui.available_height() - if compact { 66.0 } else { 132.0 }).max(60.0);
                egui::ScrollArea::vertical()
                    .id_salt("modern-navigation")
                    .max_height(navigation_height)
                    .show(ui, |ui| {
                        if !compact {
                            ui.label(
                                RichText::new(tr("  LIBRARY"))
                                    .size(text_size(10.0))
                                    .strong()
                                    .color(muted()),
                            );
                            ui.add_space(5.0);
                        }
                        self.modern_side_link(
                            ui,
                            Page::Home,
                            "Home",
                            "Your creative overview",
                            None,
                        );
                        if SHOW_YOUR_APPS {
                            self.modern_side_link(
                                ui,
                                Page::YourApps,
                                "Your apps",
                                "Open your collection",
                                Some(installed),
                            );
                        }
                        self.modern_side_link(
                            ui,
                            Page::Projects,
                            "Projects",
                            "Pick up where you left off",
                            Some(self.projects.len()),
                        );
                        ui.add_space(18.0);
                        if !compact {
                            ui.label(
                                RichText::new(tr("  MANAGE"))
                                    .size(text_size(10.0))
                                    .strong()
                                    .color(muted()),
                            );
                            ui.add_space(5.0);
                        }
                        self.modern_side_link(
                            ui,
                            Page::Apps,
                            "App Manager",
                            "Discover, install & update",
                            None,
                        );
                        self.modern_side_link(
                            ui,
                            Page::Cloud,
                            "Cloud",
                            "Your project backups",
                            None,
                        );
                        if !compact {
                            ui.add_space(22.0);
                            ui.label(
                                RichText::new(tr("Workspaces").to_uppercase())
                                    .font(toolbar::font(10.0, true))
                                    .color(muted()),
                            );
                            ui.add_space(6.0);
                        } else {
                            ui.add_space(12.0);
                        }
                        for app in APPS.iter().copied() {
                            self.sidebar_app(ui, app);
                        }
                    });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    self.modern_side_link(ui, Page::Settings, "Settings", "Make it yours", None);
                    ui.add_space(10.0);
                    if !compact {
                        let (status, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 48.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(status, 10.0, ink());
                        ui.painter().circle_filled(
                            status.left_center() + Vec2::new(17.0, 0.0),
                            3.0,
                            theme_rgb(100, 206, 163),
                        );
                        ui.painter().text(
                            status.left_center() + Vec2::new(30.0, -8.0),
                            egui::Align2::LEFT_CENTER,
                            tr("Your local workspace"),
                            egui::FontId::proportional(text_size(11.0)),
                            theme_rgb(210, 214, 223),
                        );
                        ui.painter().text(
                            status.left_center() + Vec2::new(30.0, 9.0),
                            egui::Align2::LEFT_CENTER,
                            tr(format!("{installed} apps installed")),
                            egui::FontId::proportional(text_size(10.0)),
                            muted(),
                        );
                    }
                });
            });
    }

    pub(super) fn modern_side_link(
        &mut self,
        ui: &mut egui::Ui,
        page: Page,
        label: &str,
        description: &str,
        count: Option<usize>,
    ) {
        let compact = self.sidebar_collapsed;
        let selected = self.page == page;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), egui::Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, tr(label))
        });
        response
            .clone()
            .on_hover_text(format!("{}\n{}", tr(label), tr(description)));
        let hover = if self.prefs.reduce_motion {
            if response.hovered() { 1.0 } else { 0.0 }
        } else {
            ui.ctx().animate_bool(response.id, response.hovered())
        };
        let base = if selected {
            mix_color(panel(), ACCENT, 0.13)
        } else {
            panel()
        };
        ui.painter()
            .rect_filled(rect, 7.0, mix_color(base, ACCENT, hover * 0.06));
        if selected {
            ui.painter().rect_stroke(
                rect,
                7.0,
                egui::Stroke::new(1.0_f32, mix_color(border(), ACCENT, 0.25)),
                egui::StrokeKind::Inside,
            );
            ui.painter().rect_filled(
                egui::Rect::from_center_size(
                    rect.left_center() + Vec2::new(2.0, 0.0),
                    Vec2::new(3.0, 19.0),
                ),
                2.0,
                ACCENT,
            );
        }
        let center = if compact {
            rect.center()
        } else {
            rect.left_center() + Vec2::new(25.0, 0.0)
        };
        let color = if selected {
            readable_app_color(ACCENT)
        } else {
            muted()
        };
        if page == Page::YourApps {
            let tile = egui::Rect::from_center_size(center, Vec2::splat(16.0));
            ui.painter().rect_stroke(
                tile,
                4.0,
                egui::Stroke::new(1.5_f32, color),
                egui::StrokeKind::Inside,
            );
            for y in [-3.0, 3.0] {
                ui.painter().line_segment(
                    [center + Vec2::new(-4.0, y), center + Vec2::new(4.0, y)],
                    egui::Stroke::new(1.5_f32, color),
                );
            }
        } else {
            paint_navigation_icon(
                ui.painter(),
                egui::Rect::from_center_size(
                    center
                        + Vec2::new(
                            0.0,
                            match page {
                                Page::Home => -1.0,
                                Page::Cloud => 1.0,
                                _ => 0.0,
                            },
                        ),
                    Vec2::splat(18.0),
                ),
                page,
                color,
            );
        }
        if !compact {
            // Every element uses the row's center; text height comes from its
            // actual font metrics instead of a fixed top inset.
            let center_y = rect.center().y;
            let mut text_right = rect.right() - 12.0;
            if let Some(count) = count {
                let count_text = if count > 99 {
                    "99+".into()
                } else {
                    count.to_string()
                };
                let count_galley = ui.painter().layout_no_wrap(
                    count_text,
                    egui::FontId::proportional(text_size(10.0)),
                    muted(),
                );
                let badge_size = Vec2::new(
                    (count_galley.size().x + 12.0).max(28.0),
                    (count_galley.size().y + 6.0).max(22.0),
                );
                let badge = egui::Rect::from_center_size(
                    egui::pos2(rect.right() - 14.0 - badge_size.x * 0.5, center_y),
                    badge_size,
                );
                ui.painter().rect_filled(
                    badge,
                    6.0,
                    if selected {
                        mix_color(panel(), ACCENT, 0.18)
                    } else {
                        ink()
                    },
                );
                ui.painter().galley(
                    badge.center() - count_galley.size() * 0.5,
                    count_galley,
                    muted(),
                );
                text_right = badge.left() - 12.0;
            }
            let label_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left() + 45.0, rect.top()),
                egui::pos2(text_right, rect.bottom()),
            );
            let mut job = egui::text::LayoutJob::simple_singleline(
                tr(label),
                egui::FontId::proportional(text_size(14.0)),
                foreground(),
            );
            job.wrap.max_width = label_rect.width().max(1.0);
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            let galley = ui.painter().layout_job(job);
            ui.painter()
                .with_clip_rect(label_rect.intersect(ui.clip_rect()))
                .galley(
                    egui::pos2(label_rect.left(), center_y - galley.size().y * 0.5),
                    galley,
                    foreground(),
                );
        }
        if response.clicked() {
            self.page = page;
        }
    }

    pub(super) fn heading(&self, ui: &mut egui::Ui, eyebrow: &str, title: &str, subtitle: &str) {
        page_heading(ui, eyebrow, title, subtitle);
    }

    pub(super) fn home(&mut self, ui: &mut egui::Ui) {
        let mut installed: Vec<_> = APPS
            .iter()
            .copied()
            .filter(|app| {
                self.states
                    .get(app.id)
                    .is_some_and(|state| state.installed.is_some())
            })
            .collect();
        installed.sort_by_key(|app| {
            self.projects
                .iter()
                .position(|project| project.app.id == app.id)
                .unwrap_or(usize::MAX)
        });
        let updates = self
            .states
            .values()
            .filter(|s| s.installed.is_some() && s.latest.is_some() && s.latest != s.installed)
            .count();
        let errors = self.states.values().filter(|s| s.error.is_some()).count();
        egui::ScrollArea::vertical().id_salt("home-dashboard").show(ui, |ui| {
            toolbar::style(ui);
            let hero=toolbar::page_banner(ui,"Home","From idea to your next project.",Page::Home);
            let banner=hero.rect;
            let show_art=hero.art.is_some();
            let buttons=hero.actions.min;
            let text_width=hero.actions.width();
            let button_width=["Manage apps", "Browse projects"].iter().map(|label| ui.painter().layout_no_wrap(tr(*label),toolbar::font(13.0,true),foreground()).size().x+56.0).fold(172.0_f32,f32::max).min((text_width-12.0)*0.5);
            if toolbar::button(ui,egui::Rect::from_min_size(buttons,Vec2::new(button_width,toolbar::HEIGHT)),"home-apps","Manage apps",toolbar::Glyph::Arrow,true,true).clicked() {self.page=Page::Apps;}
            if toolbar::button(ui,egui::Rect::from_min_size(buttons+Vec2::new(button_width+12.0,0.0),Vec2::new(button_width,toolbar::HEIGHT)),"home-projects","Browse projects",toolbar::Glyph::Arrow,false,true).clicked() {self.project_filter="All apps".into();self.projects_tab=false;self.project_scope="All projects".into();self.search.clear();self.page=Page::Projects;}
            if show_art {
                let center = hero.art.unwrap().center();
                let painter = ui.painter_at(banner.shrink(10.0));
                painter.circle_stroke(center, 66.0, egui::Stroke::new(1.0_f32, Color32::from_rgb(76, 61, 107)));
                painter.circle_filled(center, 52.0, Color32::from_rgb(51, 40, 80));
                painter.circle_filled(center, 39.0, Color32::from_rgb(67, 50, 110));
                let mark = egui::Rect::from_center_size(center, Vec2::splat(52.0));
                painter.image(self.brand_icon.id(), mark, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
                for (index, app) in self.orbit_apps.iter().enumerate() {
                    let angle = -1.3 + index as f32 * std::f32::consts::TAU / self.orbit_apps.len() as f32;
                    let position = center + Vec2::new(angle.cos() * 66.0, angle.sin() * 66.0);
                    let progress = index as f32 / self.orbit_apps.len().saturating_sub(1).max(1) as f32;
                    let size = 18.0 + 12.0 * progress;
                    let scale = size / 36.0;
                    let tile = egui::Rect::from_center_size(position, Vec2::splat(size));
                    painter.rect_filled(tile.expand(5.0 * scale), 11.0 * scale, mix_color(panel(), app.tint, 0.24));
                    painter.rect_stroke(tile.expand(5.0 * scale), 11.0 * scale, egui::Stroke::new(1.0_f32, mix_color(border(), app.tint, 0.45)), egui::StrokeKind::Inside);
                    if let Some(texture) = self.states.get(app.id).and_then(|state| state.icon.as_ref()) {
                        painter.image(texture.id(), tile, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
                    } else { painter.text(position, egui::Align2::CENTER_CENTER,tr(&app.name[..1]), egui::FontId::proportional(text_size(19.0 * scale)), app.tint); }
                }
            }
            ui.add_space(24.0);
            ui.spacing_mut().item_spacing.x = 14.0;
            ui.columns(3, |cols| {
                if toolbar::metric(&mut cols[0], "Your collection", &installed.len().to_string(), "Installed apps", Page::YourApps).clicked() { self.page = Page::Apps; }
                if toolbar::metric(&mut cols[1], "Project library", &self.projects.len().to_string(), &format!("Across {} folders", self.prefs.roots.len()), Page::Projects).clicked() { self.page = Page::Projects; }
                let caption = if updates > 0 { "View available updates" } else if errors > 0 { "Some checks need attention" } else { "Manage app releases" };
                if toolbar::metric(&mut cols[2], "App updates", &updates.to_string(), caption, Page::Apps).clicked() {
                    self.filter = if updates > 0 { "Available updates".into() } else { "All apps".into() };
                    if let Some(app) = APPS.iter().find(|a| self.states.get(a.id).is_some_and(|state| state.installed.is_some() && state.latest.is_some() && state.latest != state.installed)) { self.app_category = app.group; }
                    self.page = Page::Apps;
                }
            });
            ui.add_space(26.0);
            if toolbar::section(ui,"home-collection","Quick launch","Your installed tools, one click away.","Manage apps") {self.page=Page::Apps;}
            if installed.is_empty() {
                settings_section(ui, "Build your toolkit", "Explore creative and productivity apps, then install the tools you need.", |ui| {
                    if toolbar_primary_button(ui, "Explore apps").clicked() { self.page = Page::Apps; }
                });
            } else {
                let columns = ((ui.available_width() + 14.0) / 230.0).floor().clamp(2.0, 4.0) as usize;
                // Recent project activity brings the corresponding tools to the front.
                for apps in installed.iter().collect::<Vec<_>>().chunks(columns) {
                    ui.columns(columns, |cols| {
                        for (index, app) in apps.iter().enumerate() {
                            let ui = &mut cols[index];
                            let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 84.0), egui::Sense::click());
                            let state = self.states.get(app.id).cloned().unwrap_or_default();
                            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr(if state.busy.is_some() { "App operation in progress".to_owned() } else { format!("Open {}", app.name) }));
                            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, state.busy.is_none(), format!("Open {}", app.name)));
                            let hover = if self.prefs.reduce_motion { if response.hovered() { 1.0 } else { 0.0 } } else { ui.ctx().animate_bool(response.id, response.hovered()) };
                            identity_surface(ui.painter(),rect,app.tint,hover>0.5);
                            let logo = egui::Rect::from_center_size(egui::pos2(rect.left() + 36.0, rect.center().y), Vec2::splat(36.0));
                            if let Some(texture) = state.icon {
                                ui.painter().image(texture.id(), logo, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
                            } else {
                                ui.painter().rect_filled(logo, 8.0, app.tint);
                                ui.painter().text(logo.center(), egui::Align2::CENTER_CENTER,tr(&app.name[..1]), egui::FontId::proportional(text_size(20.0)), foreground());
                            }
                            ui.painter().text(egui::pos2(rect.left() + 66.0, rect.center().y - 10.0), egui::Align2::LEFT_CENTER,tr(app.name), toolbar::font(14.0,true), foreground());
                            ui.painter().text(egui::pos2(rect.left() + 66.0, rect.center().y + 12.0), egui::Align2::LEFT_CENTER,tr(if state.busy.is_some() { "Working..." } else { "Open app" }), toolbar::font(12.0,false), muted());
                            if response.clicked() && state.busy.is_none() { self.launch(**app, None); }
                        }
                    });
                    ui.add_space(8.0);
                }
            }
            ui.add_space(20.0);
            if toolbar::section(ui,"home-library","Recent projects","Continue where you left off.","View all projects") {self.project_filter="All apps".into();self.projects_tab=false;self.project_scope="All projects".into();self.search.clear();self.page=Page::Projects;}
            if self.projects.is_empty() {
                egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(14).inner_margin(24).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let (title, subtitle) = if self.prefs.roots.is_empty() {
                        ("Bring your projects together", "Choose a folder to create a home for each app and discover your saved work.")
                    } else if self.prefs.scanning {
                        ("Finding your projects", "Your selected folders are being scanned. Projects will appear here automatically.")
                    } else {
                        ("Your workspace is ready", "Save a project in one of your watched folders and it will appear here.")
                    };
                    ui.label(RichText::new(title).size(text_size(18.0)).strong());
                    ui.label(RichText::new(tr(subtitle)).size(text_size(13.0)).color(muted()));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if toolbar_primary_button(ui, "Add project folder").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() { self.add_project_folder(path); }
                        }
                        if !self.prefs.roots.is_empty() && toolbar_secondary_button(ui, "Refresh projects").clicked() { self.scan_projects(); }
                    });
                });
            } else {
                for project in self.projects.iter().take(4).cloned().collect::<Vec<_>>() {
                    self.project_list_row(ui, &project);
                    ui.add_space(4.0);
                }
            }
            ui.add_space(18.0);
        });
    }

    pub(super) fn projects_page(&mut self, ui: &mut egui::Ui) {
        if toolbar::header(
            ui,
            "Projects",
            "Your work, organized. Find a project and pick up where you left off.",
            Page::Projects,
            "Add folder",
            true,
        ) {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                self.add_project_folder(path);
            }
        }
        // One toolbar at every width; narrow windows can scroll horizontally.
        let toolbar_width = ui.available_width();
        egui::ScrollArea::horizontal()
            .id_salt("project-library-toolbar")
            .show(ui, |ui| {
                let gap = 8.0;
                let tabs_width = text_size(224.0);
                let app_width = text_size(126.0);
                let scope_width = text_size(120.0);
                let sort_width = text_size(156.0);
                let refresh_width = 40.0;
                let views_width = 116.0;
                let fixed_width = tabs_width
                    + app_width
                    + scope_width
                    + sort_width
                    + refresh_width
                    + views_width
                    + gap * 6.0;
                let width = toolbar_width.max(fixed_width + 220.0);
                ui.set_width(width);
                let row = toolbar::row(ui);
                let rect = |left: f32, width: f32| {
                    egui::Rect::from_min_size(
                        row.min + Vec2::new(left, 0.0),
                        Vec2::new(width, toolbar::HEIGHT),
                    )
                };
                if let Some(index) = toolbar::segments(
                    ui,
                    rect(0.0, tabs_width),
                    "project-sections",
                    &[
                        ("Library", self.projects.len()),
                        ("Folders", self.prefs.roots.len()),
                    ],
                    usize::from(self.projects_tab),
                ) {
                    self.projects_tab = index == 1;
                }
                if self.projects_tab {
                    return;
                }
                let mut x = tabs_width + gap;
                let mut apps = vec!["All apps"];
                apps.extend(APPS.iter().map(|app| app.name));
                toolbar::select(
                    ui,
                    rect(x, app_width),
                    "library-app",
                    &mut self.project_filter,
                    &apps,
                );
                x += app_width + gap;
                toolbar::select(
                    ui,
                    rect(x, scope_width),
                    "library-scope",
                    &mut self.project_scope,
                    &["All projects", "Favorites", "Last 7 days"],
                );
                x += scope_width + gap;
                let old_sort = self.prefs.project_sort.clone();
                toolbar::select(
                    ui,
                    rect(x, sort_width),
                    "library-sort",
                    &mut self.prefs.project_sort,
                    &["Recently modified", "Name A-Z", "Largest first", "By app"],
                );
                if old_sort != self.prefs.project_sort {
                    save_preferences(&self.prefs);
                }
                x += sort_width + gap;
                let mut refresh = toolbar::cell(ui, rect(x, refresh_width));
                if self.prefs.scanning {
                    refresh.disable();
                }
                if icon_button_sized(
                    &mut refresh,
                    ButtonIcon::Refresh,
                    "Refresh library",
                    muted(),
                    refresh_width,
                )
                .clicked()
                {
                    self.scan_projects();
                }
                x += refresh_width + gap;
                toolbar::search(
                    ui,
                    rect(x, row.width() - x - views_width - gap),
                    "project-search",
                    &mut self.search,
                    "Search projects, formats or folders",
                );
                if toolbar::views(
                    ui,
                    rect(row.width() - views_width, views_width),
                    &mut self.prefs.project_view,
                ) {
                    save_preferences(&self.prefs);
                }
            });
        toolbar::divider(ui);
        if self.projects_tab {
            self.project_folders_page(ui);
            return;
        }
        let query = self.search.trim().to_lowercase();
        let cutoff = Local::now() - chrono::Duration::days(7);
        let mut list: Vec<_> = self
            .projects
            .iter()
            .filter(|project| {
                (self.project_filter == "All apps" || self.project_filter == project.app.name)
                    && (query.is_empty()
                        || project
                            .path
                            .to_string_lossy()
                            .to_lowercase()
                            .contains(&query)
                        || project.app.name.to_lowercase().contains(&query))
                    && match self.project_scope.as_str() {
                        "Favorites" => self.prefs.favorite_projects.contains(&project.path),
                        "Last 7 days" => project.modified.is_some_and(|date| date >= cutoff),
                        _ => true,
                    }
            })
            .cloned()
            .collect();
        match self.prefs.project_sort.as_str() {
            "Name A-Z" => list.sort_by_key(|p| p.title.to_lowercase()),
            "Largest first" => list.sort_by_key(|p| std::cmp::Reverse(p.size_bytes)),
            "By app" => list.sort_by_key(|p| (p.app.name, p.title.to_lowercase())),
            _ => list.sort_by_key(|p| std::cmp::Reverse(p.modified)),
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr(format!(
                    "{} projects  /  {}",
                    list.len(),
                    format_file_size(list.iter().map(|p| p.size_bytes).sum())
                )))
                .size(text_size(12.0))
                .color(muted()),
            );
            if self.prefs.scanning {
                ui.label(
                    RichText::new(tr("Refreshing..."))
                        .size(text_size(12.0))
                        .color(ACCENT),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if (!query.is_empty()
                    || self.project_filter != "All apps"
                    || self.project_scope != "All projects")
                    && ui.link(tr("Clear filters")).clicked()
                {
                    self.search.clear();
                    self.project_filter = "All apps".into();
                    self.project_scope = "All projects".into();
                }
            });
        });
        ui.add_space(8.0);
        if self.prefs.roots.is_empty() {
            self.empty_projects(ui);
        } else if list.is_empty() {
            settings_section(
                ui,
                if self.prefs.scanning {
                    "Finding your projects"
                } else {
                    "No projects to show"
                },
                "Try another filter, refresh the library, or add a folder containing your work.",
                |ui| {
                    if secondary_button(ui, "Manage folders").clicked() {
                        self.projects_tab = true;
                    }
                },
            );
        } else {
            self.project_rows(ui, list);
        }
    }

    pub(super) fn project_folders_page(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(tr("Connected folders"))
                .size(text_size(19.0))
                .strong(),
        );
        ui.label(
            RichText::new(tr(
                "Files stay where they are. Removing a folder here only stops watching it.",
            ))
            .size(text_size(12.0))
            .color(muted()),
        );
        ui.add_space(18.0);
        let roots = self.prefs.roots.clone();
        if roots.is_empty() {
            self.empty_projects(ui);
            return;
        }
        let mut remove = None;
        egui::ScrollArea::vertical()
            .id_salt("library-folders")
            .show(ui, |ui| {
                for (index, root) in roots.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        egui::Frame::new()
                            .fill(panel())
                            .stroke(egui::Stroke::new(1.0_f32, border()))
                            .corner_radius(12)
                            .inner_margin(20)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let count = self
                                    .projects
                                    .iter()
                                    .filter(|p| p.path.starts_with(root))
                                    .count();
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(
                                                root.file_name()
                                                    .unwrap_or_default()
                                                    .to_string_lossy(),
                                            )
                                            .size(text_size(17.0))
                                            .strong(),
                                        )
                                        .truncate(),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new(tr(format!("{count} projects")))
                                                    .size(text_size(12.0))
                                                    .color(muted()),
                                            );
                                        },
                                    );
                                });
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(root.display().to_string())
                                            .size(text_size(12.0))
                                            .color(muted()),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(root.display().to_string());
                                ui.add_space(14.0);
                                ui.horizontal_wrapped(|ui| {
                                    if secondary_button(ui, "Open folder").clicked() {
                                        reveal_project_path(root, false);
                                    }
                                    if self.prefs.default_project_root.as_ref() == Some(root) {
                                        ui.label(
                                            RichText::new(tr("Default folder"))
                                                .size(text_size(12.0))
                                                .color(readable_app_color(ACCENT)),
                                        );
                                    } else if secondary_button(ui, "Make default").clicked() {
                                        self.prefs.default_project_root = Some(root.clone());
                                        save_preferences(&self.prefs);
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if icon_button(
                                                ui,
                                                ButtonIcon::Delete,
                                                "Stop watching this folder",
                                                muted(),
                                            )
                                            .clicked()
                                            {
                                                remove = Some(index);
                                            }
                                        },
                                    );
                                });
                            });
                    });
                    ui.add_space(12.0);
                }
            });
        if let Some(index) = remove {
            self.remove_project_folder(index);
        }
    }

    pub(super) fn project_list_row(&mut self, ui: &mut egui::Ui, project: &Project) {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 80.0), egui::Sense::click());
        self.project_context_menu(&response, project);
        ui.painter().rect_filled(
            rect,
            10.0,
            if response.hovered() {
                mix_color(panel(), ACCENT, 0.04)
            } else {
                panel()
            },
        );
        ui.painter().rect_stroke(
            rect,
            9.0,
            egui::Stroke::new(1.0_f32, border()),
            egui::StrokeKind::Inside,
        );
        let thumb_rect =
            egui::Rect::from_min_size(rect.min + Vec2::new(16.0, 16.0), Vec2::new(56.0, 48.0));
        let mut thumb_ui = ui.new_child(egui::UiBuilder::new().max_rect(thumb_rect));
        self.project_thumbnail_at(&mut thumb_ui, project, thumb_rect.size());
        let actions_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() - 48.0, rect.center().y - 16.0),
            Vec2::splat(32.0),
        );
        let details_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 80.0, rect.center().y - text_size(20.0)),
            egui::pos2(
                actions_rect.left() - 12.0,
                rect.center().y + text_size(20.0),
            ),
        );
        let mut details = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(details_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        details.set_clip_rect(details_rect.intersect(ui.clip_rect()));
        details.spacing_mut().item_spacing.y = 5.0;
        details
            .add(
                egui::Label::new(
                    RichText::new(
                        project
                            .path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy(),
                    )
                    .size(text_size(14.0))
                    .strong(),
                )
                .truncate(),
            )
            .on_hover_text(project.path.display().to_string());
        let extension = project
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("file")
            .to_uppercase();
        let modified = project
            .modified
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Unknown date".into());
        details
            .add(
                egui::Label::new(
                    RichText::new(tr(format!(
                        "{}  /  {}  /  {}  /  {}",
                        extension,
                        format_file_size(project.size_bytes),
                        modified,
                        project.app.name
                    )))
                    .size(text_size(11.0))
                    .color(muted()),
                )
                .truncate(),
            )
            .on_hover_text(format!(
                "{}: {}\n{}",
                tr("Modified"),
                modified,
                project.path.display()
            ));
        let summary = self.cloud.backup_status(project);
        let badge = egui::Rect::from_min_size(rect.min + Vec2::new(55.0, 48.0), Vec2::splat(22.0));
        ui.painter().rect_filled(badge, 6.0, panel());
        cloud_ui::paint_backup(ui.painter(), badge, summary.state);
        ui.interact(
            badge,
            ui.id().with(("project-backup-status", &project.path)),
            egui::Sense::hover(),
        )
        .on_hover_text(summary.tooltip);
        let mut actions = toolbar::cell(ui, actions_rect);
        self.project_more_menu(&mut actions, project);
        if response.double_clicked() {
            self.launch(*project.app, Some(&project.path));
        }
    }

    pub(super) fn project_tile(&mut self, ui: &mut egui::Ui, project: &Project, waterfall: bool) {
        let tile = egui::Frame::new()
            .fill(panel())
            .stroke(egui::Stroke::new(1.0_f32, border()))
            .corner_radius(12)
            .inner_margin(16)
            .show(ui, |ui| {
                let available = ui.available_width();
                let aspect = project
                    .preview
                    .as_ref()
                    .map(|p| p.width() as f32 / p.height().max(1) as f32)
                    .unwrap_or(1.5);
                let height = if waterfall {
                    (available / aspect).clamp(120.0, 240.0)
                } else {
                    (available / aspect).clamp(148.0, 180.0)
                };
                self.project_thumbnail_at(ui, project, Vec2::new(available, height));
                ui.add_space(7.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(project.app.name))
                            .size(text_size(10.0))
                            .strong()
                            .color(muted()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.project_more_menu(ui, project);
                        self.project_cloud_badge(ui, project);
                    });
                });
                let title = project
                    .path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();
                ui.add(
                    egui::Label::new(
                        RichText::new(title)
                            .size(text_size(15.0))
                            .strong()
                            .color(foreground()),
                    )
                    .truncate(),
                )
                .on_hover_text(project.path.display().to_string());
                let ext = project
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("FILE")
                    .to_uppercase();
                let modified = project
                    .modified
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "Unknown date".into());
                ui.label(
                    RichText::new(tr(format!(
                        "{ext}   /   {}   /   {modified}",
                        format_file_size(project.size_bytes)
                    )))
                    .size(text_size(10.0))
                    .color(muted()),
                );
                let folder = project
                    .path
                    .parent()
                    .unwrap_or(Path::new(""))
                    .display()
                    .to_string();
                ui.add(
                    egui::Label::new(
                        RichText::new(tr(folder))
                            .size(text_size(9.0))
                            .color(muted()),
                    )
                    .truncate(),
                )
                .on_hover_text(project.path.display().to_string());
            });
        self.project_context_menu(&tile.response, project);
    }

    pub(super) fn settings_page(&mut self, ui: &mut egui::Ui) {
        toolbar::style(ui);
        if toolbar::header(
            ui,
            "Settings",
            "Personalize your workspace and keep everything running smoothly.",
            Page::Settings,
            "Back to Home",
            false,
        ) {
            self.page = Page::Home;
        }
        let navigation = toolbar::row(ui);
        let width = navigation.width().min(650.0);
        if let Some(index) = toolbar::tabs(
            ui,
            egui::Rect::from_min_size(navigation.min, Vec2::new(width, toolbar::HEIGHT)),
            "settings-sections",
            &[
                ("Appearance", None),
                ("Updates", None),
                ("Projects", None),
                ("System", None),
                ("About", None),
            ],
            self.settings_tab,
        ) {
            self.settings_tab = index;
        }
        toolbar::divider(ui);
        let previous_startup = self.prefs.start_with_windows;
        let before = serde_json::to_string(&self.prefs).unwrap_or_default();
        if self.settings_tab == 0 {
            settings_section(
                ui,
                "Appearance & navigation",
                "Personalize your workspace without changing your projects.",
                |ui| {
                    let language_row = toolbar::row(ui);
                    let language_width = 280.0_f32.min(language_row.width() * 0.55);
                    let mut label_ui = toolbar::cell(
                        ui,
                        egui::Rect::from_min_max(
                            language_row.min,
                            language_row.max - Vec2::new(language_width + 16.0, 0.0),
                        ),
                    );
                    label_ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.label(tr("Language"));
                    });
                    let mut language_ui = toolbar::cell(
                        ui,
                        egui::Rect::from_min_size(
                            language_row.right_top() - Vec2::new(language_width, 0.0),
                            Vec2::new(language_width, toolbar::HEIGHT),
                        ),
                    );
                    egui::ComboBox::from_id_salt("display-language")
                        .width(language_width)
                        .selected_text(localization::name(&self.prefs.language))
                        .show_ui(&mut language_ui, |ui| {
                            for (code, name) in localization::LANGUAGES {
                                ui.selectable_value(
                                    &mut self.prefs.language,
                                    code.to_owned(),
                                    name,
                                );
                            }
                        });
                    ui.add_space(8.0);
                    if let Some(index) = toolbar::setting_choice(
                        ui,
                        "text-size",
                        "Text size",
                        &["Small", "Medium", "Large"],
                        self.prefs.text_scale.min(2) as usize,
                    ) {
                        self.prefs.text_scale = index as u8;
                        set_text_scale(self.prefs.text_scale);
                        apply_theme(ui.ctx(), self.prefs.light_mode);
                    }
                    ui.add_space(8.0);
                    if let Some(index) = toolbar::setting_choice(
                        ui,
                        "theme-choice",
                        "Theme",
                        &["Dark", "Light"],
                        usize::from(self.prefs.light_mode),
                    ) {
                        self.prefs.light_mode = index == 1;
                    }
                    ui.add_space(8.0);
                    if let Some(index) = toolbar::setting_choice(
                        ui,
                        "sidebar-choice",
                        "Sidebar design",
                        &["Modern", "Classic"],
                        usize::from(self.prefs.classic_sidebar),
                    ) {
                        self.prefs.classic_sidebar = index == 1;
                    }
                    ui.label(
                        RichText::new(tr(
                            "Classic restores the previous sidebar. Switch designs at any time.",
                        ))
                        .size(text_size(12.0))
                        .color(muted()),
                    );
                    setting_toggle(
                        ui,
                        "Classic app screens",
                        "Restore the previous app manager, collection and workspace layouts.",
                        &mut self.prefs.classic_app_screens,
                    );
                    ui.add_space(8.0);
                    setting_toggle(
                        ui,
                        "Compact sidebar",
                        "Keep navigation small, with app icons and tooltips.",
                        &mut self.prefs.compact_sidebar,
                    );
                    setting_toggle(
                        ui,
                        "Reduce motion",
                        "Use static splash artwork and instant page transitions.",
                        &mut self.prefs.reduce_motion,
                    );
                    let selected = match self.prefs.project_view {
                        ProjectView::List => 0,
                        ProjectView::Grid => 1,
                        ProjectView::Waterfall => 2,
                    };
                    if let Some(index) = toolbar::setting_choice(
                        ui,
                        "default-project-layout",
                        "Project layout",
                        &["List", "Grid", "Waterfall"],
                        selected,
                    ) {
                        self.prefs.project_view = match index {
                            1 => ProjectView::Grid,
                            2 => ProjectView::Waterfall,
                            _ => ProjectView::List,
                        };
                    }
                },
            );
        }
        if self.settings_tab == 1 {
            settings_section(
                ui,
                "Master Suite updates",
                "Keep this manager up to date from its official GitHub releases.",
                |ui| {
                    setting_toggle(
                        ui,
                        "Update Master Suite automatically",
                        "Check at startup and every 4 hours. Download verified updates and restart when idle.",
                        &mut self.prefs.automatic_suite_updates,
                    );
                    let previous_beta = self.prefs.beta_suite_updates;
                    ui.add_enabled_ui(!self.suite_update_busy && !platform::flatpak(), |ui| {
                setting_toggle(ui, "Allow prerelease updates", "Include published alpha, beta and release-candidate versions of Master Suite. Applies to automatic and manual checks; turn off for stable releases only.", &mut self.prefs.beta_suite_updates);
            });
                    ui.label(
                        RichText::new(tr(if self.prefs.beta_suite_updates {
                            "Update channel: stable + prerelease"
                        } else {
                            "Update channel: stable only"
                        }))
                        .size(text_size(12.0))
                        .color(muted()),
                    );
                    if previous_beta != self.prefs.beta_suite_updates {
                        self.suite_update_ready = None;
                        self.suite_check_started = None;
                        self.suite_update_status =
                            "Update channel changed. Check for updates to refresh.".into();
                    }
                    ui.label(
                        RichText::new(tr(format!("Installed version: {VERSION}")))
                            .size(text_size(12.0))
                            .color(muted()),
                    );
                    ui.label(
                        RichText::new(tr(&self.suite_update_status))
                            .size(text_size(12.0))
                            .color(muted()),
                    );
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled_ui(!self.suite_update_busy, |ui| {
                                toolbar_secondary_button(ui, "Check for updates")
                            })
                            .inner
                            .clicked()
                        {
                            self.check_suite_update(true);
                        }
                        ui.hyperlink_to(
                            "Release history",
                            "https://github.com/ZifuM/Craft-apps-launcher-installer/releases",
                        );
                        if !self.prefs.automatic_suite_updates
                            && self.suite_update_ready.is_some()
                            && toolbar_secondary_button(ui, "Install and restart").clicked()
                        {
                            if let Some((update, _)) = self.suite_update_ready.take() {
                                save_preferences(&self.prefs);
                                match suite_update::launch(&update) {
                                    Ok(()) => self.tray_quit_requested = true,
                                    Err(error) => self.suite_update_status = error,
                                }
                            }
                        }
                    });
                },
            );
            ui.add_space(16.0);
            settings_section(
                ui,
                "Creative & productivity app updates",
                "Check for app releases while the manager is open. You choose when to install these updates.",
                |ui| {
                    setting_toggle(
                        ui,
                        "Automatic update checks",
                        "Look for new official releases in the background.",
                        &mut self.prefs.automatic_updates,
                    );
                    ui.add_enabled_ui(self.prefs.automatic_updates, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(tr("Check every"));
                            egui::ComboBox::from_id_salt("update-frequency")
                                .selected_text(tr(format!(
                                    "{} hours",
                                    self.prefs.update_interval_hours
                                )))
                                .show_ui(ui, |ui| {
                                    for hours in [1, 2, 4, 8, 12, 24] {
                                        ui.selectable_value(
                                            &mut self.prefs.update_interval_hours,
                                            hours,
                                            tr(format!("{hours} hours")),
                                        );
                                    }
                                });
                        });
                    });
                    setting_toggle(
                        ui,
                        "Update notifications",
                        "Show a notification that stays until you dismiss it.",
                        &mut self.prefs.update_notifications,
                    );
                },
            );
        }
        if self.settings_tab == 2 {
            settings_section(
                ui,
                "Project discovery",
                "Your library stays connected to the folders you choose.",
                |ui| {
                    setting_toggle(
                        ui,
                        "Refresh projects automatically",
                        "Find new, renamed and removed files while the launcher is open.",
                        &mut self.prefs.automatic_project_scan,
                    );
                    ui.add_enabled_ui(self.prefs.automatic_project_scan, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(tr("Refresh every"));
                            egui::ComboBox::from_id_salt("scan-frequency")
                                .selected_text(tr(format!(
                                    "{} minutes",
                                    self.prefs.project_scan_minutes
                                )))
                                .show_ui(ui, |ui| {
                                    for minutes in [1, 3, 5, 10, 15, 30] {
                                        ui.selectable_value(
                                            &mut self.prefs.project_scan_minutes,
                                            minutes,
                                            tr(format!("{minutes} minutes")),
                                        );
                                    }
                                });
                        });
                    });
                },
            );
        }
        if self.settings_tab == 2 {
            egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(14).inner_margin(20).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(tr("Project folders")).size(text_size(16.0)).strong());
            ui.label(RichText::new(tr("Each folder gets a subfolder for every app. The default folder is also used as the app's launch and save location where the app supports it; an app's own saved location can override it.")).size(text_size(12.0)).color(muted()));
            ui.add_space(10.0);
            let mut remove = None;
            let roots = self.prefs.roots.clone();
            let default_root = self.prefs.default_project_root.clone();
            for (i, root) in roots.iter().enumerate() {
                let width = ui.available_width();
                let (row_rect, _) = ui.allocate_exact_size(Vec2::new(width, 40.0), egui::Sense::hover());
                let actions_width = 190.0_f32.min((width - 140.0).max(120.0));
                let path_rect = egui::Rect::from_min_max(row_rect.left_top(), egui::pos2(row_rect.right() - actions_width, row_rect.bottom()));
                ui.painter().with_clip_rect(path_rect).text(path_rect.left_center() + Vec2::new(2.0, 0.0), egui::Align2::LEFT_CENTER, root.display().to_string(), egui::FontId::proportional(text_size(12.0)), theme_rgb(205, 207, 213));
                ui.interact(path_rect, egui::Id::new(("project-root-path", i)), egui::Sense::hover()).on_hover_text(root.display().to_string());
                let actions_rect = egui::Rect::from_min_max(egui::pos2(row_rect.right() - actions_width, row_rect.top()), row_rect.right_bottom());
                let mut actions = ui.new_child(egui::UiBuilder::new().max_rect(actions_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
                actions.spacing_mut().item_spacing.x = 8.0;
                if default_root.as_ref() == Some(root) {
                    actions.add_sized([132.0, 32.0], egui::Label::new(RichText::new(tr("Default save folder")).size(text_size(11.0)).color(ACCENT)));
                } else if actions.add_sized([132.0, 32.0], egui::Button::new(tr("Use as default"))).clicked() {
                    self.prefs.default_project_root = Some(root.clone());
                    save_preferences(&self.prefs);
                    self.toast = Some(format!("{} is now the default app project folder.", root.display()));
                }
                if icon_button(&mut actions, ButtonIcon::Delete, "Stop watching this folder", theme_rgb(213, 103, 111)).clicked() { remove = Some(i); }
            }
            if let Some(i) = remove { self.remove_project_folder(i); }
            if toolbar_secondary_button(ui, "Add folder").clicked() { if let Some(path) = rfd::FileDialog::new().pick_folder() { self.add_project_folder(path); } }
        });
            ui.add_space(12.0);
        }
        if self.settings_tab == 3 {
            settings_section(
                ui,
                "Startup & system tray",
                "Choose how Master Suite runs in the background.",
                |ui| {
                    setting_toggle(
                        ui,
                        "Minimize to system tray",
                        "Keep the app running in the tray when minimized. Close still quits.",
                        &mut self.prefs.minimize_to_tray,
                    );
                    setting_toggle(
                        ui,
                        "Start at login in the tray",
                        "Start quietly when you sign in. Open the app from its tray icon.",
                        &mut self.prefs.start_with_windows,
                    );
                },
            );
        }
        if self.settings_tab == 4 {
            egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(14).inner_margin(20).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(tr("About ArtCraft Master Suite")).size(text_size(16.0)).strong());
            ui.label(RichText::new(tr(format!("Version {VERSION} · Beta"))).size(text_size(12.0)).color(muted()));
            ui.label(RichText::new(tr("Open-source creative apps by Storytold. This independent launcher is not affiliated with Adobe.")).size(text_size(12.0)).color(muted()));
            ui.horizontal(|ui| { if ui.link(tr("ArtCraft apps")).clicked() { open_url("https://getartcraft.com/apps"); } if ui.link(tr("Source repositories")).clicked() { open_url(REPO); } });
        });
            ui.add_space(16.0);
            settings_section(
                ui,
                "App credits",
                "Created by Storytold and the app contributors. Explore each project's source on GitHub.",
                |ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for apps in APPS.chunks(2) {
                        ui.columns(2, |columns| {
                            for (app, ui) in apps.iter().zip(columns.iter_mut()) {
                                ui.push_id(("app-credit", app.id), |ui| {
                                    let (row, _) = ui.allocate_exact_size(
                                        Vec2::new(ui.available_width(), 60.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(row, 10.0, card());
                                    let logo = egui::Rect::from_min_size(
                                        row.min + Vec2::new(12.0, 14.0),
                                        Vec2::splat(32.0),
                                    );
                                    let mut logo_ui =
                                        ui.new_child(egui::UiBuilder::new().max_rect(logo));
                                    self.app_logo(&mut logo_ui, app, 32.0);
                                    let text_width = (row.width() - 186.0).max(20.0);
                                    app_screens::text_at(
                                        ui,
                                        egui::Rect::from_min_size(
                                            row.min + Vec2::new(58.0, 10.0),
                                            Vec2::new(text_width, 23.0),
                                        ),
                                        app.name,
                                        14.0,
                                        foreground(),
                                    );
                                    let repository = format!("storytold/{}", release_slug(app.id));
                                    app_screens::text_at(
                                        ui,
                                        egui::Rect::from_min_size(
                                            row.min + Vec2::new(58.0, 34.0),
                                            Vec2::new(text_width, 18.0),
                                        ),
                                        &repository,
                                        11.0,
                                        muted(),
                                    )
                                    .on_hover_text(&repository);
                                    let button = egui::Rect::from_min_size(
                                        egui::pos2(row.right() - 112.0, row.top() + 13.0),
                                        Vec2::new(100.0, 34.0),
                                    );
                                    let url = format!("{REPO}/{}", release_slug(app.id));
                                    if app_screens::action(
                                        ui,
                                        button,
                                        "credit-github",
                                        "GitHub",
                                        panel(),
                                        readable_app_color(app.tint),
                                        true,
                                    )
                                    .on_hover_text(&url)
                                    .clicked()
                                    {
                                        open_url(&url);
                                    }
                                });
                            }
                        });
                        ui.add_space(4.0);
                    }
                },
            );
        }
        if before != serde_json::to_string(&self.prefs).unwrap_or_default() {
            self.tray_failed = false;
            #[cfg(target_os = "windows")]
            if previous_startup != self.prefs.start_with_windows {
                if let Err(error) = windows_tray::set_startup(self.prefs.start_with_windows) {
                    self.prefs.start_with_windows = previous_startup;
                    self.toast = Some(error);
                }
            }
            #[cfg(target_os = "macos")]
            if previous_startup != self.prefs.start_with_windows {
                if let Err(error) = macos::set_startup(self.prefs.start_with_windows) {
                    self.prefs.start_with_windows = previous_startup;
                    self.toast = Some(error);
                }
            }
            #[cfg(target_os = "linux")]
            if previous_startup != self.prefs.start_with_windows {
                if let Err(error) = linux_tray::set_startup(self.prefs.start_with_windows) {
                    self.prefs.start_with_windows = previous_startup;
                    self.toast = Some(error);
                }
            }
            self.sidebar_collapsed = self.prefs.compact_sidebar;
            if !self.prefs.update_notifications && self.persistent_toast.is_some() {
                self.toast = None;
                self.persistent_toast = None;
            }
            localization::select(&self.prefs.language);
            ui.ctx().request_repaint();
            save_preferences(&self.prefs);
        }
    }
}

fn toolbar_secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    toolbar::standard(ui, label, false)
}
fn toolbar_primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    toolbar::standard(ui, label, true)
}

/// A subtle app-color wash, tessellated within rounded corners.
pub(super) fn identity_mesh(rect: egui::Rect, tint: Color32, hover: bool) -> egui::Mesh {
    let radius = 14.0_f32.min(rect.height() * 0.5);
    let mut mesh = egui::Mesh::default();
    let color = |point: egui::Pos2| {
        let x = ((point.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
        let y = ((point.y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
        mix_color(
            panel(),
            tint,
            (if light_theme() { 0.065 } else { 0.15 }) * (1.0 - x * 0.72) * (1.0 - y)
                + if hover { 0.025 } else { 0.0 },
        )
    };
    mesh.colored_vertex(rect.center(), color(rect.center()));
    for (center, start) in [
        (rect.left_top() + Vec2::splat(radius), std::f32::consts::PI),
        (
            rect.right_top() + Vec2::new(-radius, radius),
            std::f32::consts::PI * 1.5,
        ),
        (rect.right_bottom() - Vec2::splat(radius), 0.0),
        (
            rect.left_bottom() + Vec2::new(radius, -radius),
            std::f32::consts::FRAC_PI_2,
        ),
    ] {
        for step in 0..=8 {
            let point = center
                + Vec2::angled(start + step as f32 / 8.0 * std::f32::consts::FRAC_PI_2) * radius;
            mesh.colored_vertex(point, color(point));
        }
    }
    let count = mesh.vertices.len() as u32 - 1;
    for index in 1..=count {
        mesh.add_triangle(0, index, if index == count { 1 } else { index + 1 });
    }
    mesh
}

pub(super) fn identity_surface(p: &egui::Painter, rect: egui::Rect, tint: Color32, hover: bool) {
    let radius = 14.0_f32.min(rect.height() * 0.5);
    p.add(egui::Shape::mesh(identity_mesh(rect, tint, hover)));
    p.rect_stroke(
        rect,
        radius,
        egui::Stroke::new(
            1.0_f32,
            mix_color(border(), tint, if hover { 0.4 } else { 0.09 }),
        ),
        egui::StrokeKind::Inside,
    );
}
