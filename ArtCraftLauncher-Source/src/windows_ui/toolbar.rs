//! Shared desktop library chrome. Every control owns a bounded, aligned cell.
use super::*;

pub(crate) const HEIGHT: f32 = 40.0;
pub(crate) fn style(ui: &mut egui::Ui) {
    for (kind, size, strong) in [
        (egui::TextStyle::Body, 13.0, false),
        (egui::TextStyle::Button, 13.0, true),
        (egui::TextStyle::Small, 12.0, false),
        (egui::TextStyle::Heading, 18.0, true),
    ] {
        ui.style_mut().text_styles.insert(kind, font(size, strong));
    }
    ui.spacing_mut().button_padding = Vec2::new(14.0, 10.0);
    ui.spacing_mut().interact_size.y = HEIGHT;
    ui.style_mut().visuals.widgets.inactive.corner_radius = 6.into();
}
pub(crate) fn font(size: f32, strong: bool) -> egui::FontId {
    egui::FontId::new(
        text_size(size),
        egui::FontFamily::Name(if strong { "suite-semibold" } else { "suite-ui" }.into()),
    )
}

pub(crate) fn install_fonts(fonts: &mut egui::FontDefinitions) {
    for (name, strong) in [("suite-ui", false), ("suite-semibold", true)] {
        let mut fallback = fonts.families[&egui::FontFamily::Proportional].clone();
        let candidates: Vec<PathBuf> = if cfg!(target_os = "windows") {
            let root = std::env::var_os("WINDIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| "C:/Windows".into());
            vec![
                root.join("Fonts")
                    .join(if strong { "seguisb.ttf" } else { "segoeui.ttf" }),
            ]
        } else if cfg!(target_os = "macos") {
            vec![
                PathBuf::from("/System/Library/Fonts/Supplemental").join(if strong {
                    "Arial Bold.ttf"
                } else {
                    "Arial.ttf"
                }),
            ]
        } else {
            let noto = if strong {
                "NotoSans-Bold.ttf"
            } else {
                "NotoSans-Regular.ttf"
            };
            let dejavu = if strong {
                "DejaVuSans-Bold.ttf"
            } else {
                "DejaVuSans.ttf"
            };
            ["/usr/share/fonts", "/run/host/fonts"]
                .into_iter()
                .flat_map(|root| {
                    [
                        format!("{root}/truetype/noto/{noto}"),
                        format!("{root}/noto/{noto}"),
                        format!("{root}/truetype/dejavu/{dejavu}"),
                    ]
                    .map(PathBuf::from)
                })
                .collect()
        };
        if let Some(bytes) = candidates.iter().find_map(|path| fs::read(path).ok()) {
            fonts
                .font_data
                .insert(name.into(), egui::FontData::from_owned(bytes).into());
            fallback.insert(0, name.into());
        }
        fonts
            .families
            .insert(egui::FontFamily::Name(name.into()), fallback);
    }
}

pub(crate) fn cell(ui: &mut egui::Ui, rect: egui::Rect) -> egui::Ui {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child.spacing_mut().interact_size.y = HEIGHT;
    child.spacing_mut().button_padding = Vec2::new(12.0, 10.0);
    child.style_mut().override_font_id = Some(font(13.0, false));
    child
}

pub(crate) fn row(ui: &mut egui::Ui) -> egui::Rect {
    ui.allocate_exact_size(
        Vec2::new(ui.available_width(), HEIGHT),
        egui::Sense::hover(),
    )
    .0
}

#[derive(Clone, Copy)]
pub(crate) enum Glyph {
    Arrow,
    Plus,
    Refresh,
    External,
}
fn glyph(p: &egui::Painter, c: egui::Pos2, glyph: Glyph, color: Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
    match glyph {
        Glyph::Plus => {
            p.line_segment([c - Vec2::new(5.0, 0.0), c + Vec2::new(5.0, 0.0)], stroke);
            p.line_segment([c - Vec2::new(0.0, 5.0), c + Vec2::new(0.0, 5.0)], stroke);
        }
        Glyph::Arrow => {
            p.line_segment([c - Vec2::new(5.0, 0.0), c + Vec2::new(5.0, 0.0)], stroke);
            p.add(egui::Shape::line(
                vec![
                    c + Vec2::new(1.0, -4.0),
                    c + Vec2::new(5.0, 0.0),
                    c + Vec2::new(1.0, 4.0),
                ],
                stroke,
            ));
        }
        Glyph::Refresh => paint_refresh(p, c, color),
        Glyph::External => {
            p.add(egui::Shape::line(
                vec![
                    c + Vec2::new(-1.0, -5.0),
                    c + Vec2::new(-6.0, -5.0),
                    c + Vec2::new(-6.0, 6.0),
                    c + Vec2::new(5.0, 6.0),
                    c + Vec2::new(5.0, 1.0),
                ],
                stroke,
            ));
            p.line_segment([c + Vec2::new(-1.0, 1.0), c + Vec2::new(6.0, -6.0)], stroke);
            p.add(egui::Shape::line(
                vec![
                    c + Vec2::new(1.0, -6.0),
                    c + Vec2::new(6.0, -6.0),
                    c + Vec2::new(6.0, -1.0),
                ],
                stroke,
            ));
        }
    }
}

/// Open circular arrow with a distinct arrowhead; shared by every refresh control.
pub(crate) fn paint_refresh(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let stroke = egui::Stroke::new(1.7_f32, color);
    let points = (0..33)
        .map(|i| {
            let angle = 0.45 + i as f32 / 32.0 * 4.95;
            c + Vec2::angled(angle) * 6.5
        })
        .collect();
    p.add(egui::Shape::line(points, stroke));
    let tip = c + Vec2::angled(5.4) * 6.5;
    p.add(egui::Shape::line(
        vec![tip - Vec2::new(5.0, 0.0), tip, tip - Vec2::new(0.0, 5.0)],
        stroke,
    ));
}

pub(crate) fn button(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: &str,
    label: &str,
    icon: Glyph,
    primary: bool,
    enabled: bool,
) -> egui::Response {
    let mut child = cell(ui, rect);
    if !enabled {
        child.disable();
    }
    let fill = if primary { ACCENT } else { panel() };
    let color = if !enabled {
        muted()
    } else if primary {
        Color32::WHITE
    } else {
        foreground()
    };
    let response = child
        .push_id(id, |ui| {
            ui.add_sized(
                rect.size(),
                egui::Button::new("")
                    .fill(fill)
                    .stroke(egui::Stroke::new(1.0_f32, border()))
                    .corner_radius(6),
            )
        })
        .inner;
    // Center the icon and label as a single unit, including translated labels.
    let label_galley = ui
        .painter()
        .layout_no_wrap(tr(label), font(13.0, true), color);
    let label_width = label_galley.size().x.min((rect.width() - 44.0).max(1.0));
    let start = rect.center().x - (label_width + 24.0) * 0.5;
    glyph(
        ui.painter(),
        egui::pos2(start + 6.0, rect.center().y),
        icon,
        color,
    );
    let label_rect = egui::Rect::from_min_max(
        egui::pos2(start + 24.0, rect.top()),
        egui::pos2(rect.right() - 10.0, rect.bottom()),
    );
    text(ui, label_rect, label, font(13.0, true), color);
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tr(label)));
    response.on_hover_text(tr(label))
}

fn text(ui: &egui::Ui, rect: egui::Rect, value: &str, font: egui::FontId, color: Color32) {
    let galley = ui.painter().layout_no_wrap(tr(value), font, color);
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .galley(
            egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5),
            galley,
            color,
        );
}

pub(crate) fn header(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    page: Page,
    action: &str,
    primary: bool,
) -> bool {
    let action_width = 158.0;
    let copy_width = (ui.available_width() - 58.0 - action_width - 24.0).max(100.0);
    let title_galley = ui
        .painter()
        .layout(tr(title), font(28.0, true), foreground(), copy_width);
    let subtitle_galley = ui
        .painter()
        .layout(tr(subtitle), font(13.0, false), muted(), copy_width);
    // Center both controls against the complete title/subtitle block, including
    // wrapped translations and the user's chosen text scale. Bottom page spacing
    // is excluded from this alignment box.
    let subtitle_offset = title_galley.size().y + 8.0;
    let copy_height = subtitle_offset + subtitle_galley.size().y;
    let content_height = copy_height.max(HEIGHT);
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), content_height + 24.0),
        egui::Sense::hover(),
    );
    let center_y = rect.top() + content_height * 0.5;
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 20.0, center_y),
        Vec2::splat(HEIGHT),
    );
    ui.painter()
        .rect_filled(icon_rect, 10.0, mix_color(panel(), ACCENT, 0.08));
    paint_navigation_icon(
        ui.painter(),
        icon_rect.shrink(12.0),
        page,
        readable_app_color(ACCENT),
    );
    let copy_pos = egui::pos2(rect.left() + 58.0, center_y - copy_height * 0.5);
    let subtitle_y = copy_pos.y + subtitle_offset;
    ui.painter().galley(copy_pos, title_galley, foreground());
    ui.painter()
        .galley(egui::pos2(copy_pos.x, subtitle_y), subtitle_galley, muted());
    button(
        ui,
        egui::Rect::from_center_size(
            egui::pos2(rect.right() - action_width * 0.5, center_y),
            Vec2::new(action_width, HEIGHT),
        ),
        "header-action",
        action,
        if primary { Glyph::Plus } else { Glyph::Arrow },
        primary,
        true,
    )
    .clicked()
}

pub(crate) fn standard(ui: &mut egui::Ui, label: &str, primary: bool) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(tr(label))
                .font(font(13.0, true))
                .color(if primary {
                    Color32::WHITE
                } else {
                    foreground()
                }),
        )
        .fill(if primary { ACCENT } else { panel() })
        .stroke(egui::Stroke::new(
            1.0_f32,
            if primary { ACCENT } else { border() },
        ))
        .corner_radius(6)
        .min_size(Vec2::new(100.0, HEIGHT)),
    )
}

pub(crate) fn section(
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    subtitle: &str,
    action: &str,
) -> bool {
    let rect = row(ui);
    let copy = egui::Rect::from_min_max(rect.min, rect.max - Vec2::new(186.0, 0.0));
    text(
        ui,
        egui::Rect::from_min_size(copy.min, Vec2::new(copy.width(), 22.0)),
        title,
        font(18.0, true),
        foreground(),
    );
    text(
        ui,
        egui::Rect::from_min_max(
            copy.min + Vec2::new(0.0, 24.0),
            copy.max + Vec2::new(0.0, 2.0),
        ),
        subtitle,
        font(12.0, false),
        muted(),
    );
    let clicked = button(
        ui,
        egui::Rect::from_min_size(
            rect.right_top() - Vec2::new(172.0, 0.0),
            Vec2::new(172.0, HEIGHT),
        ),
        id,
        action,
        Glyph::Arrow,
        false,
        true,
    )
    .clicked();
    ui.add_space(12.0);
    clicked
}

pub(crate) fn metric(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    caption: &str,
    page: Page,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 108.0), egui::Sense::click());
    ui.painter().rect_filled(
        rect,
        10.0,
        if response.hovered() { card() } else { panel() },
    );
    ui.painter().rect_stroke(
        rect,
        10.0,
        egui::Stroke::new(
            1.0_f32,
            if response.has_focus() {
                ACCENT
            } else {
                border()
            },
        ),
        egui::StrokeKind::Inside,
    );
    let copy = rect.shrink2(Vec2::new(18.0, 12.0));
    text(
        ui,
        egui::Rect::from_min_size(copy.min, Vec2::new(copy.width() - 28.0, 20.0)),
        label,
        font(12.0, true),
        muted(),
    );
    text(
        ui,
        egui::Rect::from_min_size(
            copy.min + Vec2::new(0.0, 24.0),
            Vec2::new(copy.width() - 28.0, 32.0),
        ),
        value,
        font(28.0, true),
        foreground(),
    );
    text(
        ui,
        egui::Rect::from_min_size(
            copy.min + Vec2::new(0.0, 66.0),
            Vec2::new(copy.width(), 18.0),
        ),
        caption,
        font(12.0, false),
        muted(),
    );
    paint_navigation_icon(
        ui.painter(),
        egui::Rect::from_center_size(
            rect.right_center() - Vec2::new(28.0, 0.0),
            Vec2::splat(20.0),
        ),
        page,
        readable_app_color(ACCENT),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("{label}: {value}. {caption}"),
        )
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn setting_choice(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    options: &[&str],
    selected: usize,
) -> Option<usize> {
    let rect = row(ui);
    let width = 280.0_f32.min(rect.width() * 0.55);
    text(
        ui,
        egui::Rect::from_min_max(rect.min, rect.max - Vec2::new(width + 16.0, 0.0)),
        label,
        font(13.0, false),
        foreground(),
    );
    let items: Vec<_> = options.iter().map(|label| (*label, None)).collect();
    tabs(
        ui,
        egui::Rect::from_min_size(
            rect.right_top() - Vec2::new(width, 0.0),
            Vec2::new(width, HEIGHT),
        ),
        id,
        &items,
        selected,
    )
}

pub(crate) fn field(ui: &mut egui::Ui, id: &str, value: &mut String, hint: &str) {
    let rect = row(ui);
    let input_id = ui.id().with(id);
    let focused = ui.memory(|m| m.has_focus(input_id));
    ui.painter()
        .rect_filled(rect, 6.0, if focused { card() } else { ink() });
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0_f32, if focused { ACCENT } else { border() }),
        egui::StrokeKind::Inside,
    );
    let mut input = cell(ui, rect);
    input.add_sized(
        rect.size(),
        egui::TextEdit::singleline(value)
            .id(input_id)
            .font(font(13.0, false))
            .text_color(foreground())
            .frame(false)
            .margin(Vec2::new(12.0, 10.0))
            .hint_text(tr(hint)),
    );
}

/// Neutral segmented navigation. Counts are visually subordinate to the label.
pub(crate) fn segments(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: &str,
    items: &[(&str, usize)],
    selected: usize,
) -> Option<usize> {
    let counted: Vec<_> = items
        .iter()
        .map(|(label, count)| (*label, Some(*count)))
        .collect();
    tabs(ui, rect, id, &counted, selected)
}

pub(crate) fn tabs(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: &str,
    items: &[(&str, Option<usize>)],
    selected: usize,
) -> Option<usize> {
    ui.painter().rect_filled(rect, 8.0, panel());
    ui.painter().rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(1.0_f32, border()),
        egui::StrokeKind::Inside,
    );
    let mut result = None;
    let width = (rect.width() - 8.0) / items.len() as f32;
    for (i, (label, count)) in items.iter().enumerate() {
        let r = egui::Rect::from_min_size(
            rect.min + Vec2::new(4.0 + i as f32 * width, 4.0),
            Vec2::new(width, rect.height() - 8.0),
        );
        let response = ui
            .interact(r, ui.id().with((id, i)), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(tr(*label));
        let active = selected == i;
        if active || response.hovered() {
            ui.painter().rect_filled(
                r,
                5.0,
                if active {
                    mix_color(card(), foreground(), 0.06)
                } else {
                    card()
                },
            );
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                r,
                5.0,
                egui::Stroke::new(1.0_f32, ACCENT),
                egui::StrokeKind::Inside,
            );
        }
        // Center the label and optional count together, with equal padding in every tab.
        let color = if active { foreground() } else { muted() };
        let count_galley = count.map(|value| {
            ui.painter()
                .layout_no_wrap(value.to_string(), font(11.0, true), color)
        });
        let badge_width = count_galley
            .as_ref()
            .map(|g| (g.size().x + 12.0).max(22.0))
            .unwrap_or(0.0);
        let gap = if count.is_some() { 8.0 } else { 0.0 };
        let mut job =
            egui::text::LayoutJob::simple_singleline(tr(*label), font(13.0, active), color);
        job.wrap.max_width = (r.width() - 20.0 - badge_width - gap).max(1.0);
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        let label_galley = ui.painter().layout_job(job);
        let start = r.center().x - (label_galley.size().x + gap + badge_width) * 0.5;
        let painter = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
        painter.galley(
            egui::pos2(start, r.center().y - label_galley.size().y * 0.5),
            label_galley.clone(),
            color,
        );
        if let Some(count_galley) = count_galley {
            let badge = egui::Rect::from_center_size(
                egui::pos2(
                    start + label_galley.size().x + gap + badge_width * 0.5,
                    r.center().y,
                ),
                Vec2::new(badge_width, (count_galley.size().y + 4.0).max(20.0)),
            );
            painter.rect_filled(
                badge,
                4.0,
                if active {
                    mix_color(panel(), ACCENT, 0.13)
                } else {
                    ink()
                },
            );
            painter.galley(
                badge.center() - count_galley.size() * 0.5,
                count_galley,
                color,
            );
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, tr(*label))
        });
        if response.clicked() {
            result = Some(i);
        }
    }
    result
}

pub(crate) fn search(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: &str,
    value: &mut String,
    hint: &str,
) {
    let input_id = ui.id().with(id);
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
        ui.memory_mut(|m| m.request_focus(input_id));
    }
    let focused = ui.memory(|m| m.has_focus(input_id));
    if focused && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        value.clear();
        ui.memory_mut(|m| m.surrender_focus(input_id));
    }
    ui.painter()
        .rect_filled(rect, 6.0, if focused { card() } else { panel() });
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0_f32, if focused { ACCENT } else { border() }),
        egui::StrokeKind::Inside,
    );
    let center = rect.left_center() + Vec2::new(18.0, -1.0);
    ui.painter()
        .circle_stroke(center, 5.0, egui::Stroke::new(1.5_f32, muted()));
    ui.painter().line_segment(
        [center + Vec2::splat(4.0), center + Vec2::splat(8.0)],
        egui::Stroke::new(1.5_f32, muted()),
    );
    let reserve = if value.is_empty() && rect.width() > 310.0 {
        68.0
    } else {
        36.0
    };
    let input_rect = egui::Rect::from_min_max(
        rect.min + Vec2::new(36.0, 1.0),
        rect.max - Vec2::new(reserve, 1.0),
    );
    let mut input = cell(ui, input_rect);
    input.add_sized(
        input_rect.size(),
        egui::TextEdit::singleline(value)
            .id(input_id)
            .font(font(13.0, false))
            .text_color(foreground())
            .hint_text(tr(hint))
            .frame(false)
            .margin(Vec2::new(0.0, 10.0)),
    );
    if !value.is_empty() {
        let mut clear = cell(
            ui,
            egui::Rect::from_min_size(rect.right_top() + Vec2::new(-34.0, 4.0), Vec2::splat(32.0)),
        );
        if icon_button_sized(&mut clear, ButtonIcon::Close, "Clear search", muted(), 32.0).clicked()
        {
            value.clear();
            ui.memory_mut(|m| m.request_focus(input_id));
        }
    } else if reserve > 36.0 {
        let key_rect = egui::Rect::from_center_size(
            rect.right_center() - Vec2::new(36.0, 0.0),
            Vec2::new(50.0, 22.0),
        );
        ui.painter().rect_stroke(
            key_rect,
            4.0,
            egui::Stroke::new(1.0_f32, border()),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            key_rect.center(),
            egui::Align2::CENTER_CENTER,
            if cfg!(target_os = "macos") {
                "⌘ F"
            } else {
                "Ctrl F"
            },
            font(11.0, false),
            muted(),
        );
    }
}

pub(crate) fn select(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: &str,
    value: &mut String,
    options: &[&str],
) {
    let mut child = cell(ui, rect);
    child.visuals_mut().widgets.inactive.weak_bg_fill = panel();
    child.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, border());
    child.visuals_mut().widgets.inactive.corner_radius = 6.into();
    egui::ComboBox::from_id_salt(id)
        .width(rect.width())
        .height(320.0)
        .selected_text(tr(value.as_str()))
        .show_ui(&mut child, |ui| {
            ui.spacing_mut().interact_size.y = 32.0;
            for option in options {
                ui.selectable_value(value, (*option).into(), tr(*option));
            }
        });
}

/// Spacing separates toolbars from content without a decorative rule.
pub(crate) fn divider(ui: &mut egui::Ui) {
    ui.add_space(16.0);
}

pub(crate) fn views(ui: &mut egui::Ui, rect: egui::Rect, selected: &mut ProjectView) -> bool {
    ui.painter().rect_filled(rect, 6.0, panel());
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0_f32, border()),
        egui::StrokeKind::Inside,
    );
    let mut changed = false;
    for (i, (view, icon, label)) in [
        (ProjectView::List, ButtonIcon::List, "List"),
        (ProjectView::Grid, ButtonIcon::Grid, "Grid"),
        (ProjectView::Waterfall, ButtonIcon::Waterfall, "Waterfall"),
    ]
    .into_iter()
    .enumerate()
    {
        let r = egui::Rect::from_min_size(
            rect.min + Vec2::new(4.0 + i as f32 * 36.0, 4.0),
            Vec2::splat(32.0),
        );
        if *selected == view {
            ui.painter()
                .rect_filled(r, 4.0, mix_color(card(), foreground(), 0.06));
        }
        let mut child = cell(ui, r);
        if icon_button_sized(
            &mut child,
            icon,
            label,
            if *selected == view {
                foreground()
            } else {
                muted()
            },
            32.0,
        )
        .clicked()
        {
            *selected = view;
            changed = true;
        }
    }
    changed
}
