//! Window placement is stored separately from preferences and never records the splash.
use eframe::egui::{self, Vec2, ViewportCommand};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Placement {
    size: [f32; 2],
    // Physical desktop coordinates preserve placement across monitor scale factors.
    position: Option<[f32; 2]>,
    maximized: bool,
}

impl Default for Placement {
    fn default() -> Self {
        Self { size: [1240.0, 800.0], position: None, maximized: false }
    }
}

pub(super) struct Memory {
    placement: Placement,
    restored_at: Option<Instant>,
    changed_at: Option<Instant>,
}

impl Memory {
    pub(super) fn load() -> Self {
        let placement = super::app_data()
            .and_then(|p| std::fs::read(p.join("window-state.json")).ok())
            .and_then(|data| serde_json::from_slice::<Placement>(&data).ok())
            .filter(|p| p.size.iter().all(|v| v.is_finite() && *v >= 64.0 && *v <= 32768.0)
                && p.position.is_none_or(|xy| xy.iter().all(|v| v.is_finite() && v.abs() < 1_000_000.0)))
            .unwrap_or_default();
        Self { placement, restored_at: None, changed_at: None }
    }

    pub(super) fn restore(&mut self, ctx: &egui::Context) {
        let size = Vec2::new(self.placement.size[0].max(920.0), self.placement.size[1].max(640.0));
        ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(Vec2::new(920.0, 640.0)));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
        if let Some(position) = self.placement.position {
            let position = accessible_position(position, size * ctx.pixels_per_point());
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(position[0], position[1]) / ctx.pixels_per_point()));
        }
        ctx.send_viewport_cmd(ViewportCommand::Maximized(self.placement.maximized));
        // Allow the OS to finish the resize/maximize before observing normal geometry.
        self.restored_at = Some(Instant::now());
        ctx.request_repaint_after(Duration::from_millis(350));
    }

    pub(super) fn observe(&mut self, ctx: &egui::Context) {
        if !self.restored_at.is_some_and(|at| at.elapsed() >= Duration::from_millis(350)) { return; }
        let viewport = ctx.input(|i| i.viewport().clone());
        // Hidden/minimized windows can report sentinel coordinates or zero dimensions.
        if viewport.minimized == Some(true) || viewport.fullscreen == Some(true) { return; }
        let Some(inner) = viewport.inner_rect.filter(|r| r.is_finite() && r.width() >= 920.0 && r.height() >= 640.0) else { return; };
        let mut next = self.placement;
        if let Some(maximized) = viewport.maximized { next.maximized = maximized; }
        if !next.maximized {
            next.size = [inner.width().round(), inner.height().round()];
            if let Some(outer) = viewport.outer_rect.filter(|r| r.is_finite() && r.min.x > -30000.0 && r.min.y > -30000.0) {
                let pos = outer.min * ctx.pixels_per_point();
                next.position = Some([pos.x.round(), pos.y.round()]);
            }
        }
        if next != self.placement {
            self.placement = next;
            self.changed_at = Some(Instant::now());
        }
        if self.changed_at.is_some_and(|at| at.elapsed() >= Duration::from_millis(500)) {
            self.flush();
        } else if self.changed_at.is_some() {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }

    pub(super) fn flush(&mut self) {
        if self.changed_at.is_none() { return; }
        if let Some(dir) = super::app_data() {
            if let Ok(data) = serde_json::to_vec_pretty(&self.placement) {
                if std::fs::create_dir_all(&dir).is_ok()
                    && std::fs::write(dir.join("window-state.json"), data).is_ok() {
                    self.changed_at = None;
                }
            }
        }
    }
}

// Recover a window saved on a monitor that has since been disconnected.
#[cfg(target_os = "windows")]
fn accessible_position(position: [f32; 2], size: Vec2) -> [f32; 2] {
    use windows::Win32::{Foundation::RECT, Graphics::Gdi::{GetMonitorInfoW, MonitorFromRect, MONITORINFO, MONITOR_DEFAULTTONEAREST}};
    let rect = RECT { left: position[0] as i32, top: position[1] as i32,
        right: (position[0] + size.x) as i32, bottom: (position[1] + size.y) as i32 };
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    unsafe {
        if GetMonitorInfoW(MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST), &mut info).as_bool() {
            let work = info.rcWork;
            return [position[0].clamp(work.left as f32, (work.right as f32 - size.x).max(work.left as f32)),
                position[1].clamp(work.top as f32, (work.bottom as f32 - size.y).max(work.top as f32))];
        }
    }
    position
}

#[cfg(not(target_os = "windows"))]
fn accessible_position(position: [f32; 2], _size: Vec2) -> [f32; 2] { position }
