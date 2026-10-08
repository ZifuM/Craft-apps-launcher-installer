#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod manager;
mod model;
mod platform;

use eframe::egui::{
    self, pos2, vec2, Align, Align2, Button, Color32, FontId, Frame, Layout, Margin, RichText, Rect,
    Rounding, Sense, Stroke, TextStyle, TextureHandle, TextureOptions, Ui, UiBuilder, Vec2,
};
use model::{scan_projects, time_ago, AppDef, Config, Project, APPS};
use std::path::PathBuf;
use manager::{Event, Release};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ───────────────────────────────── palette ────────────────────────────────────

const BG: Color32 = Color32::from_rgb(0x12, 0x13, 0x18);
const SIDEBAR: Color32 = Color32::from_rgb(0x0C, 0x0D, 0x11);
const CARD: Color32 = Color32::from_rgb(0x1B, 0x1D, 0x25);
const CARD_HOVER: Color32 = Color32::from_rgb(0x22, 0x25, 0x30);
const BORDER: Color32 = Color32::from_rgb(0x2A, 0x2D, 0x38);
const TEXT: Color32 = Color32::from_rgb(0xEC, 0xEE, 0xF3);
const MUTED: Color32 = Color32::from_rgb(0x8A, 0x90, 0xA2);
const ACCENT: Color32 = Color32::from_rgb(0x4C, 0x8D, 0xFF);
const GOOD: Color32 = Color32::from_rgb(0x3D, 0xD6, 0x8C);
const BAD: Color32 = Color32::from_rgb(0xFF, 0x6B, 0x6B);
const WARN: Color32 = Color32::from_rgb(0xFF, 0xB0, 0x40);

/// Readable text colour for a given background.
fn on_color(bg: Color32) -> Color32 {
    let lum = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if lum > 140.0 { Color32::from_rgb(0x10, 0x12, 0x18) } else { Color32::WHITE }
}

// ─────────────────────────────────── app ──────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Apps,
    Projects,
    Manager,
    Settings,
}

struct AppState {
    shortcut: Option<PathBuf>,
    icon: Option<TextureHandle>,
}

/// A download/install in progress.
struct Job {
    done: u64,
    total: u64,
    /// Set once the download is finished and the installer is running.
    installing: Option<String>,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct MgrState {
    release: Option<Result<Release, String>>,
    loading: bool,
    job: Option<Job>,
}

enum RowAction {
    None,
    Install,
    Cancel,
    Retry,
    Notes,
    Uninstall,
}

struct LauncherApp {
    cfg: Config,
    tab: Tab,
    apps: Vec<AppState>,
    logo: Option<TextureHandle>,

    projects: Vec<Project>,
    scanning: bool,
    scan_rx: Option<Receiver<Vec<Project>>>,
    filter: Option<usize>,
    search: String,
    sort_by_name: bool,

    mgr: Vec<MgrState>,
    /// Version read from each installed .exe.
    installed: Vec<Option<String>>,
    mgr_tx: Sender<Event>,
    mgr_rx: Receiver<Event>,

    msg_tx: Sender<String>,
    msg_rx: Receiver<String>,
    toast: Option<(String, Instant)>,
}

impl LauncherApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);

        let logo = image::load_from_memory(include_bytes!("../assets/icon.png"))
            .ok()
            .map(|i| {
                let r = i.to_rgba8();
                cc.egui_ctx.load_texture(
                    "logo",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [r.width() as usize, r.height() as usize],
                        r.as_raw(),
                    ),
                    TextureOptions::LINEAR,
                )
            });

        let (msg_tx, msg_rx) = channel();
        let (mgr_tx, mgr_rx) = channel();
        let mut app = Self {
            cfg: Config::load(),
            tab: Tab::Apps,
            apps: Vec::new(),
            logo,
            projects: Vec::new(),
            scanning: false,
            scan_rx: None,
            filter: None,
            search: String::new(),
            sort_by_name: false,
            mgr: APPS.iter().map(|_| MgrState::default()).collect(),
            installed: vec![None; APPS.len()],
            mgr_tx,
            mgr_rx,
            msg_tx,
            msg_rx,
            toast: None,
        };
        app.refresh_apps(&cc.egui_ctx);
        app.start_scan();
        app.check_updates(&cc.egui_ctx);
        app
    }

    /// Find every app's shortcut and load its icon.
    fn refresh_apps(&mut self, ctx: &egui::Context) {
        self.apps.clear();
        for def in APPS.iter() {
            let shortcut = platform::find_shortcut(&self.cfg.start_menu_dir, def.name);

            // 1. a user-supplied PNG, 2. the icon of the installed app.
            let img = custom_icon(def.name)
                .or_else(|| shortcut.as_ref().and_then(|p| platform::shell_icon(p)));

            let icon = img.map(|im| {
                let im = downscale(im, 160);
                ctx.load_texture(
                    format!("icon-{}", def.name),
                    egui::ColorImage::from_rgba_unmultiplied([im.width, im.height], &im.pixels),
                    TextureOptions::LINEAR,
                )
            });
            self.apps.push(AppState { shortcut, icon });
        }
        self.refresh_versions(ctx);
    }

    fn refresh_versions(&self, ctx: &egui::Context) {
        let shortcuts: Vec<Option<PathBuf>> = self.apps.iter().map(|a| a.shortcut.clone()).collect();
        let tx = self.mgr_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let v = platform::installed_versions(&shortcuts);
            let _ = tx.send(Event::Versions(v));
            ctx.request_repaint();
        });
    }

    fn check_updates(&mut self, ctx: &egui::Context) {
        for (i, def) in APPS.iter().enumerate() {
            self.mgr[i].loading = true;
            manager::check(i, def.repo, self.cfg.include_prereleases, self.mgr_tx.clone(), ctx.clone());
        }
    }

    /// Best guess at the installed version of app `i`.
    fn installed_version(&self, i: usize) -> Option<String> {
        self.apps[i].shortcut.as_ref()?;
        manager::effective_installed(
            self.installed.get(i).and_then(|v| v.as_deref()),
            self.cfg.installed_tags.get(APPS[i].name).map(|s| s.as_str()),
        )
    }

    fn update_available(&self, i: usize) -> bool {
        let Some(Ok(rel)) = &self.mgr[i].release else { return false };
        if rel.asset.is_none() {
            return false;
        }
        self.installed_version(i)
            .and_then(|v| manager::is_newer(&rel.tag, &v))
            .unwrap_or(false)
    }

    fn update_count(&self) -> usize {
        (0..APPS.len()).filter(|&i| self.update_available(i)).count()
    }

    fn start_install(&mut self, i: usize, ctx: &egui::Context) {
        if self.mgr[i].job.is_some() {
            return;
        }
        let Some(Ok(rel)) = &self.mgr[i].release else { return };
        let rel = rel.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        self.mgr[i].job = Some(Job {
            done: 0,
            total: rel.asset.as_ref().map_or(0, |a| a.size),
            installing: None,
            cancel: cancel.clone(),
        });
        manager::install(i, APPS[i].name, APPS[i].repo, rel, cancel, self.mgr_tx.clone(), ctx.clone());
    }

    fn start_scan(&mut self) {
        self.scanning = true;
        let cfg = self.cfg.clone();
        let (tx, rx) = channel();
        self.scan_rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(scan_projects(&cfg));
        });
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    fn launch_app(&mut self, i: usize) {
        let Some(lnk) = self.apps[i].shortcut.clone() else { return };
        match platform::launch_app(&lnk) {
            Ok(()) => self.toast(format!("Launching {}…", APPS[i].name)),
            Err(e) => self.toast(format!("Couldn't launch {}: {e}", APPS[i].name)),
        }
    }

    fn open_project(&mut self, idx: usize) {
        let p = self.projects[idx].clone();
        let Some(lnk) = self.apps[p.app].shortcut.clone() else {
            self.toast(format!("{} wasn't found on this PC", APPS[p.app].name));
            return;
        };
        self.toast(format!("Opening {} in {}…", p.name, APPS[p.app].name));
        let tx = self.msg_tx.clone();
        std::thread::spawn(move || {
            if let Err(e) = platform::open_project(&lnk, &p.path) {
                let _ = tx.send(format!("Couldn't open project: {e}"));
            }
        });
    }

    fn poll(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.scan_rx {
            if let Ok(list) = rx.try_recv() {
                self.projects = list;
                self.scanning = false;
                self.scan_rx = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        while let Ok(m) = self.msg_rx.try_recv() {
            self.toast(m);
        }
        while let Ok(ev) = self.mgr_rx.try_recv() {
            match ev {
                Event::Release(i, r) => {
                    self.mgr[i].loading = false;
                    self.mgr[i].release = Some(r);
                }
                Event::Versions(v) => self.installed = v,
                Event::Progress(i, done, total) => {
                    if let Some(j) = &mut self.mgr[i].job {
                        j.done = done;
                        j.total = total;
                    }
                }
                Event::Installing(i, msg) => {
                    if let Some(j) = &mut self.mgr[i].job {
                        j.installing = Some(msg);
                    }
                }
                Event::Finished(i, res) => {
                    self.mgr[i].job = None;
                    match res {
                        Ok(tag) => {
                            self.cfg.installed_tags.insert(APPS[i].name.to_string(), tag.clone());
                            self.cfg.save();
                            self.toast(format!("{} {tag} installed", APPS[i].name));
                            self.refresh_apps(ctx);
                            self.start_scan();
                        }
                        Err(e) => self.toast(format!("{}: {e}", APPS[i].name)),
                    }
                }
            }
        }
        if self.mgr.iter().any(|m| m.job.is_some()) {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
    }

    // ────────────────────────────── sidebar ──────────────────────────────

    fn sidebar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if let Some(l) = &self.logo {
                ui.add(egui::Image::new((l.id(), vec2(40.0, 40.0))));
            }
            ui.vertical(|ui| {
                ui.add_space(2.0);
                ui.label(RichText::new("Craft Suite").size(17.0).strong().color(TEXT));
                ui.label(RichText::new("Launcher").size(12.5).color(MUTED));
            });
        });
        ui.add_space(26.0);

        ui.label(RichText::new("WORKSPACE").size(11.0).color(MUTED));
        ui.add_space(2.0);
        let project_label = format!("Projects   {}", self.projects.len());
        if nav_item(ui, "Apps", self.tab == Tab::Apps) {
            self.tab = Tab::Apps;
        }
        if nav_item(ui, &project_label, self.tab == Tab::Projects) {
            self.tab = Tab::Projects;
        }
        let n_updates = self.update_count();
        let manager_label = if n_updates > 0 {
            format!("App Manager   ({n_updates})")
        } else {
            "App Manager".to_string()
        };
        if nav_item(ui, &manager_label, self.tab == Tab::Manager) {
            self.tab = Tab::Manager;
        }
        if nav_item(ui, "Settings", self.tab == Tab::Settings) {
            self.tab = Tab::Settings;
        }

        let found = self.apps.iter().filter(|a| a.shortcut.is_some()).count();
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.label(RichText::new("Craft Launcher v0.1.0").size(11.5).color(MUTED));
            ui.label(
                RichText::new(format!("{found} of {} apps installed", APPS.len()))
                    .size(12.5)
                    .color(if found == APPS.len() { GOOD } else { MUTED }),
            );
        });
    }

    // ───────────────────────────── apps tab ──────────────────────────────

    fn apps_tab(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Your apps").size(30.0).strong().color(TEXT));
        ui.label(RichText::new("Launch any app in the suite with one click.").size(14.5).color(MUTED));
        ui.add_space(22.0);

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let avail = ui.available_width();
            let gap = 18.0;
            let cols = (((avail + gap) / (250.0 + gap)).floor() as usize).max(1);
            let w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).min(330.0);

            for row_start in (0..APPS.len()).step_by(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for i in row_start..(row_start + cols).min(APPS.len()) {
                        self.app_card(ui, i, w);
                    }
                });
                ui.add_space(gap - 10.0);
            }
        });
    }

    fn app_card(&mut self, ui: &mut Ui, i: usize, w: f32) {
        let def = &APPS[i];
        let found = self.apps[i].shortcut.is_some();
        let n_projects = self.projects.iter().filter(|p| p.app == i).count();
        let update = self.update_available(i);

        let (rect, resp) = ui.allocate_exact_size(vec2(w, 266.0), Sense::hover());
        let hovered = resp.hovered();
        ui.painter().rect(
            rect,
            Rounding::same(16.0),
            if hovered { CARD_HOVER } else { CARD },
            Stroke::new(1.0, if hovered { def.color.gamma_multiply(0.75) } else { BORDER }),
        );

        let inner = rect.shrink(20.0);
        let mut launch = false;
        let mut show_projects = false;
        let mut goto_manager = false;

        ui.allocate_new_ui(
            UiBuilder::new().max_rect(inner).layout(Layout::top_down(Align::Min)),
            |ui| {
                draw_icon(ui, def, self.apps[i].icon.as_ref(), 76.0);
                ui.add_space(12.0);
                ui.label(RichText::new(def.name).size(19.0).strong().color(TEXT));
                ui.label(RichText::new(def.tagline).size(13.0).color(MUTED));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 4.0, if found { GOOD } else { BAD });
                    if update {
                        let r = ui.add(
                            egui::Label::new(RichText::new("Update available").size(12.5).color(WARN))
                                .sense(Sense::click()),
                        );
                        if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            goto_manager = true;
                        }
                    } else {
                        ui.label(
                            RichText::new(if found { "Installed" } else { "Not installed" })
                                .size(12.5)
                                .color(MUTED),
                        );
                    }
                    if found && !update && n_projects > 0 {
                        ui.label(RichText::new("·").color(MUTED));
                        let label = format!("{n_projects} project{}", if n_projects == 1 { "" } else { "s" });
                        if ui.link(RichText::new(label).size(12.5)).clicked() {
                            show_projects = true;
                        }
                    }
                });

                let btn_rect = Rect::from_min_size(
                    pos2(inner.min.x, inner.max.y - 40.0),
                    vec2(inner.width(), 40.0),
                );
                let btn = Button::new(
                    RichText::new(if found { "Launch" } else { "Install" })
                        .size(15.0)
                        .strong()
                        .color(on_color(def.color)),
                )
                .fill(def.color)
                .rounding(Rounding::same(10.0));
                let r = ui.put(btn_rect, btn);
                if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    if found {
                        launch = true;
                    } else {
                        goto_manager = true;
                    }
                }
            },
        );

        if launch {
            self.launch_app(i);
        }
        if show_projects {
            self.filter = Some(i);
            self.tab = Tab::Projects;
        }
        if goto_manager {
            self.tab = Tab::Manager;
        }
    }

    // ─────────────────────────── projects tab ────────────────────────────

    fn projects_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Projects").size(30.0).strong().color(TEXT));
                ui.label(
                    RichText::new("Everything you've made, across every app.").size(14.5).color(MUTED),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let label = if self.scanning { "Scanning…" } else { "Refresh" };
                if ui.add_enabled(!self.scanning, Button::new(label)).clicked() {
                    self.start_scan();
                }
            });
        });
        ui.add_space(14.0);

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search projects…")
                    .desired_width(260.0),
            );
            ui.add_space(6.0);
            if chip(ui, "Recent", !self.sort_by_name, ACCENT) {
                self.sort_by_name = false;
            }
            if chip(ui, "A–Z", self.sort_by_name, ACCENT) {
                self.sort_by_name = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            if chip(ui, "All apps", self.filter.is_none(), ACCENT) {
                self.filter = None;
            }
            for (i, def) in APPS.iter().enumerate() {
                if chip(ui, def.name, self.filter == Some(i), def.color) {
                    self.filter = if self.filter == Some(i) { None } else { Some(i) };
                }
            }
        });
        ui.add_space(8.0);

        // Build the filtered, sorted list of indices.
        let q = self.search.to_lowercase();
        let mut list: Vec<usize> = self
            .projects
            .iter()
            .enumerate()
            .filter(|(_, p)| self.filter.map_or(true, |f| p.app == f))
            .filter(|(_, p)| q.is_empty() || p.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        if self.sort_by_name {
            list.sort_by_key(|&i| self.projects[i].name.to_lowercase());
        }

        if self.scanning && self.projects.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.spinner();
                ui.label(RichText::new("Looking for projects…").color(MUTED));
            });
            return;
        }
        if list.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No projects found").size(18.0).strong().color(TEXT));
                ui.label(
                    RichText::new("Check each app's project folder and file types in Settings.")
                        .color(MUTED),
                );
                ui.add_space(8.0);
                if ui.button("Open Settings").clicked() {
                    self.tab = Tab::Settings;
                }
            });
            return;
        }

        ui.label(
            RichText::new(format!("{} project{}", list.len(), if list.len() == 1 { "" } else { "s" }))
                .size(12.5)
                .color(MUTED),
        );
        ui.add_space(4.0);

        let mut open: Option<usize> = None;
        let mut reveal: Option<usize> = None;
        let row_h = 64.0;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(
            ui,
            row_h,
            list.len(),
            |ui, range| {
                for r in range {
                    let idx = list[r];
                    let p = &self.projects[idx];
                    let def = &APPS[p.app];
                    let installed = self.apps[p.app].shortcut.is_some();

                    let (rect, resp) = ui.allocate_exact_size(
                        vec2(ui.available_width(), row_h - 2.0),
                        Sense::click(),
                    );
                    let hovered = resp.hovered();
                    ui.painter().rect(
                        rect,
                        Rounding::same(12.0),
                        if hovered { CARD_HOVER } else { CARD },
                        Stroke::new(1.0, BORDER),
                    );
                    if resp.double_clicked() && installed {
                        open = Some(idx);
                    }

                    let inner = rect.shrink2(vec2(14.0, 8.0));
                    let text_w = (inner.width() - 40.0 - 14.0 - 400.0).max(120.0);
                    ui.allocate_new_ui(
                        UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)),
                        |ui| {
                            draw_icon(ui, def, self.apps[p.app].icon.as_ref(), 38.0);
                            ui.add_space(8.0);
                            ui.vertical(|ui| {
                                ui.set_width(text_w);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&p.name).size(15.0).strong().color(TEXT),
                                    )
                                    .truncate(),
                                );
                                let dir = p.path.parent().map(|d| d.display().to_string()).unwrap_or_default();
                                ui.add(egui::Label::new(RichText::new(dir).size(12.0).color(MUTED)).truncate());
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let open_btn = Button::new(
                                    RichText::new("Open").size(14.0).strong().color(on_color(def.color)),
                                )
                                .fill(def.color)
                                .min_size(vec2(78.0, 32.0))
                                .rounding(Rounding::same(9.0));
                                if ui
                                    .add_enabled(installed, open_btn)
                                    .on_hover_text(format!("Open in {}", def.name))
                                    .clicked()
                                {
                                    open = Some(idx);
                                }
                                if ui
                                    .add(Button::new("Folder").min_size(vec2(64.0, 32.0)))
                                    .on_hover_text("Show in File Explorer")
                                    .clicked()
                                {
                                    reveal = Some(idx);
                                }
                                ui.add_space(6.0);
                                ui.label(RichText::new(time_ago(p.modified)).size(12.5).color(MUTED));
                                ui.add_space(8.0);
                                ui.label(RichText::new(def.name).size(12.5).strong().color(def.color));
                            });
                        },
                    );
                }
            },
        );

        if let Some(i) = open {
            self.open_project(i);
        }
        if let Some(i) = reveal {
            platform::show_in_folder(&self.projects[i].path);
        }
    }

    // ──────────────────────────── app manager tab ────────────────────────

    fn manager_tab(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let updates = self.update_count();
        let checking = self.mgr.iter().any(|m| m.loading);
        let mut do_check = false;
        let mut do_update_all = false;

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("App Manager").size(30.0).strong().color(TEXT));
                ui.label(
                    RichText::new("Install and update the Craft apps from their official GitHub releases.")
                        .size(14.5)
                        .color(MUTED),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let label = if checking { "Checking…" } else { "Check for updates" };
                if ui.add_enabled(!checking, Button::new(label)).clicked() {
                    do_check = true;
                }
                if updates > 0 {
                    let b = Button::new(
                        RichText::new(format!("Update all ({updates})")).strong().color(Color32::WHITE),
                    )
                    .fill(ACCENT);
                    if ui.add(b).clicked() {
                        do_update_all = true;
                    }
                }
            });
        });
        ui.add_space(8.0);
        if ui
            .checkbox(
                &mut self.cfg.include_prereleases,
                "Include pre-release builds (all of these apps are still early alpha)",
            )
            .changed()
        {
            self.cfg.save();
            do_check = true;
        }
        ui.add_space(8.0);

        let mut actions: Vec<(usize, RowAction)> = Vec::new();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for i in 0..APPS.len() {
                let a = self.manager_row(ui, i);
                if !matches!(a, RowAction::None) {
                    actions.push((i, a));
                }
                ui.add_space(2.0);
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Downloads come from github.com/storytold and are checked against the SHA-256 \
                     GitHub publishes for each file before they run.",
                )
                .size(12.0)
                .color(MUTED),
            );
            ui.add_space(20.0);
        });

        for (i, a) in actions {
            match a {
                RowAction::Install => self.start_install(i, &ctx),
                RowAction::Cancel => {
                    if let Some(j) = &self.mgr[i].job {
                        j.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                RowAction::Retry => {
                    self.mgr[i].loading = true;
                    manager::check(i, APPS[i].repo, self.cfg.include_prereleases, self.mgr_tx.clone(), ctx.clone());
                }
                RowAction::Notes => {
                    if let Some(Ok(r)) = &self.mgr[i].release {
                        platform::open_url(&r.html_url);
                    }
                }
                RowAction::Uninstall => {
                    platform::open_apps_settings();
                    self.toast(format!("Find {} in Windows' Installed apps list", APPS[i].name));
                }
                RowAction::None => {}
            }
        }
        if do_update_all {
            for i in 0..APPS.len() {
                if self.update_available(i) {
                    self.start_install(i, &ctx);
                }
            }
        }
        if do_check {
            self.check_updates(&ctx);
        }
    }

    fn manager_row(&self, ui: &mut Ui, i: usize) -> RowAction {
        let def = &APPS[i];
        let st = &self.mgr[i];
        let found = self.apps[i].shortcut.is_some();
        let installed = self.installed_version(i);
        let update = self.update_available(i);
        let mut action = RowAction::None;

        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
        ui.painter().rect(
            rect,
            Rounding::same(14.0),
            CARD,
            Stroke::new(1.0, if resp.hovered() { def.color.gamma_multiply(0.6) } else { BORDER }),
        );

        let inner = rect.shrink2(vec2(18.0, 12.0));
        let info_w = (inner.width() - 52.0 - 12.0 - 190.0 - 330.0).max(160.0);

        ui.allocate_new_ui(
            UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)),
            |ui| {
                draw_icon(ui, def, self.apps[i].icon.as_ref(), 52.0);
                ui.add_space(6.0);

                ui.vertical(|ui| {
                    ui.set_width(180.0);
                    ui.label(RichText::new(def.name).size(17.0).strong().color(TEXT));
                    ui.label(RichText::new(def.tagline).size(12.5).color(MUTED));
                });

                ui.vertical(|ui| {
                    ui.set_width(info_w);
                    // installed line
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.0, if found { GOOD } else { MUTED });
                        let text = if !found {
                            "Not installed".to_string()
                        } else {
                            match &installed {
                                Some(v) => format!("Installed  {v}"),
                                None => "Installed (version unknown)".to_string(),
                            }
                        };
                        ui.add(egui::Label::new(RichText::new(text).size(13.0).color(TEXT)).truncate());
                    });
                    // latest line
                    let (text, color) = match &st.release {
                        None if st.loading => ("Checking for updates…".to_string(), MUTED),
                        None => ("-".to_string(), MUTED),
                        Some(Err(e)) => (e.clone(), BAD),
                        Some(Ok(r)) => match &r.asset {
                            None => ("No Windows installer in the latest release".to_string(), WARN),
                            Some(a) => (
                                format!(
                                    "Latest  {}{}  ·  {}  ·  {:.1} MB",
                                    r.tag,
                                    if r.prerelease { " (pre-release)" } else { "" },
                                    r.published,
                                    a.size as f64 / 1_048_576.0
                                ),
                                if update { WARN } else { MUTED },
                            ),
                        },
                    };
                    ui.add(egui::Label::new(RichText::new(text).size(12.5).color(color)).truncate());
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // ── a job is running ──
                    if let Some(job) = &st.job {
                        if let Some(msg) = &job.installing {
                            ui.label(RichText::new(msg).size(12.5).color(MUTED));
                            ui.spinner();
                        } else {
                            if ui.button("Cancel").clicked() {
                                action = RowAction::Cancel;
                            }
                            let frac = if job.total > 0 { job.done as f32 / job.total as f32 } else { 0.0 };
                            ui.add(
                                egui::ProgressBar::new(frac.clamp(0.0, 1.0))
                                    .desired_width(180.0)
                                    .fill(def.color)
                                    .text(format!(
                                        "{:.0}%  ({:.1} / {:.1} MB)",
                                        frac * 100.0,
                                        job.done as f64 / 1_048_576.0,
                                        job.total as f64 / 1_048_576.0
                                    )),
                            );
                        }
                        return;
                    }

                    // ── idle ──
                    let ready = matches!(&st.release, Some(Ok(r)) if r.asset.is_some());
                    if matches!(&st.release, Some(Err(_))) {
                        if ui.button("Retry").clicked() {
                            action = RowAction::Retry;
                        }
                        return;
                    }
                    if !ready {
                        return;
                    }

                    let (label, primary) = if !found {
                        ("Install", true)
                    } else if update {
                        ("Update", true)
                    } else {
                        ("Up to date", false)
                    };

                    if primary {
                        let b = Button::new(
                            RichText::new(label).size(14.5).strong().color(on_color(def.color)),
                        )
                        .fill(def.color)
                        .min_size(vec2(110.0, 36.0))
                        .rounding(Rounding::same(10.0));
                        if ui.add(b).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            action = RowAction::Install;
                        }
                    } else {
                        ui.add_sized(
                            [110.0, 36.0],
                            egui::Label::new(RichText::new("✓ Up to date").size(13.5).color(GOOD)),
                        );
                    }

                    ui.menu_button("More", |ui| {
                        if ui.button("Release notes").clicked() {
                            action = RowAction::Notes;
                            ui.close_menu();
                        }
                        if found {
                            if !primary && ui.button("Reinstall").clicked() {
                                action = RowAction::Install;
                                ui.close_menu();
                            }
                            if ui.button("Uninstall…").clicked() {
                                action = RowAction::Uninstall;
                                ui.close_menu();
                            }
                        }
                    });
                });
            },
        );
        action
    }

    // ──────────────────────────── settings tab ───────────────────────────

    fn settings_tab(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        ui.label(RichText::new("Settings").size(30.0).strong().color(TEXT));
        ui.label(
            RichText::new("Tell the launcher where to find your apps and each app's projects.")
                .size(14.5)
                .color(MUTED),
        );
        ui.add_space(18.0);

        let mut save = false;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("App shortcuts folder").strong().size(15.5).color(TEXT));
                ui.label(
                    RichText::new("Where the launcher looks for PhotoCraft.lnk, FilmCraft.lnk, etc.")
                        .size(12.5)
                        .color(MUTED),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.cfg.start_menu_dir)
                        .desired_width(f32::INFINITY),
                );
            });
            ui.add_space(14.0);

            for (i, def) in APPS.iter().enumerate() {
                let found = self.apps[i].shortcut.is_some();
                let tex = self.apps[i].icon.clone();
                card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        draw_icon(ui, def, tex.as_ref(), 34.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(def.name).strong().size(15.5).color(TEXT));
                            ui.label(
                                RichText::new(if found { "Installed" } else { "Shortcut not found" })
                                    .size(12.0)
                                    .color(if found { GOOD } else { BAD }),
                            );
                        });
                    });
                    ui.add_space(4.0);

                    let s = self.cfg.apps.entry(def.name.to_string()).or_default();
                    ui.label(RichText::new("Project folder").size(12.5).color(MUTED));
                    ui.horizontal(|ui| {
                        let w = ui.available_width() - 90.0;
                        ui.add(
                            egui::TextEdit::singleline(&mut s.project_dir)
                                .desired_width(w),
                        );
                        if ui.button("Browse…").clicked() {
                            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                s.project_dir = dir.display().to_string();
                            }
                        }
                    });
                    ui.label(
                        RichText::new("Project file types (comma separated, blank = all files)")
                            .size(12.5)
                            .color(MUTED),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut s.extensions)
                            .hint_text("e.g. psd, png, craft")
                            .desired_width(f32::INFINITY),
                    );
                });
                ui.add_space(14.0);
            }

            let btn = Button::new(RichText::new("Save & rescan").strong().size(15.0).color(Color32::WHITE))
                .fill(ACCENT)
                .min_size(vec2(160.0, 40.0))
                .rounding(Rounding::same(10.0));
            if ui.add(btn).clicked() {
                save = true;
            }
            ui.add_space(30.0);
        });

        if save {
            self.cfg.save();
            self.refresh_apps(&ctx);
            self.start_scan();
            self.toast("Settings saved");
        }
    }

    fn toast_ui(&mut self, ctx: &egui::Context) {
        let Some((msg, at)) = &self.toast else { return };
        if at.elapsed() > Duration::from_secs(3) {
            self.toast = None;
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(250));
        egui::Area::new(egui::Id::new("toast"))
            .anchor(Align2::RIGHT_BOTTOM, vec2(-26.0, -26.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                Frame::none()
                    .fill(Color32::from_rgb(0x2B, 0x2F, 0x3D))
                    .stroke(Stroke::new(1.0, BORDER))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(Margin::symmetric(18.0, 12.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg.as_str()).size(14.0).color(TEXT));
                    });
            });
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);

        egui::SidePanel::left("sidebar")
            .exact_width(236.0)
            .resizable(false)
            .frame(Frame::none().fill(SIDEBAR).inner_margin(Margin::same(20.0)))
            .show(ctx, |ui| self.sidebar(ui));

        egui::CentralPanel::default()
            .frame(Frame::none().fill(BG).inner_margin(Margin::symmetric(40.0, 30.0)))
            .show(ctx, |ui| match self.tab {
                Tab::Apps => self.apps_tab(ui),
                Tab::Projects => self.projects_tab(ui),
                Tab::Manager => self.manager_tab(ui),
                Tab::Settings => self.settings_tab(ui),
            });

        self.toast_ui(ctx);
    }
}

// ───────────────────────────── widgets & helpers ──────────────────────────────

fn card_frame() -> Frame {
    Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .rounding(Rounding::same(14.0))
        .inner_margin(Margin::same(18.0))
}

fn nav_item(ui: &mut Ui, label: &str, selected: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, Rounding::same(10.0), ACCENT.gamma_multiply(0.22));
        painter.rect_filled(
            Rect::from_min_size(rect.min + vec2(0.0, 9.0), vec2(3.0, rect.height() - 18.0)),
            Rounding::same(2.0),
            ACCENT,
        );
    } else if resp.hovered() {
        painter.rect_filled(rect, Rounding::same(10.0), Color32::from_white_alpha(10));
    }
    painter.text(
        rect.left_center() + vec2(16.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(15.0),
        if selected { TEXT } else { MUTED },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn chip(ui: &mut Ui, label: &str, selected: bool, color: Color32) -> bool {
    let b = Button::new(
        RichText::new(label).size(13.0).color(if selected { on_color(color) } else { MUTED }),
    )
    .fill(if selected { color } else { CARD })
    .stroke(Stroke::new(1.0, if selected { color } else { BORDER }))
    .rounding(Rounding::same(16.0))
    .min_size(vec2(0.0, 30.0));
    ui.add(b).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Draws an app's logo, or a clean branded badge if no icon could be loaded.
fn draw_icon(ui: &mut Ui, def: &AppDef, tex: Option<&TextureHandle>, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter();
    match tex {
        Some(t) => {
            painter.image(
                t.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            painter.rect(
                rect,
                Rounding::same(size * 0.22),
                Color32::from_rgb(0x0B, 0x10, 0x1E),
                Stroke::new((size * 0.04).max(1.5), def.color),
            );
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                def.badge,
                FontId::proportional(size * 0.46),
                def.color,
            );
        }
    }
}

/// Optional override: drop `<AppName>.png` (or .webp) into %APPDATA%\CraftLauncher\icons\
fn custom_icon(name: &str) -> Option<platform::RgbaImage> {
    let dir = model::data_dir().join("icons");
    ["png", "webp"].iter().find_map(|ext| {
        let img = image::open(dir.join(format!("{name}.{ext}"))).ok()?.to_rgba8();
        Some(platform::RgbaImage {
            width: img.width() as usize,
            height: img.height() as usize,
            pixels: img.into_raw(),
        })
    })
}

fn downscale(im: platform::RgbaImage, max: usize) -> platform::RgbaImage {
    if im.width <= max && im.height <= max {
        return im;
    }
    let Some(buf) = image::RgbaImage::from_raw(im.width as u32, im.height as u32, im.pixels) else {
        return platform::RgbaImage { width: 1, height: 1, pixels: vec![0; 4] };
    };
    let scale = max as f32 / im.width.max(im.height) as f32;
    let (w, h) = (
        ((im.width as f32 * scale) as u32).max(1),
        ((im.height as f32 * scale) as u32).max(1),
    );
    let out = image::imageops::resize(&buf, w, h, image::imageops::FilterType::Lanczos3);
    platform::RgbaImage { width: w as usize, height: h as usize, pixels: out.into_raw() }
}

fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(0x0E, 0x0F, 0x14);
    v.faint_bg_color = CARD;
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    let btn = Color32::from_rgb(0x26, 0x29, 0x34);
    let btn_hover = Color32::from_rgb(0x33, 0x37, 0x47);
    v.widgets.noninteractive.bg_fill = CARD;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.bg_fill = btn;
    v.widgets.inactive.weak_bg_fill = btn;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.hovered.bg_fill = btn_hover;
    v.widgets.hovered.weak_bg_fill = btn_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, MUTED);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.active.bg_fill = btn_hover;
    v.widgets.active.weak_bg_fill = btn_hover;
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
    ] {
        w.rounding = Rounding::same(10.0);
    }

    let mut style = (*ctx.style()).clone();
    style.visuals = v;
    style.spacing.item_spacing = vec2(10.0, 10.0);
    style.spacing.button_padding = vec2(14.0, 8.0);
    style.text_styles.insert(TextStyle::Body, FontId::proportional(14.5));
    style.text_styles.insert(TextStyle::Button, FontId::proportional(14.5));
    style.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let icon = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .ok()
        .map(|i| {
            let r = i.to_rgba8();
            egui::IconData { width: r.width(), height: r.height(), rgba: r.into_raw() }
        });

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Craft Launcher")
        .with_inner_size([1200.0, 780.0])
        .with_min_inner_size([960.0, 620.0]);
    if let Some(i) = icon {
        viewport = viewport.with_icon(Arc::new(i));
    }

    eframe::run_native(
        "Craft Launcher",
        eframe::NativeOptions { viewport, ..Default::default() },
        Box::new(|cc| Ok(Box::new(LauncherApp::new(cc)))),
    )
}
