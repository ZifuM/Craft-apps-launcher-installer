//! App-library layouts use bounded rows so variable descriptions and errors never
//! shift the controls on adjacent cards. The prior screens remain selectable.
use super::*;
use crate::windows_ui::toolbar as tools;

/// Shared, wrapping notice for features that are still under development.
pub(super) fn experimental_banner(ui: &mut egui::Ui) {
    let amber = if light_theme() {
        Color32::from_rgb(145, 93, 15)
    } else {
        Color32::from_rgb(222, 178, 99)
    };
    egui::Frame::new().fill(mix_color(panel(),amber,0.06)).corner_radius(8).inner_margin(egui::Margin::symmetric(12,7)).show(ui,|ui|{
        ui.set_width(ui.available_width());
        ui.spacing_mut().interact_size.y=18.0;
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(tr("Experimental")).size(text_size(12.0)).strong().color(amber));
            ui.label(RichText::new(tr("These features are still being developed and may not function as intended.")).size(text_size(12.0)).color(muted()));
        });
    });
    ui.add_space(10.0);
}

pub(super) fn text_at(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    text: impl Into<String>,
    size: f32,
    color: Color32,
) -> egui::Response {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    // Keep the scroll viewport clip: replacing it lets scrolled text paint over the header.
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child.add(
        egui::Label::new(
            RichText::new(tr(text.into()))
                .size(text_size(size))
                .color(color),
        )
        .truncate(),
    )
}
pub(super) fn action(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: impl std::hash::Hash,
    label: &str,
    fill: Color32,
    text: Color32,
    enabled: bool,
) -> egui::Response {
    let enabled = enabled && ui.is_enabled();
    let response = ui.interact(
        rect,
        ui.id().with(id),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let fill = if enabled { fill } else { card() };
    let hover = enabled && response.hovered();
    let painter = ui.painter_at(rect.expand(2.0));
    painter.rect_filled(
        rect,
        9.0,
        if hover {
            mix_color(fill, foreground(), 0.08)
        } else {
            fill
        },
    );
    painter.rect_stroke(
        rect,
        9.0,
        egui::Stroke::new(
            1.0_f32,
            if response.has_focus() {
                ACCENT
            } else {
                mix_color(fill, text, 0.13)
            },
        ),
        egui::StrokeKind::Inside,
    );
    let translated = tr(label);
    let mut font_size = text_size(13.0);
    let mut galley = painter.layout_no_wrap(
        translated.clone(),
        egui::FontId::proportional(font_size),
        if enabled { text } else { muted() },
    );
    if galley.size().x > rect.width() - 16.0 {
        font_size = (font_size * (rect.width() - 16.0) / galley.size().x).max(10.0);
        galley = painter.layout_no_wrap(
            translated.clone(),
            egui::FontId::proportional(font_size),
            if enabled { text } else { muted() },
        );
    }
    painter.with_clip_rect(rect.shrink(6.0)).galley(
        rect.center() - galley.size() * 0.5,
        galley,
        if enabled { text } else { muted() },
    );
    let response = response.on_hover_text(translated);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}
pub(super) fn ink_for(tint: Color32) -> Color32 {
    // Choose the higher-contrast text color using linear sRGB luminance.
    let linear = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance =
        0.2126 * linear(tint.r()) + 0.7152 * linear(tint.g()) + 0.0722 * linear(tint.b());
    if luminance > 0.179 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

impl Launcher {
    fn library_header(&mut self, ui: &mut egui::Ui, collection: bool) {
        tools::style(ui);
        if tools::header(
            ui,
            if collection {
                "Your apps"
            } else {
                "App Manager"
            },
            if collection {
                "Your tools, ready to work. Launch an app or open its workspace."
            } else {
                "Creative tools and everyday essentials, together."
            },
            if collection {
                Page::YourApps
            } else {
                Page::Apps
            },
            if collection {
                "Manage apps"
            } else {
                "Browse projects"
            },
            false,
        ) {
            self.page = if collection {
                Page::Apps
            } else {
                Page::Projects
            };
        }
    }

    fn library_updates(&mut self, ui: &mut egui::Ui) {
        let apps: Vec<_> = APPS
            .iter()
            .copied()
            .filter(|app| {
                self.states.get(app.id).is_some_and(|s| {
                    s.installed.is_some()
                        && s.latest.is_some()
                        && s.latest != s.installed
                        && s.busy.is_none()
                })
            })
            .collect();
        if apps.is_empty() {
            return;
        }
        egui::Frame::new()
            .fill(mix_color(panel(), ACCENT, 0.06))
            .corner_radius(10)
            .inner_margin(14)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("{}  {}", apps.len(), tr("Available updates")))
                            .font(tools::font(13.0, true)),
                    );
                    ui.add_space(6.0);
                    if tools::standard(ui, "Update all", true).clicked() {
                        for app in &apps {
                            self.install(*app);
                        }
                    }
                    if tools::standard(ui, "View apps", false).clicked() {
                        self.filter = "Available updates".into();
                        self.manager_search.clear();
                        self.page = Page::Apps;
                        if let Some(app) = apps.first() {
                            self.app_category = app.group;
                        }
                    }
                });
            });
        ui.add_space(16.0);
    }

    pub(super) fn app_action_menu(&mut self, ui: &mut egui::Ui, app: AppInfo) {
        use crate::windows_ui::menus;
        menus::style(ui);
        let state = self.states.get(app.id).cloned().unwrap_or_default();
        let available = state.busy.is_none();
        self.app_menu_shortcuts(ui, app);
        if state.installed.is_some() && state.latest.is_some() && state.latest != state.installed {
            if menus::item(ui, "Install available update", available, false).clicked() {
                self.install(app);
                ui.close_menu();
            }
        }
        if menus::item(ui, "Check for updates", available, false).clicked() {
            self.check_app_release(app);
            ui.close_menu();
        }
        if menus::item(ui, "Properties", true, false).clicked() {
            self.properties = Some(menus::Properties::app(app));
            ui.close_menu();
        }
        if menus::item(ui, "Source repository", true, false).clicked() {
            open_url(&format!("{REPO}/{}", release_slug(app.id)));
            ui.close_menu();
        }
        ui.separator();
        if menus::item(
            ui,
            "Uninstall…",
            available && state.installed.is_some(),
            true,
        )
        .clicked()
        {
            self.show_remove = Some(app.id.into());
            ui.close_menu();
        }
    }

    fn library_toolbar(&mut self, ui: &mut egui::Ui, collection: bool) {
        let installed = |group: Option<AppGroup>| {
            APPS.iter()
                .filter(|a| {
                    group.is_none_or(|g| a.group == g)
                        && self.states.get(a.id).is_some_and(|s| s.installed.is_some())
                })
                .count()
        };
        let counts = [
            installed(None),
            installed(Some(AppGroup::Creative)),
            installed(Some(AppGroup::Office)),
        ];
        let row = tools::row(ui);
        // Reflow the search/actions below navigation on smaller windows.
        let wide = row.width() >= if collection { 910.0 } else { 1080.0 };
        let nav_width: f32 = if collection { 440.0 } else { 336.0 };
        let nav_rect = egui::Rect::from_min_size(
            row.min,
            Vec2::new(nav_width.min(row.width()), tools::HEIGHT),
        );
        if collection {
            if let Some(index) = tools::segments(
                ui,
                nav_rect,
                "collection-groups",
                &[
                    ("All apps", counts[0]),
                    ("Creative", counts[1]),
                    ("Productivity", counts[2]),
                ],
                self.collection_group as usize,
            ) {
                self.collection_group = index as u8;
            }
        } else {
            let creative = APPS
                .iter()
                .filter(|a| a.group == AppGroup::Creative)
                .count();
            if let Some(index) = tools::segments(
                ui,
                nav_rect,
                "manager-groups",
                &[
                    ("Creative", creative),
                    ("Productivity", APPS.len() - creative),
                ],
                if self.app_category == AppGroup::Creative {
                    0
                } else {
                    1
                },
            ) {
                self.app_category = if index == 0 {
                    AppGroup::Creative
                } else {
                    AppGroup::Office
                };
            }
        }
        let controls = if wide {
            egui::Rect::from_min_max(row.min + Vec2::new(nav_width + 20.0, 0.0), row.max)
        } else {
            ui.add_space(8.0);
            tools::row(ui)
        };
        let reserve = if collection { 0.0 } else { 342.0 };
        let search_rect =
            egui::Rect::from_min_max(controls.min, controls.max - Vec2::new(reserve, 0.0));
        tools::search(
            ui,
            search_rect,
            if collection {
                "collection-search"
            } else {
                "manager-search"
            },
            if collection {
                &mut self.your_apps_search
            } else {
                &mut self.manager_search
            },
            if collection {
                "Search installed apps"
            } else {
                "Search apps"
            },
        );
        if !collection {
            tools::select(
                ui,
                egui::Rect::from_min_size(
                    egui::pos2(controls.right() - 330.0, controls.top()),
                    Vec2::new(164.0, tools::HEIGHT),
                ),
                "manager-status",
                &mut self.filter,
                &[
                    "All apps",
                    "Installed",
                    "Not installed",
                    "Available updates",
                    "Needs attention",
                ],
            );
            if tools::button(
                ui,
                egui::Rect::from_min_size(
                    egui::pos2(controls.right() - 154.0, controls.top()),
                    Vec2::new(154.0, tools::HEIGHT),
                ),
                "check-app-updates",
                if self.release_check_busy {
                    "Checking…"
                } else {
                    "Check updates"
                },
                tools::Glyph::Refresh,
                false,
                !self.release_check_busy,
            )
            .clicked()
            {
                self.checked = false;
                self.check_releases();
            }
        }
        // Match the final grid-row spacing plus the community section spacing.
        ui.add_space(if collection { 4.0 } else { 36.0 });
    }
    pub(super) fn apps_page(&mut self, ui: &mut egui::Ui) {
        if self.prefs.classic_app_screens {
            self.classic_apps_page(ui);
            return;
        }
        self.library_header(ui, false);
        self.library_toolbar(ui, false);
        self.library_updates(ui);
        let query = self.manager_search.trim().to_lowercase();
        let apps: Vec<_> = APPS
            .iter()
            .copied()
            .filter(|app| app.group == self.app_category)
            .filter(|app| {
                format!("{} {} {}", app.name, app.category, app.blurb)
                    .to_lowercase()
                    .contains(&query)
            })
            .filter(|app| {
                let state = self.states.get(app.id);
                match self.filter.as_str() {
                    "Installed" => state.is_some_and(|s| s.installed.is_some()),
                    "Not installed" => state.is_none_or(|s| s.installed.is_none()),
                    "Needs attention" => state.is_some_and(|s| s.error.is_some()),
                    "Available updates" => state.is_some_and(|s| {
                        s.installed.is_some() && s.latest.is_some() && s.latest != s.installed
                    }),
                    _ => true,
                }
            })
            .collect();
        if !self.manager_search.is_empty() || self.filter != "All apps" {
            if ui.small_button(tr("Reset filters")).clicked() {
                self.manager_search.clear();
                self.filter = "All apps".into();
            }
        }
        egui::ScrollArea::vertical()
            .id_salt("manager-redesign")
            .show(ui, |ui| {
                if apps.is_empty() {
                    self.library_empty(ui, false);
                } else {
                    self.library_grid(ui, &apps, false);
                }
                self.manager_source_links(ui);
            });
    }

    fn manager_source_links(&self, ui: &mut egui::Ui) {
        tools::divider(ui);
        ui.label(
            RichText::new(tr("Links & community"))
                .font(tools::font(14.0, true))
                .color(foreground()),
        );
        ui.label(
            RichText::new(tr(
                "Official ArtCraft resources and the Master Suite project.",
            ))
            .font(tools::font(12.0, false))
            .color(muted()),
        );
        ui.add_space(8.0);
        let links = [
            ("Official website", "https://getartcraft.com/apps"),
            ("ArtCraft Discord", "https://discord.gg/artcraft"),
            ("ArtCraft GitHub", "https://github.com/storytold"),
            (
                "Master Suite GitHub",
                "https://github.com/ZifuM/Craft-apps-launcher-installer",
            ),
        ];
        let columns = if ui.available_width() >= 920.0 {
            4
        } else if ui.available_width() >= 460.0 {
            2
        } else {
            1
        };
        for row in links.chunks(columns) {
            ui.columns(columns, |cells| {
                for (index, (label, url)) in row.iter().enumerate() {
                    let ui = &mut cells[index];
                    let rect = tools::row(ui);
                    if tools::button(ui, rect, url, label, tools::Glyph::External, false, true)
                        .on_hover_text(*url)
                        .clicked()
                    {
                        open_url(url);
                    }
                }
            });
            ui.add_space(4.0);
        }
    }
    pub(super) fn your_apps_page(&mut self, ui: &mut egui::Ui) {
        if self.prefs.classic_app_screens {
            self.classic_your_apps_page(ui);
            return;
        }
        self.library_header(ui, true);
        self.library_toolbar(ui, true);
        self.library_updates(ui);
        let query = self.your_apps_search.trim().to_lowercase();
        let apps: Vec<_> = APPS
            .iter()
            .copied()
            .filter(|a| self.states.get(a.id).is_some_and(|s| s.installed.is_some()))
            .filter(|a| {
                self.collection_group == 0
                    || (self.collection_group == 1 && a.group == AppGroup::Creative)
                    || (self.collection_group == 2 && a.group == AppGroup::Office)
            })
            .filter(|a| {
                format!("{} {} {}", a.name, a.category, a.blurb)
                    .to_lowercase()
                    .contains(&query)
            })
            .collect();
        if !self.your_apps_search.is_empty() || self.collection_group != 0 {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr(format!("{} apps", apps.len())))
                        .font(tools::font(12.0, false))
                        .color(muted()),
                );
                if ui.small_button(tr("Reset filters")).clicked() {
                    self.your_apps_search.clear();
                    self.collection_group = 0;
                }
            });
            ui.add_space(8.0);
        }
        if apps.is_empty() {
            self.library_empty(ui, true);
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("collection-redesign")
            .show(ui, |ui| {
                for (group, label) in [
                    (AppGroup::Creative, "Creative workspace"),
                    (AppGroup::Office, "Productivity workspace"),
                ] {
                    let group_apps: Vec<_> =
                        apps.iter().copied().filter(|a| a.group == group).collect();
                    if group_apps.is_empty() {
                        continue;
                    }
                    ui.label(
                        RichText::new(tr(format!("{label}   /   {}", group_apps.len())))
                            .size(text_size(16.0))
                            .strong(),
                    );
                    ui.add_space(12.0);
                    self.library_grid(ui, &group_apps, true);
                    ui.add_space(14.0);
                }
            });
    }
    fn library_empty(&mut self, ui: &mut egui::Ui, collection: bool) {
        settings_section(
            ui,
            if collection {
                "No apps to show"
            } else {
                "No matching apps"
            },
            if collection {
                "Try another search or install an app to start your collection."
            } else {
                "Try another search, category or installation filter."
            },
            |ui| {
                if secondary_button(
                    ui,
                    if collection {
                        "Browse App Manager"
                    } else {
                        "Clear filters"
                    },
                )
                .clicked()
                {
                    if collection {
                        self.page = Page::Apps;
                    } else {
                        self.manager_search.clear();
                        self.filter = "All apps".into();
                    }
                }
            },
        );
    }
    fn library_grid(&mut self, ui: &mut egui::Ui, apps: &[AppInfo], collection: bool) {
        let columns = ((ui.available_width() + 20.0) / 320.0)
            .floor()
            .clamp(1.0, 4.0) as usize;
        ui.spacing_mut().item_spacing.x = 20.0;
        for row in apps.chunks(columns) {
            ui.columns(columns, |columns| {
                for (i, app) in row.iter().enumerate() {
                    columns[i].push_id(app.id, |ui| self.library_card(ui, *app, collection));
                }
            });
            ui.add_space(16.0);
        }
    }
    fn library_card(&mut self, ui: &mut egui::Ui, app: AppInfo, _collection: bool) {
        let state = self.states.get(app.id).cloned().unwrap_or_default();
        let installed = state.installed.is_some();
        let updating = installed && state.latest.is_some() && state.latest != state.installed;
        let height = text_size(236.0);
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), height),
            egui::Sense::click(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                format!("{} — {}", app.name, tr("Open workspace")),
            )
        });
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        crate::windows_ui::identity_surface(
            ui.painter(),
            rect,
            app.tint,
            response.hovered() || response.has_focus(),
        );
        let painter = ui.painter_at(rect.shrink(1.0));
        let logo = egui::Rect::from_min_size(rect.min + Vec2::splat(20.0), Vec2::splat(54.0));
        if let Some(texture) = &state.icon {
            painter.image(
                texture.id(),
                logo,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            painter.rect_filled(logo, 12.0, app.tint);
            painter.text(
                logo.center(),
                egui::Align2::CENTER_CENTER,
                &app.name[..1],
                tools::font(25.0, true),
                ink_for(app.tint),
            );
        }
        let (status, color) = if state.busy.is_some() {
            ("Working…", readable_app_color(app.tint))
        } else if state.error.is_some() {
            ("Needs attention", theme_rgb(225, 112, 114))
        } else if updating {
            ("Update available", readable_app_color(app.tint))
        } else if installed {
            ("Installed", theme_rgb(102, 192, 154))
        } else {
            ("Not installed", muted())
        };
        let status_width = (rect.width() - 114.0).max(20.0);
        let status_galley = painter.layout_no_wrap(tr(status), tools::font(10.0, true), color);
        let badge_width = (status_galley.size().x + 20.0).min(status_width);
        let badge = egui::Rect::from_min_size(
            egui::pos2(rect.right() - 20.0 - badge_width, rect.top() + 22.0),
            Vec2::new(badge_width, 25.0),
        );
        painter.rect_filled(badge, 6.0, mix_color(panel(), color, 0.075));
        painter.with_clip_rect(badge.shrink(5.0)).galley(
            badge.center() - status_galley.size() * 0.5,
            status_galley,
            color,
        );
        let status_response = ui.interact(badge, ui.id().with("app-status"), egui::Sense::hover());
        status_response.on_hover_text(
            state
                .error
                .as_deref()
                .or(state.busy.as_deref())
                .map(tr)
                .unwrap_or_else(|| tr(status)),
        );
        let mut name = tools::cell(
            ui,
            egui::Rect::from_min_size(
                rect.min + Vec2::new(20.0, 88.0),
                Vec2::new(rect.width() - 40.0, text_size(31.0)),
            ),
        );
        name.add(
            egui::Label::new(RichText::new(app.name).font(tools::font(23.0, true))).truncate(),
        );
        text_at(
            ui,
            egui::Rect::from_min_size(
                rect.min + Vec2::new(20.0, 88.0 + text_size(33.0)),
                Vec2::new(rect.width() - 40.0, text_size(18.0)),
            ),
            app.category,
            10.0,
            readable_app_color(app.tint),
        );
        let mut description = tools::cell(
            ui,
            egui::Rect::from_min_size(
                rect.min + Vec2::new(20.0, 88.0 + text_size(59.0)),
                Vec2::new(rect.width() - 40.0, text_size(24.0)),
            ),
        );
        description
            .add(
                egui::Label::new(
                    RichText::new(tr(compact_description(app.id)))
                        .font(tools::font(12.0, false))
                        .color(muted()),
                )
                .truncate(),
            )
            .on_hover_text(tr(app.blurb));
        let menu_rect = egui::Rect::from_min_size(
            rect.right_bottom() - Vec2::new(52.0, 54.0),
            Vec2::splat(34.0),
        );
        let mut menu_ui = tools::cell(ui, menu_rect);
        crate::windows_ui::menus::more(
            &mut menu_ui,
            &format!("{} — {}", app.name, tr("Details and actions")),
            |ui| self.app_action_menu(ui, app),
        );
        response.context_menu(|ui| self.app_action_menu(ui, app));
        let primary = egui::Rect::from_min_size(
            rect.left_bottom() + Vec2::new(20.0, -54.0),
            Vec2::new(118.0, 34.0),
        );
        let enabled = state.busy.is_none() && (installed || app.has_release);
        if action(
            ui,
            primary,
            "card-primary",
            if state.busy.is_some() {
                "Working…"
            } else if installed {
                "Open app"
            } else if app.has_release {
                "Install app"
            } else {
                "Coming soon"
            },
            app.tint,
            ink_for(app.tint),
            enabled,
        )
        .clicked()
        {
            if installed {
                self.launch(app, None);
            } else {
                self.install(app);
            }
        }
        let update_rect = egui::Rect::from_min_size(
            primary.right_top() + Vec2::new(8.0, 0.0),
            Vec2::new(
                (menu_rect.left() - primary.right() - 16.0)
                    .min(100.0)
                    .max(0.0),
                34.0,
            ),
        );
        if updating && update_rect.width() >= 70.0 {
            if action(
                ui,
                update_rect,
                "card-update",
                "Update",
                panel(),
                foreground(),
                state.busy.is_none(),
            )
            .clicked()
            {
                self.install(app);
            }
        }
        let child_click = ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| {
            menu_rect.contains(p) || primary.contains(p) || (updating && update_rect.contains(p))
        });
        if response.clicked() && !child_click {
            self.detail_parent = Page::Apps;
            self.page = Page::App(app.id);
        }
    }
}

fn compact_description(id: &str) -> &'static str {
    match id {
        "photocraft" => "Edit photos and layered artwork.",
        "vectorcraft" => "Create scalable illustrations.",
        "filmcraft" => "Edit videos and add effects.",
        "lightcraft" => "Organize and develop photos.",
        "printcraft" => "Edit and organize PDFs.",
        "effectcraft" => "Create motion and visual effects.",
        "designcraft" => "Design layouts for print.",
        "soundcraft" => "Record, edit and mix audio.",
        "cadcraft" => "Create precise technical drawings.",
        "gridcraft" => "Work with data and spreadsheets.",
        "wordcraft" => "Write and format documents.",
        "deckcraft" => "Create and present slides.",
        _ => "Open your creative workspace.",
    }
}
