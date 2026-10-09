//! V2: Creative Cloud-style app catalog and workspace shell, with paired light/dark palettes.
use super::*;

fn mask_banner_corners(painter: &egui::Painter, rect: egui::Rect, background: Color32) {
    let radius = (UI_RADIUS as f32).min(rect.width() * 0.5).min(rect.height() * 0.5);
    let mut mask = painter.clone();
    // The surrounding page is opaque even during the content entrance animation.
    mask.set_opacity(1.0);
    for (corner, direction) in [
        (rect.left_top(), Vec2::new(1.0, 1.0)),
        (rect.right_top(), Vec2::new(-1.0, 1.0)),
        (rect.right_bottom(), Vec2::new(-1.0, -1.0)),
        (rect.left_bottom(), Vec2::new(1.0, -1.0)),
    ] {
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(corner, background);
        for step in 0..=16 {
            let angle = std::f32::consts::FRAC_PI_2 * step as f32 / 16.0;
            mesh.colored_vertex(corner + Vec2::new(
                direction.x * radius * (1.0 - angle.cos()),
                direction.y * radius * (1.0 - angle.sin()),
            ), background);
            if step > 0 { mesh.add_triangle(0, step, step + 1); }
        }
        mask.add(egui::Shape::mesh(mesh));
    }
}

fn outline_button(label: &str) -> impl egui::Widget {
    let label = tr(label);
    move |ui: &mut egui::Ui| {
        let response = ui
            .scope(|ui| {
                let widgets = &mut ui.visuals_mut().widgets;
                for (widget, fill, stroke) in [
                    (&mut widgets.inactive, Color32::TRANSPARENT, muted()),
                    (&mut widgets.hovered, neutral(58, 240), foreground()),
                    (&mut widgets.active, neutral(68, 229), foreground()),
                ] {
                    widget.bg_fill = fill;
                    widget.weak_bg_fill = fill;
                    widget.bg_stroke = egui::Stroke::new(1.3_f32, stroke);
                }
                ui.add(
                    egui::Button::new(RichText::new(label).color(foreground()))
                        .corner_radius(UI_RADIUS)
                        .min_size(Vec2::new(70.0, 30.0)),
                )
            })
            .inner;
        if response.has_focus() {
            ui.painter().rect_stroke(
                response.rect,
                UI_RADIUS,
                egui::Stroke::new(2.0_f32, accent()),
                egui::StrokeKind::Inside,
            );
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}
fn more_menu(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    let menu = egui::menu::menu_custom_button(
        ui,
        egui::Button::new(RichText::new("…").size(18.0))
            .frame(false)
            .min_size(Vec2::new(26.0, 30.0)),
        |ui| {
            windows_ui::menus::style(ui);
            contents(ui);
        },
    );
    menu.response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            tr("More actions"),
        )
    });
    menu.response.on_hover_text(tr("More actions"));
}
fn has_update(state: &AppState) -> bool {
    state.installed.is_some() && state.latest.is_some() && state.latest != state.installed
}
fn category_label(index: u8) -> &'static str {
    match index {
        1 => "Photography",
        2 => "Design and layout",
        3 => "Video and motion",
        4 => "Illustration and CAD",
        5 => "Audio",
        6 => "Documents and data",
        _ => "All Apps",
    }
}
fn category_matches(index: u8, app: &AppInfo) -> bool {
    match index {
        1 => matches!(app.id, "photocraft" | "lightcraft"),
        2 => matches!(app.id, "designcraft" | "deckcraft"),
        3 => matches!(app.id, "filmcraft" | "effectcraft"),
        4 => matches!(app.id, "vectorcraft" | "cadcraft"),
        5 => app.id == "soundcraft",
        6 => matches!(app.id, "printcraft" | "gridcraft" | "wordcraft"),
        _ => true,
    }
}
#[derive(Clone, Copy)]
enum Glyph {
    None,
    Grid,
    Refresh,
    Camera,
    Layout,
    Film,
    Pen,
    Audio,
    Document,
    Folder,
    Desktop,
    Settings,
    Cloud,
    Home,
    Theme,
    Globe,
    Code,
    Workspace,
}

fn paint_glyph(painter: &egui::Painter, rect: egui::Rect, glyph: Glyph, color: Color32) {
    let rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(16.0));
    let c = rect.center();
    let p = |x: f32, y: f32| rect.min + Vec2::new(x, y);
    let stroke = egui::Stroke::new(1.25_f32, color);
    let line = |a, b| {
        painter.line_segment([a, b], stroke);
    };
    let box_at = |x, y, w, h| {
        painter.rect_stroke(
            egui::Rect::from_min_size(p(x, y), Vec2::new(w, h)),
            0,
            stroke,
            egui::StrokeKind::Inside,
        );
    };
    match glyph {
        Glyph::None => {}
        Glyph::Grid => {
            for y in [2.0, 7.0, 12.0] {
                for x in [2.0, 7.0, 12.0] {
                    painter.rect_filled(
                        egui::Rect::from_min_size(p(x, y), Vec2::splat(2.5)),
                        0,
                        color,
                    );
                }
            }
        }
        Glyph::Desktop => {
            box_at(1.0, 2.0, 14.0, 10.0);
            line(p(8.0, 12.0), p(8.0, 15.0));
            line(p(4.0, 15.0), p(12.0, 15.0));
        }
        Glyph::Camera => {
            box_at(1.0, 4.0, 14.0, 10.0);
            box_at(5.0, 2.0, 6.0, 3.0);
            painter.circle_stroke(p(8.0, 9.0), 3.0, stroke);
        }
        Glyph::Layout | Glyph::Workspace => {
            box_at(1.0, 2.0, 14.0, 12.0);
            line(p(1.0, 6.0), p(15.0, 6.0));
            line(p(6.0, 6.0), p(6.0, 14.0));
        }
        Glyph::Film => {
            box_at(1.0, 2.0, 14.0, 12.0);
            line(p(5.0, 2.0), p(5.0, 14.0));
            line(p(11.0, 2.0), p(11.0, 14.0));
            for y in [5.0, 9.0] {
                line(p(1.0, y), p(5.0, y));
                line(p(11.0, y), p(15.0, y));
            }
        }
        Glyph::Pen => {
            line(p(3.0, 11.0), p(12.0, 2.0));
            line(p(5.0, 13.0), p(14.0, 4.0));
            line(p(12.0, 2.0), p(14.0, 4.0));
            line(p(3.0, 11.0), p(2.0, 14.0));
            line(p(2.0, 14.0), p(5.0, 13.0));
        }
        Glyph::Audio => {
            for (x, h) in [
                (2.0, 4.0),
                (5.0, 10.0),
                (8.0, 14.0),
                (11.0, 7.0),
                (14.0, 3.0),
            ] {
                line(p(x, 8.0 - h / 2.0), p(x, 8.0 + h / 2.0));
            }
        }
        Glyph::Refresh => {
            let points: Vec<_> = (0..25)
                .map(|i| {
                    let a = 0.45 + i as f32 * 5.0 / 24.0;
                    c + Vec2::new(a.cos(), a.sin()) * 6.0
                })
                .collect();
            painter.add(egui::Shape::line(points, stroke));
            line(p(12.0, 1.0), p(12.0, 6.0));
            line(p(12.0, 6.0), p(7.0, 6.0));
        }
        Glyph::Folder => {
            line(p(1.0, 5.0), p(1.0, 14.0));
            line(p(1.0, 14.0), p(15.0, 14.0));
            line(p(15.0, 14.0), p(15.0, 5.0));
            line(p(15.0, 5.0), p(7.0, 5.0));
            line(p(7.0, 5.0), p(5.0, 2.0));
            line(p(5.0, 2.0), p(1.0, 2.0));
            line(p(1.0, 2.0), p(1.0, 5.0));
        }
        Glyph::Theme => {
            painter.circle_stroke(c, 6.0, stroke);
            let mut points = vec![c];
            points.extend((0..17).map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 16.0;
                c + Vec2::new(a.cos(), a.sin()) * 5.0
            }));
            painter.add(egui::Shape::convex_polygon(
                points,
                color,
                egui::Stroke::NONE,
            ));
        }
        Glyph::Globe => {
            painter.circle_stroke(c, 6.5, stroke);
            line(p(1.5, 8.0), p(14.5, 8.0));
            line(p(8.0, 1.5), p(8.0, 14.5));
        }
        Glyph::Code => {
            line(p(5.0, 3.0), p(1.0, 8.0));
            line(p(1.0, 8.0), p(5.0, 13.0));
            line(p(11.0, 3.0), p(15.0, 8.0));
            line(p(15.0, 8.0), p(11.0, 13.0));
        }
        Glyph::Settings => paint_navigation_icon(painter, rect, Page::Settings, color),
        Glyph::Cloud => paint_navigation_icon(painter, rect, Page::Cloud, color),
        Glyph::Home => paint_navigation_icon(painter, rect, Page::Home, color),
        Glyph::Document => paint_navigation_icon(painter, rect, Page::Projects, color),
    }
}
fn chrome_button(ui: &mut egui::Ui, glyph: Glyph, label: &str) -> egui::Response {
    let response = ui.add(
        egui::Button::new("")
            .frame(false)
            .min_size(Vec2::splat(30.0)),
    );
    if response.hovered() || response.has_focus() {
        ui.painter()
            .rect_filled(response.rect, UI_RADIUS, neutral(57, 238));
    }
    paint_glyph(ui.painter(), response.rect, glyph, muted());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tr(label))
    });
    response
        .on_hover_text(tr(label))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}
fn sidebar_item(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    glyph: Glyph,
    compact: bool,
    count: Option<usize>,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            0,
            neutral(
                if selected { 55 } else { 46 },
                if selected { 242 } else { 248 },
            ),
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            UI_RADIUS,
            egui::Stroke::new(1.0_f32, accent()),
            egui::StrokeKind::Inside,
        );
    }
    let icon = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 18.0, rect.center().y),
        Vec2::splat(16.0),
    );
    paint_glyph(
        ui.painter(),
        icon,
        glyph,
        if selected { foreground() } else { muted() },
    );
    if !compact {
        let clip = egui::Rect::from_min_max(
            rect.min + Vec2::new(34.0, 0.0),
            egui::pos2(
                rect.right() - if count.is_some() { 30.0 } else { 6.0 },
                rect.bottom(),
            ),
        );
        ui.painter().with_clip_rect(clip).text(
            egui::pos2(rect.left() + 34.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            tr(label),
            egui::FontId::proportional(text_size(12.0)),
            foreground(),
        );
        if let Some(count) = count {
            let center = egui::pos2(rect.right() - 15.0, rect.center().y);
            ui.painter()
                .circle_filled(center, 9.0, Color32::from_rgb(20, 126, 222));
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                count,
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
        }
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, tr(label))
    });
    response
        .on_hover_text(tr(label))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}
fn sidebar_caption(ui: &mut egui::Ui, label: &str, compact: bool) {
    if !compact {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            ui.label(
                RichText::new(tr(label))
                    .size(text_size(10.0))
                    .color(muted()),
            );
        });
        ui.add_space(6.0);
    }
}
fn sidebar_divider(ui: &mut egui::Ui) {
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
}

fn neutral(dark: u8, light: u8) -> Color32 {
    Color32::from_gray(if light_theme() { light } else { dark })
}
pub(crate) fn foreground() -> Color32 {
    neutral(230, 50)
}
pub(crate) fn ink() -> Color32 {
    neutral(29, 249)
}
pub(crate) fn panel() -> Color32 {
    neutral(36, 255)
}
pub(crate) fn card() -> Color32 {
    neutral(39, 255)
}
pub(crate) fn border() -> Color32 {
    neutral(61, 226)
}
pub(crate) fn muted() -> Color32 {
    neutral(162, 116)
}
fn accent() -> Color32 {
    if light_theme() {
        Color32::from_rgb(13, 102, 208)
    } else {
        Color32::from_rgb(92, 170, 255)
    }
}
fn chrome() -> Color32 {
    neutral(34, 255)
}
fn field() -> Color32 {
    neutral(30, 255)
}
pub(crate) fn primary_button(label: &str) -> impl egui::Widget {
    let label = tr(label);
    move |ui: &mut egui::Ui| {
        let response = ui
            .scope(|ui| {
                let widgets = &mut ui.visuals_mut().widgets;
                for (widget, fill) in [
                    (&mut widgets.inactive, Color32::from_rgb(20, 110, 220)),
                    (&mut widgets.hovered, Color32::from_rgb(13, 96, 198)),
                    (&mut widgets.active, Color32::from_rgb(9, 80, 169)),
                ] {
                    widget.bg_fill = fill;
                    widget.weak_bg_fill = fill;
                    widget.bg_stroke = egui::Stroke::NONE;
                }
                ui.add(
                    egui::Button::new(RichText::new(label).strong().color(Color32::WHITE))
                        .corner_radius(UI_RADIUS)
                        .min_size(Vec2::new(76.0, 30.0)),
                )
            })
            .inner;
        if response.has_focus() {
            ui.painter().rect_stroke(
                response.rect,
                UI_RADIUS,
                egui::Stroke::new(2.0_f32, foreground()),
                egui::StrokeKind::Inside,
            );
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

// Text tabs leave the content unobstructed; the underline carries selection.
fn tab(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let response = ui.add(
        egui::Button::new(RichText::new(tr(label)).strong().color(if selected {
            foreground()
        } else {
            muted()
        }))
        .frame(false)
        .min_size(Vec2::new(0.0, 34.0)),
    );
    if selected || response.hovered() {
        let rect = response.rect.shrink2(Vec2::new(10.0, 0.0));
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(2.0_f32, if selected { foreground() } else { muted() }),
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            UI_RADIUS,
            egui::Stroke::new(1.0_f32, accent()),
            egui::StrokeKind::Inside,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            tr(label),
        )
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn refine_style(style: &mut egui::Style) {
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(12.0, 5.0);
    style.spacing.interact_size = Vec2::new(30.0, 30.0);
    style.spacing.window_margin = egui::Margin::same(20);
    style.spacing.menu_margin = egui::Margin::same(6);
    for (kind, size) in [
        (egui::TextStyle::Heading, 22.0),
        (egui::TextStyle::Body, 13.0),
        (egui::TextStyle::Button, 13.0),
        (egui::TextStyle::Small, 12.0),
    ] {
        style
            .text_styles
            .insert(kind, egui::FontId::proportional(text_size(size)));
    }
    style.visuals.window_fill = panel();
    style.visuals.panel_fill = ink();
    style.visuals.extreme_bg_color = field();
    style.visuals.faint_bg_color = neutral(34, 245);
    style.visuals.override_text_color = Some(foreground());
    style.visuals.hyperlink_color = accent();
    style.visuals.selection.bg_fill = if light_theme() {
        Color32::from_rgb(219, 235, 253)
    } else {
        Color32::from_rgb(47, 72, 99)
    };
    style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, accent());
    style.visuals.window_stroke = egui::Stroke::new(1.0_f32, border());
    style.visuals.window_corner_radius = UI_RADIUS.into();
    style.visuals.menu_corner_radius = UI_RADIUS.into();
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    for widget in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = UI_RADIUS.into();
        widget.bg_fill = card();
        widget.weak_bg_fill = card();
        widget.bg_stroke = egui::Stroke::new(1.0_f32, border());
        widget.fg_stroke = egui::Stroke::new(1.0_f32, foreground());
        widget.expansion = 0.0;
    }
    style.visuals.widgets.noninteractive.bg_fill = panel();
    style.visuals.widgets.noninteractive.weak_bg_fill = panel();
    style.visuals.widgets.noninteractive.fg_stroke.color = muted();
    style.visuals.widgets.hovered.bg_fill = neutral(51, 244);
    style.visuals.widgets.hovered.weak_bg_fill = neutral(51, 244);
    style.visuals.widgets.hovered.bg_stroke.color = neutral(118, 150);
    style.visuals.widgets.active.bg_fill = neutral(47, 233);
    style.visuals.widgets.active.weak_bg_fill = neutral(47, 233);
    style.visuals.widgets.active.bg_stroke.color = accent();
    style.visuals.widgets.open.bg_fill = neutral(47, 233);
    style.visuals.widgets.open.weak_bg_fill = neutral(47, 233);
    style.visuals.widgets.open.bg_stroke.color = accent();
    style.visuals.text_cursor.stroke = egui::Stroke::new(1.5_f32, accent());
}

fn heading(ui: &mut egui::Ui, title: &str, description: &str) {
    ui.label(RichText::new(tr(title)).size(text_size(18.0)).strong());
    if !description.is_empty() {
        ui.label(RichText::new(tr(description)).color(muted()));
    }
    ui.add_space(14.0);
}
fn transition_progress(
    ctx: &egui::Context,
    id: egui::Id,
    key: egui::Id,
    reduce_motion: bool,
) -> f32 {
    let now = ctx.input(|i| i.time);
    let started = ctx.data_mut(|data| {
        let previous = data.get_temp::<(egui::Id, f64)>(id);
        let started = match previous {
            Some((previous_key, started)) if previous_key == key => started,
            Some(_) => now,
            None => now - 0.18,
        };
        data.insert_temp(id, (key, started));
        started
    });
    let progress = if reduce_motion {
        1.0
    } else {
        ((now - started) / 0.18).clamp(0.0, 1.0) as f32
    };
    if progress < 1.0 {
        ctx.request_repaint_after(Duration::from_millis(16));
    }
    1.0 - (1.0 - progress).powi(3)
}
fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(24.0);
    ui.label(RichText::new(tr(title)).size(text_size(14.0)).strong());
    ui.add_space(10.0);
}
fn preferences_section(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    contents: impl FnOnce(&mut egui::Ui),
) {
    let response = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(24, 20))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 10.0;
            if !title.is_empty() {
                ui.label(RichText::new(tr(title)).size(text_size(14.0)).strong());
            }
            if !description.is_empty() {
                ui.add(egui::Label::new(RichText::new(tr(description)).color(muted())).wrap());
            }
            if !title.is_empty() || !description.is_empty() {
                ui.add_space(6.0);
            }
            contents(ui);
        });
    ui.painter().line_segment(
        [
            response.response.rect.left_bottom(),
            response.response.rect.right_bottom(),
        ],
        egui::Stroke::new(1.0_f32, border()),
    );
}

fn preferences_toggle(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    let mut track = egui::Rect::NOTHING;
    let mut response = ui
        .horizontal(|ui| {
            let (rect, control) =
                ui.allocate_exact_size(Vec2::new(30.0, 18.0), egui::Sense::click());
            track = rect;
            let text = ui.add(egui::Label::new(tr(label)).sense(egui::Sense::click()));
            control.union(text)
        })
        .inner;
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let opacity = if ui.is_enabled() { 1.0 } else { 0.45 };
    let fill = if *value {
        Color32::from_rgb(20, 110, 220)
    } else {
        neutral(96, 166)
    };
    ui.painter()
        .rect_filled(track, UI_RADIUS, fill.gamma_multiply(opacity));
    let knob = egui::pos2(
        if *value {
            track.right() - 9.0
        } else {
            track.left() + 9.0
        },
        track.center().y,
    );
    ui.painter()
        .circle_filled(knob, 6.5, Color32::WHITE.gamma_multiply(opacity));
    if response.has_focus() {
        ui.painter().rect_stroke(
            track.expand(2.0),
            UI_RADIUS,
            egui::Stroke::new(1.0_f32, accent()),
            egui::StrokeKind::Inside,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            ui.is_enabled(),
            *value,
            tr(label),
        )
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn line(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal_top(|ui| {
        ui.add_sized(
            [150.0, 22.0],
            egui::Label::new(RichText::new(tr(label)).color(muted())),
        );
        ui.add(egui::Label::new(value).wrap());
    });
}
fn search(ui: &mut egui::Ui, id: &str, value: &mut String, hint: &str, width: f32) {
    let edit = ui.add_sized(
        [width.max(120.0), 30.0],
        egui::TextEdit::singleline(value)
            .id_salt(id)
            .background_color(field())
            .margin(Vec2::new(10.0, 6.0))
            .hint_text(tr(hint)),
    );
    let accepts_shortcuts = ui.ctx().memory(|memory| {
        memory
            .top_modal_layer()
            .is_none_or(|layer| layer == ui.layer_id())
    });
    if accepts_shortcuts && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F))
    {
        edit.request_focus();
    }
}
fn view_options(ui: &mut egui::Ui, value: &mut ProjectView) -> bool {
    let before = *value;
    egui::ComboBox::from_id_salt("v2-view")
        .width(112.0)
        .selected_text(tr(match value {
            ProjectView::List => "List",
            ProjectView::Grid => "Grid",
            ProjectView::Waterfall => "Waterfall",
        }))
        .show_ui(ui, |ui| {
            for (v, label) in [
                (ProjectView::List, "List"),
                (ProjectView::Grid, "Grid"),
                (ProjectView::Waterfall, "Waterfall"),
            ] {
                ui.selectable_value(value, v, tr(label));
            }
        });
    before != *value
}
fn scroll(ui: &mut egui::Ui, id: impl std::hash::Hash, contents: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .show_scoped(ui, |ui| {
            ui.set_width(ui.available_width());
            contents(ui);
        });
}

impl Launcher {
    pub(crate) fn v2_ui(&mut self, ctx: &egui::Context) {
        // Older navigation routes may still refer to the Legacy settings page.
        if self.page == Page::Settings {
            self.page = Page::Apps;
            self.settings_open = true;
        }
        if self.page == Page::Cloud {
            self.page = Page::Apps;
            self.open_cloud();
        }
        egui::TopBottomPanel::top("v2-navigation")
            .frame(
                egui::Frame::new()
                    .fill(chrome())
                    .inner_margin(egui::Margin::symmetric(16, 4)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let logo = ui
                        .add(
                            egui::Image::new((self.brand_icon.id(), Vec2::splat(24.0)))
                                .sense(egui::Sense::click()),
                        )
                        .on_hover_text(tr("Collapse or expand navigation"));
                    if logo.clicked() {
                        self.sidebar_collapsed = !self.sidebar_collapsed;
                        self.prefs.compact_sidebar = self.sidebar_collapsed;
                        save_preferences(&self.prefs);
                    }
                    let search_width = (ui.available_width() - 186.0).clamp(140.0, 320.0);
                    ui.add_space(((ui.available_width() - search_width - 172.0) * 0.5).max(0.0));
                    let original = match self.page {
                        Page::Projects | Page::Home => self.search.clone(),
                        Page::App(id) => self.workspace_search.get(id).cloned().unwrap_or_default(),
                        Page::Cloud => self.cloud.search.clone(),
                        _ => self.manager_search.clone(),
                    };
                    let mut query = original.clone();
                    let hint = match self.page {
                        Page::Projects | Page::Home | Page::App(_) => "Search projects",
                        Page::Cloud => "Search backup files",
                        _ => "Search apps",
                    };
                    search(ui, "v2-global-search", &mut query, hint, search_width);
                    if original != query {
                        match self.page {
                            Page::Projects | Page::Home => {
                                self.search = query;
                                self.page = Page::Projects;
                            }
                            Page::App(id) => {
                                self.workspace_search.insert(id.into(), query);
                                self.workspace_tabs.insert(id.into(), 0);
                            }
                            Page::Cloud => {
                                self.cloud.search = query;
                                self.cloud.tab = 0;
                            }
                            _ => {
                                self.manager_search = query;
                                self.page = Page::Apps;
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if chrome_button(ui, Glyph::Settings, "Settings").clicked() {
                            self.settings_open = true;
                        }
                        if chrome_button(
                            ui,
                            Glyph::Theme,
                            if self.prefs.light_mode {
                                "Use dark mode"
                            } else {
                                "Use light mode"
                            },
                        )
                        .clicked()
                        {
                            self.prefs.light_mode = !self.prefs.light_mode;
                            save_preferences(&self.prefs);
                            apply_theme(ctx, self.prefs.light_mode);
                        }
                        if chrome_button(ui, Glyph::Cloud, "Cloud backups").clicked() {
                            self.open_cloud();
                        }
                        if chrome_button(ui, Glyph::Home, "Home").clicked() {
                            self.page = Page::Home;
                        }
                    });
                });
            });
        if self.prefs.ui_theme != self.active_theme {
            egui::TopBottomPanel::top("v2-pending-theme")
                .frame(egui::Frame::new().fill(card()).inner_margin(8))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr(format!(
                            "{} selected. Restart to apply.",
                            self.prefs.ui_theme.name()
                        )));
                        if ui.link(tr("Themes")).clicked() {
                            self.settings_open = true;
                            self.settings_tab = 5;
                        }
                    });
                });
        }
        egui::SidePanel::left("v2-sidebar")
            .exact_width(if self.sidebar_collapsed { 58.0 } else { 218.0 })
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(chrome())
                    .inner_margin(egui::Margin::symmetric(10, 16)),
            )
            .show(ctx, |ui| self.v2_sidebar(ui));
        egui::TopBottomPanel::top("v2-content-bar")
            .frame(
                egui::Frame::new()
                    .fill(chrome())
                    .inner_margin(egui::Margin::symmetric(26, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.set_min_height(30.0);
                    let title = match self.page {
                        Page::Apps | Page::YourApps => {
                            if self.filter == "Available updates" {
                                "Updates"
                            } else {
                                category_label(self.v2_category)
                            }
                        }
                        Page::Home => "Home",
                        Page::Projects => {
                            if self.projects_tab {
                                "Project folders"
                            } else {
                                "Projects"
                            }
                        }
                        Page::Cloud => "Cloud",
                        Page::Settings => "Preferences",
                        Page::App(id) => app_by_id(id).map(|a| a.name).unwrap_or("Workspace"),
                    };
                    ui.label(RichText::new(tr(title)).strong().size(text_size(14.0)));
                    if matches!(self.page, Page::Apps | Page::YourApps) {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    !self.release_check_busy,
                                    outline_button("Check updates"),
                                )
                                .clicked()
                            {
                                self.checked = false;
                                self.check_releases();
                            }
                        });
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(ink()).inner_margin(egui::Margin {
                left: 26,
                right: 5,
                top: 24,
                bottom: 16,
            }))
            .show(ctx, |ui| {
                let progress = transition_progress(
                    ctx,
                    egui::Id::new("v2-page-transition"),
                    egui::Id::new((self.page, self.v2_category, &self.filter)),
                    self.prefs.reduce_motion,
                );
                ui.multiply_opacity(0.35 + 0.65 * progress);
                ui.add_space((1.0 - progress) * 4.0);
                match self.page {
                    Page::Home => self.v2_home(ui),
                    Page::Apps | Page::YourApps => self.v2_apps(ui),
                    Page::Projects => self.v2_projects(ui, None),
                    Page::App(id) => self.v2_workspace(ui, id),
                    Page::Cloud => self.v2_apps(ui),
                    Page::Settings => self.v2_apps(ui),
                }
            });
    }

    fn v2_sidebar(&mut self, ui: &mut egui::Ui) {
        let compact = self.sidebar_collapsed;
        egui::ScrollArea::vertical()
            .id_salt("v2-side-scroll")
            .show_scoped(ui, |ui| {
                ui.set_width(ui.available_width());
                sidebar_caption(ui, "YOUR WORK", compact);
                for (page, label, glyph) in [
                    (Page::Home, "Home", Glyph::Home),
                    (Page::Projects, "Projects", Glyph::Folder),
                ] {
                    if sidebar_item(ui, label, self.page == page, glyph, compact, None).clicked() {
                        self.page = page;
                    }
                }
                sidebar_divider(ui);
                {
                    sidebar_caption(ui, "APPS", compact);
                    let manager = matches!(self.page, Page::Apps | Page::YourApps);
                    if sidebar_item(
                        ui,
                        "All Apps",
                        manager && self.v2_category == 0 && self.filter != "Available updates",
                        Glyph::Grid,
                        compact,
                        None,
                    )
                    .clicked()
                    {
                        self.page = Page::Apps;
                        self.v2_category = 0;
                        self.filter = "All apps".into();
                    }
                    let updates = APPS
                        .iter()
                        .filter(|a| self.states.get(a.id).is_some_and(has_update))
                        .count();
                    if sidebar_item(
                        ui,
                        "Updates",
                        manager && self.filter == "Available updates",
                        Glyph::Refresh,
                        compact,
                        (updates > 0).then_some(updates),
                    )
                    .clicked()
                    {
                        self.page = Page::Apps;
                        self.v2_category = 0;
                        self.filter = "Available updates".into();
                    }
                    sidebar_divider(ui);
                }
                if sidebar_item(
                    ui,
                    "Settings",
                    self.settings_open,
                    Glyph::Settings,
                    compact,
                    None,
                )
                .clicked()
                {
                    self.settings_open = true;
                }
                sidebar_divider(ui);
                {
                    sidebar_caption(ui, "CATEGORIES", compact);
                    let manager = matches!(self.page, Page::Apps | Page::YourApps);
                    for (index, glyph) in [
                        (1, Glyph::Camera),
                        (2, Glyph::Layout),
                        (3, Glyph::Film),
                        (4, Glyph::Pen),
                        (5, Glyph::Audio),
                        (6, Glyph::Document),
                    ] {
                        if sidebar_item(
                            ui,
                            category_label(index),
                            manager && self.v2_category == index,
                            glyph,
                            compact,
                            None,
                        )
                        .clicked()
                        {
                            self.page = Page::Apps;
                            self.v2_category = index;
                            self.filter = "All apps".into();
                        }
                    }
                }
                let installed: Vec<_> = APPS
                    .iter()
                    .copied()
                    .filter(|a| self.states.get(a.id).is_some_and(|s| s.installed.is_some()))
                    .collect();
                if !installed.is_empty() {
                    sidebar_divider(ui);
                    sidebar_caption(ui, "WORKSPACES", compact);
                    for app in installed {
                        let response = sidebar_item(
                            ui,
                            app.name,
                            self.page == Page::App(app.id),
                            Glyph::None,
                            compact,
                            None,
                        );
                        if let Some(icon) = self.states.get(app.id).and_then(|s| s.icon.as_ref()) {
                            let r = egui::Rect::from_center_size(
                                egui::pos2(response.rect.left() + 18.0, response.rect.center().y),
                                Vec2::splat(19.0),
                            );
                            ui.painter().image(
                                icon.id(),
                                r,
                                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        }
                        if response.clicked() {
                            self.page = Page::App(app.id);
                        }
                    }
                }
                sidebar_divider(ui);
                sidebar_caption(ui, "RESOURCES", compact);
                for (label, glyph, url) in [
                    ("ArtCraft website", Glyph::Globe, "https://getartcraft.com"),
                    ("ArtCraft on GitHub", Glyph::Code, REPO),
                    (
                        "Master Suite source",
                        Glyph::Code,
                        "https://github.com/ZifuM/Craft-apps-launcher-installer",
                    ),
                ] {
                    if sidebar_item(ui, label, false, glyph, compact, None).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                    }
                }
            });
    }

    fn v2_home(&mut self, ui: &mut egui::Ui) {
        scroll(ui, "v2-home", |ui| {
            self.v2_feature_banner(ui, None);
            section(ui, "Recent projects");
            let projects: Vec<_> = self.projects.iter().take(6).cloned().collect();
            self.v2_project_grid(ui, &projects, false);
            ui.add_space(12.0);
            if ui.add(outline_button("Browse all projects")).clicked() {
                self.page = Page::Projects;
            }
            let apps: Vec<_> = APPS
                .iter()
                .copied()
                .filter(|a| self.states.get(a.id).is_some_and(|s| s.installed.is_some()))
                .collect();
            if !apps.is_empty() {
                section(ui, "Installed");
                self.v2_app_grid(ui, &apps);
            }
        });
    }

    fn v2_apps(&mut self, ui: &mut egui::Ui) {
        let query = self.manager_search.trim().to_lowercase();
        let apps: Vec<_> = APPS
            .iter()
            .copied()
            .filter(|app| category_matches(self.v2_category, app))
            .filter(|a| {
                format!("{} {} {}", a.name, a.category, a.blurb)
                    .to_lowercase()
                    .contains(&query)
            })
            .filter(|a| {
                self.filter != "Available updates" || self.states.get(a.id).is_some_and(has_update)
            })
            .collect();
        scroll(ui, "v2-apps", |ui| {
            if self.filter != "Available updates" && query.is_empty() {
                self.v2_feature_banner(ui, apps.first().copied());
            }
            if self.filter == "Available updates" {
                section(ui, "Available updates");
                if apps.is_empty() {
                    ui.label(tr(
                        "Your apps are up to date. Use Check updates to refresh.",
                    ));
                }
                self.v2_app_grid(ui, &apps);
            } else {
                let (installed, available): (Vec<_>, Vec<_>) = apps
                    .into_iter()
                    .partition(|a| self.states.get(a.id).is_some_and(|s| s.installed.is_some()));
                if !installed.is_empty() {
                    section(ui, "Installed");
                    self.v2_app_grid(ui, &installed);
                }
                if !available.is_empty() {
                    section(ui, "Available to install");
                    self.v2_app_grid(ui, &available);
                }
                if installed.is_empty() && available.is_empty() {
                    ui.label(tr("No apps match your search."));
                }
            }
        });
    }

    fn v2_feature_banner(&mut self, ui: &mut egui::Ui, app: Option<AppInfo>) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(width, if width < 640.0 { 180.0 } else { 210.0 }),
            egui::Sense::hover(),
        );
        let painter = ui.painter().with_clip_rect(rect);
        let base = if light_theme() {
            Color32::from_rgb(46, 47, 185)
        } else {
            Color32::from_rgb(43, 42, 141)
        };
        painter.rect_filled(rect, UI_RADIUS, base);
        // Product artwork on the right, using ArtCraft's own application icons.
        if width > 630.0 {
            let featured: Vec<_> = app
                .into_iter()
                .chain(
                    APPS.iter()
                        .copied()
                        .filter(|a| app.is_none_or(|featured| a.id != featured.id)),
                )
                .take(3)
                .collect();
            let mut artwork = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("v2-feature-art")
                    .max_rect(rect),
            );
            artwork.set_clip_rect(rect.intersect(ui.clip_rect()));
            for (i, info) in featured.iter().enumerate() {
                if let Some(icon) = self.states.get(info.id).and_then(|s| s.icon.as_ref()) {
                    let side = if i == 0 { 188.0 } else { 128.0 };
                    let center = match i {
                        0 => egui::pos2(rect.right() - 151.0, rect.top() + 132.0),
                        1 => egui::pos2(rect.right() - 21.0, rect.top() + 52.0),
                        _ => egui::pos2(rect.right() - 305.0, rect.bottom() + 24.0),
                    };
                    let r = egui::Rect::from_center_size(center, Vec2::splat(side));
                    painter.rect_filled(r.expand(7.0), UI_RADIUS, Color32::from_black_alpha(28));
                    let background = self.states.get(info.id).and_then(|s| s.icon_background).unwrap_or(info.tint);
                    painter.rect_filled(r, UI_RADIUS, background);
                    egui::Image::new((icon.id(), Vec2::splat(side)))
                        .corner_radius(UI_RADIUS)
                        .paint_at(&artwork, r);
                }
            }
        }
        // Artwork uses a rectangular scissor. Cover only the four outside corner
        // wedges to keep the banner radius intact when an icon crosses its edge.
        mask_banner_corners(&painter, rect, ink());
        let text_width = if width > 630.0 {
            (width - 390.0).max(240.0)
        } else {
            width - 52.0
        };
        let content = egui::Rect::from_min_size(
            rect.min + Vec2::new(32.0, 40.0),
            Vec2::new(text_width, rect.height() - 58.0),
        );
        let mut banner = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("v2-feature-copy")
                .max_rect(content)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        banner.style_mut().visuals.override_text_color = Some(Color32::WHITE);
        banner.label(
            RichText::new(tr(app
                .map(|a| format!("Create with {}", a.name))
                .unwrap_or_else(|| "Your next project starts here".into())))
            .size(text_size(21.0))
            .strong()
            .color(Color32::WHITE),
        );
        banner.add_space(6.0);
        banner.label(
            RichText::new(tr(app.map(|a| a.blurb).unwrap_or(
                "Your apps, project files and creative work. All together.",
            )))
            .color(Color32::WHITE),
        );
        banner.add_space(12.0);
        let label = app
            .map(|a| format!("Explore {}", a.name))
            .unwrap_or_else(|| "Browse apps".into());
        if banner
            .add(
                egui::Button::new(RichText::new(tr(label)).color(Color32::WHITE).strong())
                    .fill(Color32::TRANSPARENT)
                    .stroke(egui::Stroke::new(1.5_f32, Color32::WHITE))
                    .corner_radius(UI_RADIUS)
                    .min_size(Vec2::new(112.0, 32.0)),
            )
            .clicked()
        {
            self.page = app.map(|a| Page::App(a.id)).unwrap_or(Page::Apps);
        }
    }

    fn v2_app_grid(&mut self, ui: &mut egui::Ui, apps: &[AppInfo]) {
        let columns = if ui.available_width() >= 820.0 {
            3
        } else if ui.available_width() >= 530.0 {
            2
        } else {
            1
        };
        ui.spacing_mut().item_spacing.x = 16.0;
        for row in apps.chunks(columns) {
            ui.columns(columns, |uis| {
                for (app, ui) in row.iter().zip(uis) {
                    self.v2_catalog_card(ui, *app);
                }
            });
            ui.add_space(12.0);
        }
    }

    fn v2_catalog_card(&mut self, ui: &mut egui::Ui, app: AppInfo) {
        let state = self.states.get(app.id).cloned().unwrap_or_default();
        ui.push_id(("v2-card", app.id), |ui| {
            egui::Frame::new()
                .fill(card())
                .stroke(egui::Stroke::new(1.0_f32, border()))
                .corner_radius(UI_RADIUS)
                .show(ui, |ui| {
                    let width = ui.available_width();
                    egui::Frame::new().inner_margin(16).show(ui, |ui| {
                        ui.set_width((width - 32.0).max(120.0));
                        ui.set_min_height(76.0);
                        ui.horizontal(|ui| {
                            if let Some(icon) = &state.icon {
                                ui.add(
                                    egui::Image::new((icon.id(), Vec2::splat(32.0)))
                                        .corner_radius(UI_RADIUS),
                                );
                            }
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(app.name).strong())
                                        .frame(false),
                                )
                                .clicked()
                            {
                                self.page = Page::App(app.id);
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let (r, response) = ui.allocate_exact_size(
                                        Vec2::splat(18.0),
                                        egui::Sense::hover(),
                                    );
                                    paint_glyph(ui.painter(), r, Glyph::Desktop, muted());
                                    response.on_hover_text(platform::label());
                                },
                            );
                        });
                        ui.add_space(4.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(tr(app.blurb))
                                    .size(text_size(12.0))
                                    .color(muted()),
                            )
                            .wrap(),
                        );
                    });
                    let x = ui.max_rect();
                    let y = ui.cursor().top();
                    ui.painter().line_segment(
                        [egui::pos2(x.left(), y), egui::pos2(x.right(), y)],
                        egui::Stroke::new(1.0_f32, border()),
                    );
                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_width((width - 28.0).max(120.0));
                            ui.horizontal(|ui| {
                                if chrome_button(ui, Glyph::Workspace, "Open workspace").clicked() {
                                    self.page = Page::App(app.id);
                                }
                                if let Some(version) = &state.installed {
                                    ui.label(
                                        RichText::new(version).size(text_size(10.0)).color(muted()),
                                    );
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        more_menu(ui, |ui| self.app_action_menu(ui, app));
                                        let update = has_update(&state);
                                        let label = if state.busy.is_some() {
                                            "Working…"
                                        } else if update {
                                            "Update"
                                        } else if state.installed.is_some() {
                                            "Open"
                                        } else if app.has_release {
                                            "Install"
                                        } else {
                                            "Unavailable"
                                        };
                                        let enabled = state.busy.is_none()
                                            && (state.installed.is_some() || app.has_release);
                                        let response = if update || state.installed.is_none() {
                                            ui.add_enabled(enabled, primary_button(label))
                                        } else {
                                            ui.add_enabled(enabled, outline_button(label))
                                        };
                                        if response.clicked() {
                                            if update || state.installed.is_none() {
                                                self.install(app);
                                            } else {
                                                self.launch(app, None);
                                            }
                                        }
                                    },
                                );
                            });
                        });
                });
            if let Some(message) = state.busy.as_ref().or(state.error.as_ref()) {
                ui.label(RichText::new(tr(message)).small().color(muted()));
            }
        });
    }

    fn v2_app_actions(&mut self, ui: &mut egui::Ui, app: AppInfo, state: &AppState) {
        let enabled = state.busy.is_none() && (state.installed.is_some() || app.has_release);
        let action = if state.installed.is_some() {
            ui.add_enabled(enabled, outline_button("Open"))
        } else {
            ui.add_enabled(
                enabled,
                primary_button(if app.has_release {
                    "Install"
                } else {
                    "Unavailable"
                }),
            )
        };
        if action.clicked() {
            if state.installed.is_some() {
                self.launch(app, None);
            } else {
                self.install(app);
            }
        }
        if state.installed.is_some() && state.latest.is_some() && state.latest != state.installed {
            if ui
                .add_enabled(state.busy.is_none(), primary_button("Update"))
                .clicked()
            {
                self.install(app);
            }
        }
        more_menu(ui, |ui| {
            self.app_action_menu(ui, app);
        });
    }

    fn v2_workspace(&mut self, ui: &mut egui::Ui, id: &'static str) {
        let Some(app) = app_by_id(id).copied() else {
            return;
        };
        let profile = app_workspace::profile::for_app(id);
        let state = self.states.get(id).cloned().unwrap_or_default();
        ui.horizontal(|ui| {
            if let Some(icon) = &state.icon {
                ui.add(egui::Image::new((icon.id(), Vec2::splat(44.0))).corner_radius(UI_RADIUS));
            }
            ui.vertical(|ui| {
                ui.label(RichText::new(app.name).size(text_size(24.0)).strong());
                ui.label(RichText::new(tr(app.category)).color(muted()));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.v2_app_actions(ui, app, &state);
            });
        });
        ui.add_space(12.0);
        ui.label(tr(profile.summary));
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    tr("Installed"),
                    state.installed.as_deref().unwrap_or("—")
                ))
                .small()
                .color(muted()),
            );
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    tr("Latest"),
                    state.latest.as_deref().unwrap_or("—")
                ))
                .small()
                .color(muted()),
            );
            ui.label(RichText::new(profile.platforms).small().color(muted()));
            ui.hyperlink_to(tr("Website"), profile.website);
            ui.hyperlink_to(tr("Source"), format!("{REPO}/{}", release_slug(id)));
            ui.hyperlink_to(
                tr("Release notes"),
                format!("{REPO}/{}/releases", release_slug(id)),
            );
        });
        if let Some(message) = state.busy.as_ref().or(state.error.as_ref()) {
            ui.label(tr(message));
        }
        ui.add_space(16.0);
        let mut selected_tab = *self.workspace_tabs.get(id).unwrap_or(&0);
        ui.horizontal(|ui| {
            for (i, label) in ["Projects", "Assets", "Plugins"].iter().enumerate() {
                if tab(ui, selected_tab == i as u8, label).clicked() {
                    selected_tab = i as u8;
                }
            }
        });
        self.workspace_tabs.insert(id.into(), selected_tab);
        ui.separator();
        ui.add_space(8.0);
        match selected_tab {
            1 => {
                scroll(ui, ("v2-assets", id), |ui| {
                    ui.label(tr("Asset organization is not available yet. You can open this app’s asset folder."));
                    if ui.button(tr("Open assets folder")).clicked() {
                        if let Some(root) = &self.prefs.default_project_root {
                            match workspace_bridge::prepare(root, &app) {
                                Ok(base) => reveal_project_path(&base.join("Assets"), false),
                                Err(e) => self.toast = Some(e),
                            }
                        } else {
                            self.settings_open = true;
                            self.settings_tab = 2;
                        }
                    }
                });
            }
            2 => {
                scroll(ui, ("v2-plugins", id), |ui| self.v2_plugins(ui, app));
            }
            _ => self.v2_projects(ui, Some(app)),
        }
    }

    fn v2_projects(&mut self, ui: &mut egui::Ui, app: Option<AppInfo>) {
        if app.is_none() {
            heading(ui, "Projects", "Files from your connected project folders.");
        }
        if app.is_none() {
            ui.horizontal(|ui| {
                if tab(ui, !self.projects_tab, "Library").clicked() {
                    self.projects_tab = false;
                }
                if tab(ui, self.projects_tab, "Folders").clicked() {
                    self.projects_tab = true;
                }
            });
        }
        if app.is_none() && self.projects_tab {
            ui.add_space(14.0);
            scroll(ui, "v2-folders", |ui| self.v2_folders(ui));
            return;
        }
        ui.horizontal_wrapped(|ui| {
            if app.is_none() {
                egui::ComboBox::from_id_salt("v2-project-app")
                    .selected_text(tr(&self.project_filter))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.project_filter,
                            "All apps".into(),
                            tr("All apps"),
                        );
                        for app in APPS {
                            ui.selectable_value(
                                &mut self.project_filter,
                                app.name.into(),
                                app.name,
                            );
                        }
                    });
            }
            egui::ComboBox::from_id_salt("v2-project-scope")
                .selected_text(tr(&self.project_scope))
                .show_ui(ui, |ui| {
                    for label in ["All projects", "Favorites", "Last 7 days"] {
                        ui.selectable_value(&mut self.project_scope, label.into(), tr(label));
                    }
                });
            egui::ComboBox::from_id_salt("v2-project-sort")
                .selected_text(tr(&self.prefs.project_sort))
                .show_ui(ui, |ui| {
                    for label in ["Recently modified", "Name A-Z", "Largest first", "By app"] {
                        if ui
                            .selectable_value(&mut self.prefs.project_sort, label.into(), tr(label))
                            .changed()
                        {
                            save_preferences(&self.prefs);
                        }
                    }
                });
            let view = if app.is_some() {
                &mut self.prefs.workspace_project_view
            } else {
                &mut self.prefs.project_view
            };
            if view_options(ui, view) {
                save_preferences(&self.prefs);
            }
            if ui
                .add_enabled(!self.prefs.scanning, egui::Button::new(tr("Refresh")))
                .clicked()
            {
                self.scan_projects();
            }
            if ui.add(primary_button("Add folder")).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.add_project_folder(path);
                }
            }
        });
        ui.add_space(12.0);
        ui.separator();
        let query = app
            .and_then(|a| self.workspace_search.get(a.id))
            .unwrap_or(&self.search)
            .trim()
            .to_lowercase();
        let cutoff = Local::now() - chrono::Duration::days(7);
        let mut projects: Vec<_> = self
            .projects
            .iter()
            .filter(|p| {
                app.map_or(
                    self.project_filter == "All apps" || self.project_filter == p.app.name,
                    |a| p.app.id == a.id,
                ) && (p.path.to_string_lossy().to_lowercase().contains(&query)
                    || p.app.name.to_lowercase().contains(&query))
                    && match self.project_scope.as_str() {
                        "Favorites" => self.prefs.favorite_projects.contains(&p.path),
                        "Last 7 days" => p.modified.is_some_and(|d| d >= cutoff),
                        _ => true,
                    }
            })
            .cloned()
            .collect();
        match self.prefs.project_sort.as_str() {
            "Name A-Z" => projects.sort_by_key(|p| p.title.to_lowercase()),
            "Largest first" => projects.sort_by_key(|p| std::cmp::Reverse(p.size_bytes)),
            "By app" => projects.sort_by_key(|p| (p.app.name, p.title.to_lowercase())),
            _ => projects.sort_by_key(|p| std::cmp::Reverse(p.modified)),
        }
        let view = if app.is_some() {
            self.prefs.workspace_project_view
        } else {
            self.prefs.project_view
        };
        scroll(ui, ("v2-project-results", app.map(|a| a.id)), |ui| {
            if view == ProjectView::List {
                self.v2_project_list(ui, &projects);
            } else {
                self.v2_project_grid(ui, &projects, view == ProjectView::Waterfall);
            }
        });
    }

    fn v2_project_list(&mut self, ui: &mut egui::Ui, projects: &[Project]) {
        if projects.is_empty() {
            ui.add_space(18.0);
            ui.label(tr("No projects to show."));
            ui.label(
                RichText::new(tr("Add a project folder or change the current filters."))
                    .color(muted()),
            );
            return;
        }
        let width = ui.available_width();
        let columns = [(width - 404.0).max(160.0), 115.0, 125.0, 80.0, 28.0];
        let cell = |ui: &mut egui::Ui, width: f32, label: egui::Label, right: bool| {
            let layout = if right {
                egui::Layout::right_to_left(egui::Align::Center)
            } else {
                egui::Layout::left_to_right(egui::Align::Center)
            };
            ui.allocate_ui_with_layout(Vec2::new(width, 34.0), layout, |ui| {
                ui.set_min_size(Vec2::new(width, 34.0));
                ui.add(label.truncate())
            })
            .inner
        };
        egui::Grid::new(ui.id().with("file-table"))
            .striped(true)
            .spacing(Vec2::new(14.0, 2.0))
            .min_row_height(34.0)
            .show(ui, |ui| {
                for (index, label) in ["Name", "Application", "Modified", "Size", ""]
                    .iter()
                    .enumerate()
                {
                    cell(
                        ui,
                        columns[index],
                        egui::Label::new(RichText::new(tr(*label)).small().strong().color(muted())),
                        index == 3,
                    );
                }
                ui.end_row();
                for p in projects {
                    let response = cell(
                        ui,
                        columns[0],
                        egui::Label::new(&p.title).sense(egui::Sense::click()),
                        false,
                    )
                    .on_hover_text(p.path.display().to_string())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if response.double_clicked() {
                        self.launch(*p.app, Some(&p.path));
                    }
                    self.project_context_menu(&response, p);
                    cell(ui, columns[1], egui::Label::new(p.app.name), false);
                    cell(
                        ui,
                        columns[2],
                        egui::Label::new(
                            p.modified
                                .map(|d| d.format("%d %b %Y").to_string())
                                .unwrap_or_else(|| "—".into()),
                        ),
                        false,
                    );
                    cell(
                        ui,
                        columns[3],
                        egui::Label::new(format_file_size(p.size_bytes)),
                        true,
                    );
                    more_menu(ui, |ui| self.project_menu_items(ui, p));
                    ui.end_row();
                }
            });
    }

    fn v2_project_grid(&mut self, ui: &mut egui::Ui, projects: &[Project], waterfall: bool) {
        if projects.is_empty() {
            self.v2_project_list(ui, projects);
            return;
        }
        let tile_width = ui.available_width().min(266.0);
        let gap = 16.0;
        let count = (((ui.available_width() + gap) / (tile_width + gap)).floor() as usize)
            .max(1)
            .min(projects.len());
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.set_width(count as f32 * tile_width + (count - 1) as f32 * gap);
            ui.columns(count, |columns| {
                for (i, p) in projects.iter().enumerate() {
                    let ui = &mut columns[i % count];
                    let width = ui.available_width();
                    let height = if waterfall {
                        p.preview
                            .as_ref()
                            .map(|im| width * im.height() as f32 / im.width().max(1) as f32)
                            .unwrap_or(150.0)
                            .clamp(100.0, 300.0)
                    } else {
                        width * 9.0 / 16.0
                    };
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
                    ui.painter().rect_filled(rect, UI_RADIUS, ink());
                    ui.painter().rect_stroke(
                        rect,
                        UI_RADIUS,
                        egui::Stroke::new(
                            1.0_f32,
                            if response.hovered() {
                                accent()
                            } else {
                                border()
                            },
                        ),
                        egui::StrokeKind::Inside,
                    );
                    if let Some(preview) = &p.preview {
                        let texture =
                            self.project_previews
                                .entry(p.path.clone())
                                .or_insert_with(|| {
                                    ui.ctx().load_texture(
                                        format!("v2-preview-{}", p.path.display()),
                                        egui::ColorImage::from_rgba_unmultiplied(
                                            [preview.width() as usize, preview.height() as usize],
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
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    } else {
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            p.path
                                .extension()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_uppercase(),
                            egui::FontId::proportional(18.0),
                            muted(),
                        );
                    }
                    if response.double_clicked() {
                        self.launch(*p.app, Some(&p.path));
                    }
                    self.project_context_menu(&response, p);
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [(width - 38.0).max(60.0), 28.0],
                            egui::Label::new(&p.title).truncate(),
                        );
                        more_menu(ui, |ui| self.project_menu_items(ui, p));
                    });
                    ui.label(
                        RichText::new(format!(
                            "{} · {}",
                            p.app.name,
                            format_file_size(p.size_bytes)
                        ))
                        .small()
                        .color(muted()),
                    );
                    ui.add_space(20.0);
                }
            });
        });
    }

    fn v2_folders(&mut self, ui: &mut egui::Ui) {
        if ui.button(tr("Add folder")).clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                self.add_project_folder(path);
            }
        }
        ui.add_space(10.0);
        let roots = self.prefs.roots.clone();
        let mut remove = None;
        for (index, root) in roots.iter().enumerate() {
            ui.push_id(root, |ui| {
                ui.label(RichText::new(root.display().to_string()).strong());
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr("Open")).clicked() {
                        reveal_project_path(root, false);
                    }
                    if ui.button(tr("Move folder")).clicked() {
                        self.choose_folder_move(root.clone());
                    }
                    if self.prefs.default_project_root.as_ref() == Some(root) {
                        ui.label(RichText::new(tr("Default save folder")).color(muted()));
                    } else if ui.button(tr("Use as default")).clicked() {
                        self.prefs.default_project_root = Some(root.clone());
                        save_preferences(&self.prefs);
                    }
                    if ui.button(tr("Stop watching")).clicked() {
                        remove = Some(index);
                    }
                });
                ui.separator();
                ui.add_space(8.0);
            });
        }
        if let Some(index) = remove {
            self.remove_project_folder(index);
        }
        if roots.is_empty() {
            ui.label(tr("No project folders connected."));
        }
    }

    pub(crate) fn v2_settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let available = ctx.screen_rect().size() - Vec2::splat(32.0);
        let size = Vec2::new(available.x.min(1040.0), available.y.min(600.0));
        let modal = egui::Modal::new(egui::Id::new("v2-settings-dialog"))
            .area(
                egui::Modal::default_area(egui::Id::new("v2-settings-dialog"))
                    .fade_in(!self.prefs.reduce_motion),
            )
            .frame(
                egui::Frame::new()
                    .fill(panel())
                    .corner_radius(UI_RADIUS)
                    .inner_margin(0)
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 8],
                        blur: 28,
                        spread: 0,
                        color: Color32::from_black_alpha(65),
                    }),
            )
            .show(ctx, |ui| self.v2_settings(ui, size));
        if modal.should_close() {
            self.settings_open = false;
        }
    }

    fn v2_settings(&mut self, ui: &mut egui::Ui, size: Vec2) {
        let before = serde_json::to_string(&self.prefs).unwrap_or_default();
        let previous_startup = self.prefs.start_with_windows;
        let (bounds, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let header_bottom = bounds.top() + 52.0;
        let header =
            egui::Rect::from_min_max(bounds.min, egui::pos2(bounds.right(), header_bottom));
        let footer_top = (bounds.bottom() - 56.0).max(bounds.top() + 40.0);
        let divider_x = bounds.left() + if bounds.width() < 650.0 { 140.0 } else { 176.0 };
        let rail = egui::Rect::from_min_max(
            egui::pos2(bounds.left(), header_bottom),
            egui::pos2(divider_x, footer_top),
        );
        let detail = egui::Rect::from_min_max(
            egui::pos2(divider_x, header_bottom),
            egui::pos2(bounds.right(), footer_top),
        );
        let footer = egui::Rect::from_min_max(egui::pos2(bounds.left(), footer_top), bounds.max);
        ui.painter().rect_filled(bounds, UI_RADIUS, panel());
        ui.painter().rect_filled(rail, 0.0, neutral(33, 245));
        ui.painter().line_segment(
            [header.left_bottom(), header.right_bottom()],
            egui::Stroke::new(1.0_f32, border()),
        );
        ui.painter().line_segment(
            [rail.right_top(), rail.right_bottom()],
            egui::Stroke::new(1.0_f32, border()),
        );
        ui.painter().line_segment(
            [footer.left_top(), footer.right_top()],
            egui::Stroke::new(1.0_f32, border()),
        );
        ui.painter().rect_stroke(
            bounds,
            UI_RADIUS,
            egui::Stroke::new(1.0_f32, border()),
            egui::StrokeKind::Inside,
        );

        let mut header_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("v2-preferences-header")
                .max_rect(header.shrink2(Vec2::new(20.0, 9.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        header_ui.label(RichText::new(tr("Settings")).size(text_size(16.0)).strong());
        header_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, ButtonIcon::Close, "Close settings", muted()).clicked() {
                self.settings_open = false;
            }
        });

        let mut navigation = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("v2-preferences-navigation")
                .max_rect(rail.shrink2(Vec2::new(12.0, 16.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        navigation.set_clip_rect(rail.intersect(ui.clip_rect()));
        egui::ScrollArea::vertical()
            .id_salt("v2-preferences-categories")
            .auto_shrink([false, false])
            .show_scoped(&mut navigation, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for (index, label) in [
                    (3, "General"),
                    (1, "Apps"),
                    (6, "Language"),
                    (7, "Notifications"),
                    (8, "Cloud"),
                    (0, "Appearance"),
                    (5, "Themes"),
                    (2, "Projects"),
                    (4, "About"),
                ] {
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 36.0),
                        egui::Sense::click(),
                    );
                    let selected = self.settings_tab == index;
                    if selected || response.hovered() {
                        ui.painter().rect_filled(rect, UI_RADIUS, neutral(48, 233));
                    }
                    if response.has_focus() {
                        ui.painter().rect_stroke(
                            rect,
                            UI_RADIUS,
                            egui::Stroke::new(1.0_f32, accent()),
                            egui::StrokeKind::Inside,
                        );
                    }
                    ui.painter().text(
                        rect.left_center() + Vec2::new(12.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        tr(label),
                        egui::FontId::proportional(text_size(13.0)),
                        if selected { foreground() } else { muted() },
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::SelectableLabel,
                            true,
                            selected,
                            tr(label),
                        )
                    });
                    if response
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        self.settings_tab = index;
                    }
                }
            });

        let mut content = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("v2-preferences-content")
                .max_rect(detail.shrink2(Vec2::new(1.0, 1.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        content.set_clip_rect(detail.intersect(ui.clip_rect()));
        let progress = transition_progress(
            ui.ctx(),
            egui::Id::new("v2-settings-transition"),
            egui::Id::new(self.settings_tab),
            self.prefs.reduce_motion,
        );
        content.multiply_opacity(0.35 + 0.65 * progress);
        content.style_mut().text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::proportional(text_size(14.0)),
        );
        egui::ScrollArea::vertical().id_salt(("v2-preferences-sections", self.settings_tab)).auto_shrink([false, false]).show_scoped(&mut content, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            match self.settings_tab {
                0 => {
                    preferences_section(ui, "Color theme", "Choose the appearance of your workspace.", |ui| {
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.prefs.light_mode, false, tr("Dark"));
                            ui.radio_value(&mut self.prefs.light_mode, true, tr("Light"));
                        });
                    });
                    preferences_section(ui, "Text size", "Adjust text throughout Master Suite.", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (index, label) in ["Small", "Medium", "Large"].iter().enumerate() {
                                if ui.radio_value(&mut self.prefs.text_scale, index as u8, tr(*label)).changed() {
                                    windows_ui::set_text_scale(self.prefs.text_scale);
                                    apply_theme(ui.ctx(), self.prefs.light_mode);
                                }
                            }
                        });
                    });
                    preferences_section(ui, "Motion", "Keep transitions and startup artwork still.", |ui| {
                        preferences_toggle(ui, &mut self.prefs.reduce_motion, "Reduce motion");
                    });
                }
                1 => {
                    preferences_section(ui, "App update checks", "Check your installed apps for new releases automatically.", |ui| {
                        preferences_toggle(ui, &mut self.prefs.automatic_updates, "Check for updates automatically");
                        ui.add_space(10.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(tr("Check every")).color(muted()));
                            egui::ComboBox::from_id_salt("v2-update-interval").selected_text(tr(format!("{} hours", self.prefs.update_interval_hours))).show_ui(ui, |ui| {
                                for value in [1, 2, 4, 8, 12, 24] { ui.selectable_value(&mut self.prefs.update_interval_hours, value, tr(format!("{value} hours"))); }
                            });
                        });
                    });
                    preferences_section(ui, "Master Suite updates", "Keep Master Suite up to date automatically.", |ui| {
                        preferences_toggle(ui, &mut self.prefs.automatic_suite_updates, "Auto-update Master Suite");
                        let beta = self.prefs.beta_suite_updates;
                        ui.add_enabled_ui(!platform::flatpak() && !self.suite_update_busy, |ui| {
                            preferences_toggle(ui, &mut self.prefs.beta_suite_updates, "Include prerelease updates");
                        });
                        if beta != self.prefs.beta_suite_updates { self.suite_update_ready = None; self.suite_check_started = None; }
                        ui.add_space(10.0);
                        line(ui, "Installed version", VERSION);
                        ui.label(RichText::new(tr(&self.suite_update_status)).small().color(muted()));
                        ui.horizontal_wrapped(|ui| {
                            if ui.add_enabled(!self.suite_update_busy, outline_button("Check for updates")).clicked() { self.check_suite_update(true); }
                            ui.hyperlink_to(tr("Release history"), "https://github.com/ZifuM/Craft-apps-launcher-installer/releases");
                            if self.suite_update_ready.is_some() && !self.prefs.automatic_suite_updates && ui.add(primary_button("Install and restart")).clicked() {
                                if let Some((update, _)) = self.suite_update_ready.take() {
                                    save_preferences(&self.prefs);
                                    match suite_update::launch(&update) { Ok(()) => self.tray_quit_requested = true, Err(error) => self.suite_update_status = error }
                                }
                            }
                        });
                    });
                }
                2 => {
                    preferences_section(ui, "Project folders", "Choose where your projects are stored. Move a folder or set the default save location below.", |ui| self.v2_folders(ui));
                    preferences_section(ui, "Project refresh", "Keep the library in sync with changes to your files.", |ui| {
                        preferences_toggle(ui, &mut self.prefs.automatic_project_scan, "Refresh projects automatically");
                        ui.add_space(10.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(tr("Refresh every")).color(muted()));
                            egui::ComboBox::from_id_salt("v2-scan-interval").selected_text(tr(format!("{} minutes", self.prefs.project_scan_minutes))).show_ui(ui, |ui| {
                                for value in [1, 3, 5, 10, 15, 30] { ui.selectable_value(&mut self.prefs.project_scan_minutes, value, tr(format!("{value} minutes"))); }
                            });
                        });
                    });
                    preferences_section(ui, "Default layout", "Choose how projects appear in the library.", |ui| { view_options(ui, &mut self.prefs.project_view); });
                }
                3 => {
                    preferences_section(ui, "Startup", "Choose how Master Suite starts and stays available.", |ui| {
                        preferences_toggle(ui, &mut self.prefs.start_with_windows, "Start when I sign in");
                        preferences_toggle(ui, &mut self.prefs.minimize_to_tray, "Minimize to the system tray");
                    });
                    preferences_section(ui, "Window", "Your window size, position and maximized state are saved automatically.", |_| {});
                }
                5 => preferences_section(ui, "", "", |ui| self.theme_picker(ui)),
                6 => preferences_section(ui, "App language", "Choose the language used by Master Suite.", |ui| {
                    egui::ComboBox::from_id_salt("v2-language").width(220.0).selected_text(localization::name(&self.prefs.language)).show_ui(ui, |ui| {
                        for (code, name) in localization::LANGUAGES { ui.selectable_value(&mut self.prefs.language, code.to_owned(), name); }
                    });
                }),
                7 => preferences_section(ui, "App updates", "Show a notification when updates are available for your apps.", |ui| {
                    preferences_toggle(ui, &mut self.prefs.update_notifications, "Show update notifications");
                }),
                8 => self.v2_cloud_settings(ui),
                _ => {
                    preferences_section(ui, "ArtCraft Master Suite", "", |ui| {
                        line(ui, "Version", VERSION); line(ui, "Platform", &platform::label());
                        ui.hyperlink_to(tr("Source repository"), "https://github.com/ZifuM/Craft-apps-launcher-installer");
                    });
                    preferences_section(ui, "App credits", "Created by Storytold and the app contributors.", |ui| {
                        egui::Grid::new("v2-credits").striped(true).min_col_width(120.0).show(ui, |ui| {
                            for app in APPS { ui.label(app.name); ui.hyperlink_to(format!("storytold/{}", release_slug(app.id)), format!("{REPO}/{}", release_slug(app.id))); ui.end_row(); }
                        });
                    });
                }
            }
        });
        let mut footer_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("v2-preferences-footer")
                .max_rect(footer.shrink2(Vec2::new(24.0, 12.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        if footer_ui.add(primary_button("Done")).clicked() {
            self.settings_open = false;
        }

        if before != serde_json::to_string(&self.prefs).unwrap_or_default() {
            if previous_startup != self.prefs.start_with_windows {
                #[cfg(windows)]
                let result = windows_tray::set_startup(self.prefs.start_with_windows);
                #[cfg(target_os = "linux")]
                let result = linux_tray::set_startup(self.prefs.start_with_windows);
                #[cfg(target_os = "macos")]
                let result = macos::set_startup(self.prefs.start_with_windows);
                if let Err(error) = result {
                    self.prefs.start_with_windows = previous_startup;
                    self.toast = Some(error);
                }
            }
            self.tray_failed = false;
            localization::select(&self.prefs.language);
            save_preferences(&self.prefs);
            if !self.prefs.update_notifications && self.persistent_toast.is_some() {
                self.toast = None;
                self.persistent_toast = None;
            }
        }
    }

    fn v2_cloud_settings(&mut self, ui: &mut egui::Ui) {
        preferences_section(
            ui,
            "Cloud backup",
            "Keep versioned project copies in your own sync folders. Cloud features are experimental.",
            |ui| {
                let automatic = self.cloud.settings.automatic;
                ui.add_enabled_ui(!self.cloud.busy, |ui| {
                    preferences_toggle(
                        ui,
                        &mut self.cloud.settings.automatic,
                        "Back up selected projects automatically",
                    );
                });
                if automatic != self.cloud.settings.automatic {
                    self.cloud.save();
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(tr(format!(
                            "{} connected sync folders",
                            self.cloud.settings.folders.len()
                        )))
                        .color(muted()),
                    );
                    if ui.add(outline_button("Manage sync folders")).clicked() {
                        self.cloud.tab = 3;
                    }
                });
            },
        );
        preferences_section(ui, "", "", |ui| self.v2_cloud_controls(ui));
    }

    fn v2_cloud_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (i, label) in [
                (0, "Project files"),
                (1, "Saved versions"),
                (3, "Sync folders"),
                (2, "Assets"),
            ] {
                if tab(ui, self.cloud.tab == i, label).clicked() {
                    self.cloud.tab = i;
                }
            }
        });
        ui.separator();
        ui.add_space(12.0);
        if self.cloud.busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr(&self.cloud.message));
                if ui.button(tr("Cancel")).clicked() {
                    self.cloud.cancel();
                }
            });
        } else if !self.cloud.message.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(tr(&self.cloud.message))
                        .small()
                        .color(muted()),
                );
                if ui.small_button(tr("Dismiss")).clicked() {
                    self.cloud.message.clear();
                }
            });
        }
        let progress = transition_progress(
            ui.ctx(),
            egui::Id::new("v2-cloud-transition"),
            egui::Id::new(self.cloud.tab),
            self.prefs.reduce_motion,
        );
        ui.scope(|ui| {
        ui.multiply_opacity(0.35 + 0.65 * progress);
        match self.cloud.tab {
            3 => {
                for provider in cloud::PROVIDERS {
                    ui.push_id(provider.name(), |ui| {
                        ui.label(RichText::new(provider.name()).strong());
                        if let Some(folder) = self.cloud.settings.folders.get(&provider).cloned() {
                            ui.add(egui::Label::new(folder.path.display().to_string()).wrap());
                            let mut selected = self.cloud.settings.targets.contains(&provider);
                            if ui
                                .add_enabled(
                                    !self.cloud.busy,
                                    |ui: &mut egui::Ui| preferences_toggle(ui, &mut selected, "Use for backups"),
                                )
                                .changed()
                            {
                                if selected {
                                    self.cloud.settings.targets.push(provider);
                                } else {
                                    self.cloud.settings.targets.retain(|p| *p != provider);
                                }
                                self.cloud.save();
                            }
                            ui.horizontal_wrapped(|ui| {
                                if ui.button(tr("Open folder")).clicked() {
                                    self.cloud.open_folder(provider);
                                }
                                if ui
                                    .add_enabled(
                                        !self.cloud.busy,
                                        egui::Button::new(tr("Change folder")),
                                    )
                                    .clicked()
                                {
                                    self.cloud.connect(provider);
                                }
                                if ui
                                    .add_enabled(
                                        !self.cloud.busy,
                                        egui::Button::new(tr("Disconnect")),
                                    )
                                    .clicked()
                                {
                                    self.cloud.disconnect_provider = Some(provider);
                                }
                            });
                        } else {
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .add_enabled(
                                        !self.cloud.busy,
                                        egui::Button::new(tr("Choose sync folder")),
                                    )
                                    .clicked()
                                {
                                    self.cloud.connect(provider);
                                }
                                if ui.button(tr("Open desktop app")).clicked() {
                                    if let Err(error) = provider.open_desktop() {
                                        self.cloud.message = error;
                                    }
                                }
                                ui.hyperlink_to(tr("Download desktop app"), provider.website());
                            });
                        }
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);
                    });
                }
                ui.label(RichText::new(tr("Your sync provider handles uploading. Master Suite stores versioned copies in the folders you select.")).color(muted()));
            }
            1 => {
                if ui
                    .add_enabled(
                        !self.cloud.busy && !self.cloud.settings.folders.is_empty(),
                        egui::Button::new(tr("Refresh saved versions")),
                    )
                    .clicked()
                {
                    self.cloud.refresh();
                }
                ui.label(
                    RichText::new(tr("Restore an earlier version as a separate file."))
                        .color(muted()),
                );
                ui.add_space(12.0);
                let mut history = self.cloud.history.clone();
                history.sort_by(|a, b| b.modified.cmp(&a.modified));
                if history.is_empty() {
                    ui.label(tr("No saved versions to show."));
                }
                for remote in history {
                    ui.push_id((&remote.id, remote.provider), |ui| {
                        ui.horizontal(|ui| {
                            let title_width = (ui.available_width() - 126.0).max(80.0);
                            ui.allocate_ui_with_layout(Vec2::new(title_width, 32.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.set_min_width(title_width);
                                ui.add(egui::Label::new(RichText::new(&remote.name).strong()).truncate());
                            });
                            if ui
                                .add_enabled(
                                    !self.cloud.busy,
                                    egui::Button::new(tr("Restore copy")),
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
                        });
                        ui.label(
                            RichText::new(format!(
                                "{} · {} · {}",
                                remote.provider.name(),
                                remote.modified,
                                format_file_size(remote.bytes)
                            ))
                            .small()
                            .color(muted()),
                        );
                        ui.separator();
                    });
                }
            }
            2 => {
                ui.label(tr("Asset backups are not available yet."));
            }
            _ => {
                ui.horizontal_wrapped(|ui| {
                    search(
                        ui,
                        "v2-backup-search",
                        &mut self.cloud.search,
                        "Search files",
                        240.0,
                    );
                    if ui
                        .add_enabled(
                            !self.cloud.busy
                                && !self.cloud.settings.selected.is_empty()
                                && !self.cloud.settings.targets.is_empty(),
                            primary_button("Back up now"),
                        )
                        .clicked()
                    {
                        self.cloud.sync(&self.projects);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    for (i, label) in ["All files", "Selected", "Needs attention"]
                        .iter()
                        .enumerate()
                    {
                        ui.selectable_value(&mut self.cloud.view_filter, i as u8, tr(*label));
                    }
                });
                let query = self.cloud.search.trim().to_lowercase();
                let projects: Vec<_> = self
                    .projects
                    .iter()
                    .filter(|p| p.path.to_string_lossy().to_lowercase().contains(&query))
                    .filter(|p| match self.cloud.view_filter {
                        1 => self.cloud.settings.selected.contains(&p.path),
                        2 => matches!(
                            self.cloud.status(p),
                            "Needs attention" | "Pending copy" | "Not connected"
                        ),
                        _ => true,
                    })
                    .cloned()
                    .collect();
                let mut all = !projects.is_empty()
                    && projects
                        .iter()
                        .all(|p| self.cloud.settings.selected.contains(&p.path));
                if ui
                    .add_enabled(
                        !self.cloud.busy,
                        egui::Checkbox::new(&mut all, tr("Select all visible files")),
                    )
                    .changed()
                {
                    for p in &projects {
                        self.cloud.settings.selected.retain(|path| path != &p.path);
                        if all {
                            self.cloud.settings.selected.push(p.path.clone());
                        }
                    }
                    self.cloud.save();
                }
                ui.separator();
                if projects.is_empty() {
                    ui.label(tr("No project files to show."));
                }
                for p in projects {
                    ui.push_id(&p.path, |ui| {
                        let mut selected = self.cloud.settings.selected.contains(&p.path);
                        ui.horizontal(|ui| {
                            let content_width = (ui.available_width() - 44.0).max(120.0);
                            ui.allocate_ui_with_layout(Vec2::new(content_width, 54.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.set_width(content_width);
                            ui.spacing_mut().item_spacing.y = 4.0;
                            if ui
                                .add_enabled(
                                    !self.cloud.busy,
                                    egui::Checkbox::new(&mut selected, &p.title),
                                )
                                .changed()
                            {
                                self.cloud.settings.selected.retain(|path| path != &p.path);
                                if selected {
                                    self.cloud.settings.selected.push(p.path.clone());
                                }
                                self.cloud.save();
                            }
                            let summary = self.cloud.backup_status(&p);
                            ui.add(egui::Label::new(RichText::new(format!(
                                "{} · {} · {}", p.app.name, format_file_size(p.size_bytes), tr(summary.state.label())
                            )).small().color(muted())).truncate())
                            .on_hover_text(summary.tooltip);
                            });
                            more_menu(ui, |ui| self.project_menu_items(ui, &p));
                        });
                        ui.separator();
                    });
                }
            }
        }
        });
        if let Some(provider) = self.cloud.disconnect_provider {
            let mut confirm = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("v2-disconnect")).show(ui.ctx(), |ui| {
                ui.set_width(420.0);
                ui.heading(tr(format!("Disconnect {}?", provider.name())));
                ui.label(tr("Existing backup files will be kept."));
                ui.horizontal(|ui| {
                    confirm = ui.button(tr("Disconnect")).clicked();
                    cancel = ui.button(tr("Cancel")).clicked();
                });
            });
            if confirm {
                self.cloud.disconnect(provider);
                self.cloud.disconnect_provider = None;
            } else if cancel || modal.should_close() {
                self.cloud.disconnect_provider = None;
            }
        }
    }

    fn v2_plugins(&mut self, ui: &mut egui::Ui, app: AppInfo) {
        ui.label(tr(
            "Plugin management is experimental. Close the app before changing plugins.",
        ));
        let Some(root) = self.prefs.default_project_root.clone() else {
            ui.label(tr("Choose a default project folder in Settings first."));
            return;
        };
        ui.horizontal_wrapped(|ui| {
            for folder in ["Projects", "Exports", "Assets", "Plugins"] {
                if ui.button(tr(folder)).clicked() {
                    match workspace_bridge::prepare(&root, &app) {
                        Ok(base) => reveal_project_path(&base.join(folder), false),
                        Err(e) => self.plugins.message = e,
                    }
                }
            }
        });
        if !plugins::supported(app.id) {
            ui.add_space(16.0);
            ui.label(tr("This app does not support plugin installation yet."));
            return;
        }
        section(ui, "Install a plugin");
        search(
            ui,
            "v2-plugin-source",
            &mut self.plugins.link,
            "GitHub repository or release URL",
            ui.available_width().min(650.0),
        );
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.plugins.busy && !self.plugins.link.trim().is_empty(),
                    egui::Button::new(tr("Install from GitHub")),
                )
                .clicked()
            {
                self.plugins
                    .install(app, root.clone(), self.plugins.link.trim().into(), false);
            }
            if ui
                .add_enabled(
                    !self.plugins.busy,
                    egui::Button::new(tr("Install from file")),
                )
                .clicked()
            {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("WebAssembly plugin", &["wasm", "zip"])
                    .pick_file()
                {
                    self.plugins
                        .install(app, root.clone(), file.display().to_string(), true);
                }
            }
        });
        ui.label(
            RichText::new(tr("Compiled .wasm plugins and ZIP packages are supported."))
                .small()
                .color(muted()),
        );
        if !self.plugins.message.is_empty() {
            ui.label(tr(&self.plugins.message));
        }
        section(ui, "Installed plugins");
        match plugins::entries(&app) {
            Ok(entries) => {
                if entries.is_empty() {
                    ui.label(tr("No installed plugins."));
                }
                for (index, entry) in entries.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.horizontal(|ui| {
                            let mut enabled = entry.enabled;
                            if ui
                                .add_enabled(
                                    !self.plugins.busy,
                                    egui::Checkbox::new(&mut enabled, &entry.name),
                                )
                                .changed()
                            {
                                if let Err(error) = plugins::toggle(&app, index) {
                                    self.plugins.message = error;
                                }
                            }
                            if ui
                                .add_enabled(!self.plugins.busy, egui::Button::new(tr("Remove")))
                                .clicked()
                            {
                                self.plugins.remove = Some((app.id.into(), index));
                            }
                        });
                        ui.separator();
                    });
                }
            }
            Err(error) => {
                ui.label(error);
            }
        }
        if let Some((id, index)) = self.plugins.remove.clone() {
            if id == app.id {
                let mut remove = false;
                let mut cancel = false;
                let modal =
                    egui::Modal::new(egui::Id::new("v2-remove-plugin")).show(ui.ctx(), |ui| {
                        ui.set_width(380.0);
                        ui.heading(tr("Remove plugin?"));
                        ui.label(tr(
                            "Your projects and the original downloaded package will be kept.",
                        ));
                        ui.horizontal(|ui| {
                            remove = ui.button(tr("Remove plugin")).clicked();
                            cancel = ui.button(tr("Cancel")).clicked();
                        });
                    });
                if remove {
                    self.plugins.message = match plugins::uninstall(&app, index) {
                        Ok(()) => tr("Plugin removed."),
                        Err(e) => e,
                    };
                    self.plugins.remove = None;
                } else if cancel || modal.should_close() {
                    self.plugins.remove = None;
                }
            }
        }
    }

    pub(crate) fn v2_properties(&mut self, ctx: &egui::Context) {
        let Some(properties) = self.properties.take() else {
            return;
        };
        let app = properties.app;
        let state = self.states.get(app.id).cloned().unwrap_or_default();
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("v2-properties")).show(ctx, |ui| {
            ui.set_width((ctx.screen_rect().width() - 80.0).clamp(280.0, 560.0));
            ui.heading(tr("Properties"));
            ui.label(
                RichText::new(
                    properties
                        .project
                        .as_ref()
                        .map(|p| p.title.as_str())
                        .unwrap_or(app.name),
                )
                .strong(),
            );
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height((ctx.screen_rect().height() - 240.0).max(120.0))
                .show_scoped(ui, |ui| {
                    line(ui, "Application", app.name);
                    if properties.project.is_none() {
                        line(
                            ui,
                            "Installed version",
                            state.installed.as_deref().unwrap_or("Not installed"),
                        );
                        line(
                            ui,
                            "Latest version",
                            state.latest.as_deref().unwrap_or("Not checked"),
                        );
                        line(ui, "Description", &tr(app.blurb));
                    }
                    if let Some(file) = &properties.file {
                        line(ui, "Location", &file.display().to_string());
                    }
                    if let Some(directory) = &properties.directory {
                        line(ui, "Folder", &directory.display().to_string());
                    }
                    for (key, value) in &properties.metadata {
                        line(ui, key, value);
                    }
                    if properties.project.is_none() {
                        line(
                            ui,
                            "File formats",
                            &app.filetypes
                                .iter()
                                .map(|ext| format!(".{ext}"))
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
                    }
                });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                if let Some(file) = &properties.file {
                    if ui.button(tr(platform::reveal_label())).clicked() {
                        reveal_project_path(file, true);
                    }
                    if ui.button(tr("Copy path")).clicked() {
                        ctx.copy_text(file.display().to_string());
                    }
                }
                if properties.project.is_none()
                    && state.installed.is_some()
                    && state.latest.is_some()
                    && state.latest != state.installed
                {
                    if ui
                        .add_enabled(state.busy.is_none(), primary_button("Update"))
                        .clicked()
                    {
                        self.install(app);
                    }
                }
                close = ui.button(tr("Close")).clicked();
            });
        });
        if !close && !modal.should_close() {
            self.properties = Some(properties);
        }
    }
}

impl Launcher {
    pub(crate) fn v2_project_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.show_remove.clone() {
            let mut remove = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("v2-uninstall-app")).show(ctx, |ui| {
                ui.set_width(420.0);
                ui.heading(tr(format!(
                    "Uninstall {}?",
                    app_by_id(&id).map(|a| a.name).unwrap_or(&id)
                )));
                ui.label(tr("Your project files will be kept."));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    remove = ui.button(tr("Uninstall")).clicked();
                    cancel = ui.button(tr("Cancel")).clicked();
                });
            });
            if remove {
                self.remove(&id);
            } else if cancel || modal.should_close() {
                self.show_remove = None;
            }
            return;
        }
        if let Some(draft) = self.show_project_rename.as_mut() {
            let mut save = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("v2-rename-project")).show(ctx, |ui| {
                ui.set_width(420.0);
                ui.heading(tr("Rename project"));
                ui.label(
                    RichText::new(draft.path.display().to_string())
                        .small()
                        .color(muted()),
                );
                let input = ui
                    .add(egui::TextEdit::singleline(&mut draft.name).desired_width(f32::INFINITY));
                if !draft.focused {
                    input.request_focus();
                    draft.focused = true;
                }
                ui.label(
                    RichText::new(tr("The file extension stays the same."))
                        .small()
                        .color(muted()),
                );
                let valid = !draft.name.trim().is_empty()
                    && !draft
                        .name
                        .chars()
                        .any(|c| "<>:\"/\\|?*".contains(c) || c.is_control());
                ui.horizontal(|ui| {
                    save = ui.add_enabled(valid, primary_button("Rename")).clicked();
                    cancel = ui.button(tr("Cancel")).clicked();
                });
                save |=
                    valid && input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
            let (path, name) = (draft.path.clone(), draft.name.clone());
            if save {
                self.rename_project(&path, &name);
            } else if cancel || modal.should_close() {
                self.show_project_rename = None;
            }
            return;
        }
        if let Some(project) = self.show_project_delete.clone() {
            let mut remove = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("v2-delete-project")).show(ctx, |ui| {
                ui.set_width(420.0);
                ui.heading(tr("Delete project?"));
                ui.label(&project.title);
                ui.label(tr("This permanently removes the project file from disk."));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    remove = ui.button(tr("Delete project")).clicked();
                    cancel = ui.button(tr("Cancel")).clicked();
                });
            });
            if remove {
                self.delete_project(&project.path);
            } else if cancel || modal.should_close() {
                self.show_project_delete = None;
            }
        }
    }

    pub(crate) fn v2_update_dialog(&mut self, ctx: &egui::Context) {
        let Some(check) = self.manual_update_check.clone() else {
            return;
        };
        let Some(app) = app_by_id(&check.app_id).copied() else {
            return;
        };
        let mut close = false;
        let mut install = false;
        let mut retry = false;
        let modal = egui::Modal::new(egui::Id::new("v2-release-check")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(tr(format!("{} updates", app.name)));
            ui.add_space(12.0);
            match check.status {
                ManualUpdateStatus::Checking => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr("Checking the latest release…"));
                    });
                }
                ManualUpdateStatus::Current(version) => {
                    ui.label(tr(format!("Version {version} is up to date.")));
                }
                ManualUpdateStatus::Available(version) => {
                    ui.label(tr(format!("Version {version} is available.")));
                    install = ui
                        .add_enabled(
                            self.states.get(app.id).is_none_or(|s| s.busy.is_none()),
                            primary_button("Install"),
                        )
                        .clicked();
                }
                ManualUpdateStatus::Failed(error) => {
                    ui.label(tr(error));
                    retry = ui.button(tr("Retry")).clicked();
                }
            }
            ui.add_space(12.0);
            close = ui.button(tr("Close")).clicked();
        });
        if install {
            self.install(app);
            self.manual_update_check = None;
        } else if retry {
            self.check_app_release(app);
        } else if close || modal.should_close() {
            self.manual_update_check = None;
        }
    }

    pub(crate) fn v2_onboarding(&mut self, ctx: &egui::Context) {
        let mut finish = false;
        egui::Modal::new(egui::Id::new("v2-setup")).show(ctx, |ui| {
            ui.set_width(500.0_f32.min(ctx.screen_rect().width() - 80.0));
            ui.heading(tr("Set up Master Suite"));
            ui.label(tr(
                "Choose a project folder now, or add one later in Settings.",
            ));
            ui.add_space(18.0);
            if let Some(root) = &self.prefs.default_project_root {
                ui.label(root.display().to_string());
            }
            if ui.button(tr("Choose project folder")).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.add_project_folder(path);
                }
            }
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.label(tr("Language"));
                egui::ComboBox::from_id_salt("v2-setup-language")
                    .selected_text(localization::name(&self.prefs.language))
                    .show_ui(ui, |ui| {
                        for (code, name) in localization::LANGUAGES {
                            ui.selectable_value(&mut self.prefs.language, code.to_owned(), name);
                        }
                    });
            });
            ui.checkbox(&mut self.prefs.light_mode, tr("Use light colors"));
            ui.add_space(18.0);
            ui.label(
                RichText::new(tr(
                    "Cloud backup is optional and can be connected from Settings → Cloud → Sync folders.",
                ))
                .color(muted()),
            );
            finish = ui.button(tr("Continue")).clicked();
        });
        if finish {
            self.prefs.onboarding_complete = true;
            localization::select(&self.prefs.language);
            save_preferences(&self.prefs);
        }
    }

    pub(crate) fn v2_notification(&mut self, ctx: &egui::Context, message: &str) {
        egui::Area::new(egui::Id::new("v2-message"))
            .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-14.0, -14.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(panel())
                    .stroke(egui::Stroke::new(1.0_f32, border()))
                    .corner_radius(UI_RADIUS)
                    .inner_margin(12)
                    .show(ui, |ui| {
                        ui.set_max_width((ctx.screen_rect().width() - 60.0).min(560.0));
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr(message));
                            if ui.small_button(tr("Dismiss")).clicked() {
                                self.toast = None;
                                self.toast_last_message = None;
                                self.toast_started = None;
                                self.persistent_toast = None;
                            }
                        });
                    });
            });
    }
}
