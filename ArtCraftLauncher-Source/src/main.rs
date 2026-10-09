#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
mod windows_tray;
mod windows_ui;

mod project_preview;
#[path = "windows_suite_update.rs"]
mod suite_update;
mod windows_releases;
#[path = "windows_ui/app_screens.rs"]
mod app_screens;
#[path = "windows_ui/app_workspace.rs"]
mod app_workspace;
mod localization;
#[path = "windows_ui/project_dialogs.rs"]
mod project_dialogs;
#[path = "windows_ui/onboarding.rs"]
mod onboarding;
#[path = "windows_ui/cloud_ui.rs"]
mod cloud_ui;
mod plugins;
#[path = "windows_ui/plugin_ui.rs"]
mod plugin_ui;
mod workspace_bridge;
mod cloud;
mod platform;
#[cfg(target_os="linux")]
mod linux_tray;
#[cfg(target_os="macos")]
mod macos;
use localization::tr;
mod preview_formats;
#[cfg(target_os = "windows")]
mod preview_windows;

use chrono::{DateTime, Local};
use eframe::egui::{self, Color32, RichText, Vec2};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{Cursor, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, mpsc::{self, Receiver, Sender}},
    thread,
    time::{Duration, Instant},
};
use walkdir::WalkDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO: &str = "https://github.com/storytold";
const ACCENT: Color32 = Color32::from_rgb(130, 99, 255);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Cloud,
    Home,
    Apps,
    YourApps,
    Projects,
    Settings,
    App(&'static str),
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
enum ProjectView {
    #[default]
    List,
    Grid,
    Waterfall,
}

#[derive(Clone, Copy)]
struct AppInfo {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    blurb: &'static str,
    filetypes: &'static [&'static str],
    tint: Color32,
    group: AppGroup,
    has_release: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppGroup { Creative, Office }

const APPS: &[AppInfo] = &[
    AppInfo {
        id: "photocraft",
        name: "PhotoCraft",
        category: "IMAGE EDITOR",
        blurb: "Layers, masks, type and real PSD files.",
        filetypes: &[
            "pcraft", "psd", "psb", "png", "jpg", "jpeg", "tif", "tiff", "webp",
        ],
        tint: Color32::from_rgb(48, 112, 238),
        group: AppGroup::Creative,
        has_release: true,
    },
    AppInfo {
        id: "vectorcraft",
        name: "VectorCraft",
        category: "VECTOR ILLUSTRATION",
        blurb: "Illustration, typography and shape craft.",
        filetypes: &["vectorcraft", "svg", "eps", "ai"],
        tint: Color32::from_rgb(232, 78, 66),
        group: AppGroup::Creative,
        has_release: true,
    },
    AppInfo {
        id: "filmcraft",
        name: "FilmCraft",
        category: "VIDEO EDITOR",
        blurb: "Edit, color, effects, audio and titles.",
        filetypes: &["fcproj", "otio", "edl", "aaf"],
        tint: Color32::from_rgb(139, 88, 246),
        group: AppGroup::Creative,
        has_release: true,
    },
    AppInfo {
        id: "lightcraft",
        name: "LightCraft",
        category: "PHOTO LIBRARY",
        blurb: "A local photo library and raw developer.",
        filetypes: &[
            "dng", "cr2", "cr3", "nef", "arw", "raf", "orf", "rw2", "pef",
        ],
        tint: Color32::from_rgb(246, 171, 24),
        group: AppGroup::Creative,
        has_release: true,
    },
    AppInfo {
        id: "printcraft",
        name: "PdfCraft",
        category: "PDF WORKBENCH",
        blurb: "Read, organize, combine and secure PDFs.",
        filetypes: &["pdf"],
        tint: Color32::from_rgb(18, 158, 143),
        group: AppGroup::Office,
        has_release: true,
    },
    AppInfo {
        id: "effectcraft",
        name: "EffectCraft",
        category: "MOTION & VFX",
        blurb: "Compositions, keyframes and visual effects.",
        filetypes: &["ecproj", "lottie"],
        tint: Color32::from_rgb(232, 47, 146),
        group: AppGroup::Creative,
        has_release: true,
    },
    AppInfo {
        id: "designcraft",
        name: "DesignCraft",
        category: "PAGE LAYOUT",
        blurb: "Page layout, typography and publishing.",
        filetypes: &["designcraft", "dcbook", "idml"],
        tint: Color32::from_rgb(126, 187, 35),
        group: AppGroup::Creative,
        has_release: true,
    },
    AppInfo {
        id: "soundcraft",
        name: "SoundCraft",
        category: "AUDIO WORKSTATION",
        blurb: "Record, edit and mix audio in a creative workspace.",
        filetypes: &["wav", "aiff", "aif", "flac"],
        tint: Color32::from_rgb(15, 156, 145),
        group: AppGroup::Creative,
        has_release: true,
    },
    AppInfo {
        id: "cadcraft",
        name: "CADCraft",
        category: "CAD & DRAFTING",
        blurb: "Create precise technical drawings and CAD projects.",
        filetypes: &["dxf", "dwg"],
        tint: Color32::from_rgb(54, 174, 196),
        group: AppGroup::Office,
        has_release: true,
    },
    AppInfo {
        id: "gridcraft",
        name: "GridCraft",
        category: "SPREADSHEETS",
        blurb: "Organize data, calculations and spreadsheets.",
        filetypes: &["xlsx", "csv", "tsv"],
        tint: Color32::from_rgb(65, 177, 119),
        group: AppGroup::Office,
        has_release: true,
    },
    AppInfo {
        id: "wordcraft",
        name: "WordCraft",
        category: "WORD PROCESSOR",
        blurb: "Write and format documents in one workspace.",
        filetypes: &["docx", "odt", "rtf", "html", "md", "txt"],
        tint: Color32::from_rgb(64, 132, 221),
        group: AppGroup::Office,
        has_release: true,
    },
    AppInfo {
        id: "deckcraft",
        name: "DeckCraft",
        category: "PRESENTATIONS",
        blurb: "Build and present polished slide decks.",
        filetypes: &["deckcraft", "pptx"],
        tint: Color32::from_rgb(236, 137, 58),
        group: AppGroup::Office,
        has_release: true,
    },
];

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    onboarding_complete: bool,
    language: String,
    automatic_updates: bool,
    automatic_suite_updates: bool,
    beta_suite_updates: bool,
    update_interval_hours: u64,
    update_notifications: bool,
    automatic_project_scan: bool,
    project_scan_minutes: u64,
    reduce_motion: bool,
    compact_sidebar: bool,
    classic_sidebar: bool,
    classic_app_screens: bool,
    light_mode: bool,
    minimize_to_tray: bool,
    start_with_windows: bool,
    roots: Vec<PathBuf>,
    #[serde(default)]
    default_project_root: Option<PathBuf>,
    #[serde(default)]
    project_view: ProjectView,
    cloud_project_view: ProjectView,
    text_scale: u8,
    project_sort: String,
    favorite_projects: Vec<PathBuf>,
    installed: HashMap<String, String>,
    #[serde(skip)]
    scanning: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            onboarding_complete: false,
            language: "en".into(),
            automatic_updates: true, automatic_suite_updates: true, beta_suite_updates: false, update_interval_hours: 4,
            update_notifications: true, automatic_project_scan: true,
            project_scan_minutes: 3, reduce_motion: false, compact_sidebar: false, classic_sidebar: false, classic_app_screens: false, light_mode: false, minimize_to_tray: false, start_with_windows: false,
            roots: Vec::new(), default_project_root: None,
            cloud_project_view: ProjectView::List,
            text_scale: 0,
            project_view: ProjectView::List, project_sort: "Recently modified".into(), favorite_projects: Vec::new(), installed: HashMap::new(), scanning: false,
        }
    }
}

#[derive(Clone)]
struct Project {
    revision: u64,
    path: PathBuf,
    title: String,
    app: &'static AppInfo,
    modified: Option<DateTime<Local>>,
    size_bytes: u64,
    preview: Option<Arc<image::RgbaImage>>,
    preview_note: &'static str,
}

struct ProjectRename {
    focused: bool,
    path: PathBuf,
    name: String,
}

#[derive(Clone)]
enum ManualUpdateStatus {
    Checking,
    Current(String),
    Available(String),
    Failed(String),
}

#[derive(Clone)]
struct ManualUpdateCheck {
    app_id: String,
    status: ManualUpdateStatus,
}

#[derive(Clone, Default)]
struct AppState {
    latest: Option<String>,
    installed: Option<String>,
    busy: Option<String>,
    error: Option<String>,
    icon: Option<egui::TextureHandle>,
}

#[derive(Clone)]
struct PendingLaunch {
    app: &'static AppInfo,
    executable: PathBuf,
    working_directory: PathBuf,
    file: Option<PathBuf>,
    started: Instant,
    orbit_apps: Vec<&'static AppInfo>,
}

enum Event {
    PreviewChanges(bool),
    SuiteUpdateProgress(String),
    SuiteUpdateReady(Result<Option<suite_update::Prepared>, String>),
    TrayOpen,
    TrayQuit,
    ReleaseChecksComplete(Vec<(String, Result<String, String>)>),
    ManualRelease(String, Result<String, String>),
    Progress(String, String),
    Logo(String, egui::TextureHandle),
    Done(String, Result<String, String>),
    Scan(Vec<Project>),
}

struct Launcher {
    plugins: plugins::Manager,
    onboarding_step: usize,
    onboarding_error: Option<String>,
    cloud: cloud::Cloud,
    #[cfg(target_os = "windows")]
    tray: Option<windows_tray::Tray>,
    #[cfg(target_os="linux")]
    linux_tray: Option<linux_tray::Tray>,
    #[cfg(target_os="macos")]
    mac_tray: Option<macos::Tray>,
    tray_failed: bool,
    silent_start: bool,
    tray_open_requested: bool,
    tray_quit_requested: bool,
    applied_light_mode: Option<bool>,
    page: Page,
    detail_parent: Page,
    settings_tab: usize,
    your_apps_search: String,
    manager_search: String,
    collection_group: u8,
    workspace_tabs: HashMap<String, u8>,
    workspace_search: HashMap<String, String>,
    transitioned_page: Page,
    page_transition_started: Instant,
    brand_icon: egui::TextureHandle,
    prefs: Preferences,
    states: HashMap<String, AppState>,
    projects: Vec<Project>,
    project_previews: HashMap<PathBuf, egui::TextureHandle>,
    events_rx: Receiver<Event>,
    events_tx: Sender<Event>,
    search: String,
    filter: String,
    app_category: AppGroup,
    project_filter: String,
    projects_tab: bool,
    project_scope: String,
    toast: Option<String>,
    toast_last_message: Option<String>,
    toast_started: Option<Instant>,
    persistent_toast: Option<String>,
    checked: bool,
    release_check_busy: bool,
    suite_check_started: Option<Instant>,
    suite_update_busy: bool,
    suite_update_status: String,
    suite_update_ready: Option<(suite_update::Prepared, Instant)>,
    logo_loaded: bool,
    properties: Option<windows_ui::menus::Properties>,
    show_remove: Option<String>,
    show_project_rename: Option<ProjectRename>,
    show_project_delete: Option<Project>,
    pending_launch: Option<PendingLaunch>,
    last_release_check: Instant,
    last_project_scan: Instant,
    last_preview_check: Instant,
    preview_check_busy: bool,
    manual_update_check: Option<ManualUpdateCheck>,
    startup_splash_started: Instant,
    startup_splash_initialized: bool,
    startup_splash_window_shown: bool,
    startup_splash_finished: bool,
    startup_splash_centered: bool,
    window_rounding_applied: bool,
    sidebar_collapsed: bool,
    orbit_apps: Vec<&'static AppInfo>,
}

impl Launcher {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (events_tx, events_rx) = mpsc::channel();
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.window_fill = panel();
        style.visuals.panel_fill = ink();
        style.visuals.extreme_bg_color = ink();
        style.visuals.faint_bg_color = panel();
        style.visuals.widgets.noninteractive.bg_fill = panel();
        style.visuals.widgets.inactive.bg_fill = theme_rgb(39, 41, 48);
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, border());
        style.visuals.widgets.inactive.corner_radius = 8.into();
        style.visuals.widgets.hovered.bg_fill = theme_rgb(50, 51, 61);
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, theme_rgb(102, 87, 160));
        style.visuals.widgets.hovered.corner_radius = 8.into();
        style.visuals.widgets.active.bg_fill = theme_rgb(79, 62, 132);
        style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT);
        style.visuals.widgets.active.corner_radius = 8.into();
        style.visuals.selection.bg_fill = ACCENT;
        style.interaction.selectable_labels = false;
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(13.0, 8.0);
        style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(text_size(14.0)));
        style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(text_size(13.0)));
        style.visuals.override_text_color = Some(theme_rgb(230, 232, 238));
        style.spacing.scroll = suite_scroll_style();
        cc.egui_ctx.set_style(style);
        let brand = image::load_from_memory(include_bytes!("../assets/artcraft-icon.png"))
            .expect("embedded ArtCraft logo is a valid PNG")
            .to_rgba8();
        let brand_image = egui::ColorImage::from_rgba_unmultiplied(
            [brand.width() as usize, brand.height() as usize],
            brand.as_raw(),
        );
        let brand_icon = cc.egui_ctx.load_texture(
            "artcraft-launcher-brand",
            brand_image,
            egui::TextureOptions::LINEAR,
        );
        let mut prefs = read_preferences();
        localization::select(&prefs.language);
        windows_ui::set_text_scale(prefs.text_scale);
        localization::install_fonts(&cc.egui_ctx);
        if !prefs
            .default_project_root
            .as_ref()
            .is_some_and(|root| prefs.roots.contains(root))
        {
            prefs.default_project_root = prefs.roots.first().cloned();
            save_preferences(&prefs);
        }
        for root in &prefs.roots {
            for app in APPS {
                let _ = workspace_bridge::prepare(root, app);
            }
        }
        let mut states = HashMap::new();
        for app in APPS {
            let installed = prefs
                .installed
                .get(app.id)
                .cloned()
                .or_else(|| detect_install(app));
            states.insert(
                app.id.to_owned(),
                AppState {
                    installed,
                    ..Default::default()
                },
            );
        }
        let mut launcher = Self {
            #[cfg(target_os = "windows")]
            tray: None,
            #[cfg(target_os="linux")]
            linux_tray: None,
            #[cfg(target_os="macos")]
            mac_tray: None,
            tray_failed: false,
            silent_start: prefs.start_with_windows && std::env::args().any(|arg| arg == "--tray"),
            tray_open_requested: false,
            tray_quit_requested: false,
            applied_light_mode: None,
            orbit_apps: random_orbit_apps(None),
            page: Page::Home,
            detail_parent: Page::YourApps,
            settings_tab: 0,
            your_apps_search: String::new(),
            manager_search: String::new(),
            collection_group: 0,
            workspace_tabs: HashMap::new(),
            workspace_search: HashMap::new(),
            transitioned_page: Page::Home,
            page_transition_started: Instant::now() - Duration::from_millis(300),
            brand_icon,
            sidebar_collapsed: prefs.compact_sidebar,
            prefs,
            states,
            projects: vec![],
            project_previews: HashMap::new(),
            events_rx,
            events_tx,
            search: String::new(),
            filter: "All apps".into(),
            app_category: AppGroup::Creative,
            project_filter: "All apps".into(),
            projects_tab: false,
            project_scope: "All projects".into(),
            toast: None,
            toast_last_message: None,
            toast_started: None,
            persistent_toast: None,
            checked: false,
            release_check_busy: false,
            suite_check_started: None,
            suite_update_busy: false,
            suite_update_status: "Not checked yet".into(),
            suite_update_ready: None,
            logo_loaded: false,
            properties: None,
            show_remove: None,
            show_project_rename: None,
            show_project_delete: None,
            pending_launch: None,
            last_release_check: Instant::now() - Duration::from_secs(4 * 60 * 60),
            last_project_scan: Instant::now(),
            last_preview_check: Instant::now(),
            preview_check_busy: false,
            manual_update_check: None,
            plugins: plugins::Manager::default(),
            onboarding_step: 0,
            onboarding_error: None,
            cloud: cloud::Cloud::new(),
            startup_splash_started: Instant::now(),
            startup_splash_initialized: false,
            startup_splash_window_shown: false,
            startup_splash_finished: false,
            startup_splash_centered: false,
            window_rounding_applied: false,
        };
        launcher.scan_projects();
        launcher
    }

    fn check_suite_update(&mut self, manual: bool) {
        if self.suite_update_busy || self.suite_update_ready.is_some() { return; }
        if !manual && self.suite_check_started.is_some_and(|at| at.elapsed() < Duration::from_secs(4 * 60 * 60)) { return; }
        self.suite_check_started = Some(Instant::now());
        let include_beta = self.prefs.beta_suite_updates;
        self.suite_update_busy = true;
        self.suite_update_status = if platform::flatpak(){"Checking the Flatpak update source..."}else{"Checking GitHub releases..."}.into();
        let tx = self.events_tx.clone();
        thread::spawn(move || {
            let result = suite_update::prepare(VERSION, include_beta, |message| { let _ = tx.send(Event::SuiteUpdateProgress(message)); });
            let _ = tx.send(Event::SuiteUpdateReady(result));
        });
    }

    fn check_releases(&mut self) {
        if self.release_check_busy { return; }
        if self.checked && self.last_release_check.elapsed() < Duration::from_secs(self.prefs.update_interval_hours.clamp(1, 24) * 60 * 60) {
            return;
        }
        { self.release_check_busy = true; }
        self.checked = true;
        self.last_release_check = Instant::now();
        let tx = self.events_tx.clone();
        thread::spawn(move || {
            let mut results = Vec::with_capacity(APPS.len());
            for app in APPS {
                if !app.has_release { continue; }
                let id = app.id.to_owned();
                let result = (|| -> Result<String, String> {
                    let client = http_client()?;
                    Ok(latest_release(&client, &id)?.version)
                })();
                results.push((id, result));
                thread::sleep(Duration::from_millis(100));
            }
            let _ = tx.send(Event::ReleaseChecksComplete(results));
        });
    }

    fn check_app_release(&mut self, app: AppInfo) {
        if !app.has_release {
            self.manual_update_check = Some(ManualUpdateCheck {
                app_id: app.id.to_owned(),
                status: ManualUpdateStatus::Failed("No official release is published yet. Check the source repository for release announcements.".into()),
            });
            return;
        }
        self.manual_update_check = Some(ManualUpdateCheck {
            app_id: app.id.to_owned(),
            status: ManualUpdateStatus::Checking,
        });
        let tx = self.events_tx.clone();
        let id = app.id.to_owned();
        thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let client = http_client()?;
                Ok(latest_release(&client, &id)?.version)
            })();
            let _ = tx.send(Event::ManualRelease(id, result));
        });
    }

    fn draw_manual_update_dialog(&mut self, ctx: &egui::Context) {
        let Some(check) = self.manual_update_check.clone() else { return; };
        let Some(app) = app_by_id(&check.app_id).copied() else { return; };
        let installed = self.states.get(app.id).and_then(|state| state.installed.as_deref()).is_some();
        let mut open = true;
        let mut close = false;
        let mut retry = false;
        let mut install = false;
        egui::Window::new(tr("Check for updates"))
            .id(egui::Id::new("manual-update-check"))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .default_width(440.0)
            .open(&mut open)
            .frame(egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(16).inner_margin(22))
            .show(ctx, |ui| {
                ui.set_min_width(350.0);
                ui.horizontal(|ui| {
                    self.app_logo(ui, &app, 48.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tr(app.name)).size(text_size(19.0)).strong().color(foreground()));
                        ui.label(RichText::new(tr("Release status")).size(text_size(12.0)).color(muted()));
                    });
                });
                ui.add_space(16.0);
                match &check.status {
                    ManualUpdateStatus::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new(tr("Checking the latest release…")).size(text_size(13.0)).color(muted()));
                        });
                    }
                    ManualUpdateStatus::Current(version) => {
                        ui.label(RichText::new(tr("You’re up to date")).size(text_size(15.0)).strong().color(theme_rgb(108, 211, 153)));
                        ui.label(RichText::new(tr(format!("Latest version: {version}"))).size(text_size(12.0)).color(muted()));
                    }
                    ManualUpdateStatus::Available(version) => {
                        ui.label(RichText::new(tr("An update is available")).size(text_size(15.0)).strong().color(ACCENT));
                        ui.label(RichText::new(tr(format!("Version {version} is ready to install."))).size(text_size(12.0)).color(muted()));
                    }
                    ManualUpdateStatus::Failed(error) => {
                        ui.label(RichText::new(tr("Couldn’t check for updates")).size(text_size(15.0)).strong().color(theme_rgb(255, 156, 135)));
                        ui.label(RichText::new(tr(error)).size(text_size(12.0)).color(muted()));
                    }
                }
                ui.add_space(18.0);
                ui.horizontal(|ui| match &check.status {
                    ManualUpdateStatus::Checking => {
                        if ui.button(tr("Close")).clicked() { close = true; }
                    }
                    ManualUpdateStatus::Current(_) => {
                        if app_primary_button(ui, "Done", app.tint).clicked() { close = true; }
                    }
                    ManualUpdateStatus::Available(_) => {
                        if app_primary_button(ui, if installed { "Install update" } else { "Install app" }, app.tint).clicked() { install = true; }
                        if ui.button(tr("Later")).clicked() { close = true; }
                    }
                    ManualUpdateStatus::Failed(_) => {
                        if app_primary_button(ui, "Try again", app.tint).clicked() { retry = true; }
                        if ui.button(tr("Close")).clicked() { close = true; }
                    }
                });
            });
        if !open || close || install { self.manual_update_check = None; }
        if retry { self.check_app_release(app); }
        if install { self.install(app); }
    }

    fn load_logos(&mut self, ctx: &egui::Context) {
        if self.logo_loaded {
            return;
        }
        self.logo_loaded = true;
        for app in APPS {
            let tx = self.events_tx.clone();
            let ctx = ctx.clone();
            let id = app.id.to_owned();
            thread::spawn(move || {
                let result = (|| -> Result<Vec<u8>, String> {
                    let logo_slug = match id.as_str() {
                        "printcraft" => "pdfcraft",
                        "deckcraft" => "slidecraft",
                        _ => id.as_str(),
                    };
                    let client = http_client()?;
                    let repo_slug = if id == "printcraft" { "pdfcraft" } else { id.as_str() };
                    let candidates = [
                        format!("https://getartcraft.com/images/apps/{logo_slug}/icon.webp"),
                        format!("https://raw.githubusercontent.com/storytold/{id}/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.{id}.png"),
                        format!("https://raw.githubusercontent.com/storytold/{id}/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.{repo_slug}.png"),
                    ];
                    for url in candidates {
                        if let Ok(response) = client.get(url).send().and_then(reqwest::blocking::Response::error_for_status) {
                            if let Ok(bytes) = response.bytes() {
                                if !bytes.is_empty() { return Ok(bytes.to_vec()); }
                            }
                        }
                    }
                    Err("No app logo was available from the app website or source repository.".into())
                })();
                if let Ok(bytes) = result {
                    if let Ok(image) = image::load_from_memory(&bytes) {
                        let image = image.to_rgba8();
                        let size = [image.width() as usize, image.height() as usize];
                        let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                        let handle = ctx.load_texture(
                            format!("brand-{id}"),
                            color,
                            egui::TextureOptions::LINEAR,
                        );
                        let _ = tx.send(Event::Logo(id, handle));
                        ctx.request_repaint();
                    }
                }
            });
        }
    }

    fn scan_projects(&mut self) {
        if self.prefs.scanning {
            return;
        }
        self.prefs.scanning = true;
        self.last_project_scan = Instant::now();
        let roots = self.prefs.roots.clone();
        let tx = self.events_tx.clone();
        let previous: HashMap<_, _> = self.projects.iter().cloned().map(|p| (p.path.clone(), p)).collect();
        thread::spawn(move || {
            let _ = tx.send(Event::Scan(scan_roots(&roots, &previous)));
        });
    }

    fn add_project_folder(&mut self, path: PathBuf) {
        let already_watched = self.prefs.roots.contains(&path);
        let mut failed = Vec::new();
        for app in APPS {
            if let Err(error) = workspace_bridge::prepare(&path, app) {
                failed.push(format!("{} ({error})", app.name));
            }
        }
        if !already_watched {
            self.prefs.roots.push(path.clone());
            if self.prefs.default_project_root.is_none() {
                self.prefs.default_project_root = Some(path.clone());
            }
            save_preferences(&self.prefs);
        }
        self.scan_projects();
        self.toast = if failed.is_empty() {
            Some(format!(
                "{} {} and created a project folder for each app.",
                if already_watched { "Checked" } else { "Added" },
                path.display()
            ))
        } else {
            Some(format!(
                "Added {}. Could not create folders for: {}.",
                path.display(),
                failed.join(", ")
            ))
        };
    }

    fn remove_project_folder(&mut self, index: usize) {
        if index >= self.prefs.roots.len() {
            return;
        }
        let removed = self.prefs.roots.remove(index);
        if self.prefs.default_project_root.as_ref() == Some(&removed) {
            self.prefs.default_project_root = self.prefs.roots.first().cloned();
        }
        save_preferences(&self.prefs);
        self.scan_projects();
    }

    fn install(&mut self, app: AppInfo) {
        if self.states.get(app.id).is_some_and(|s| s.busy.is_some()) {
            return;
        }
        let tx = self.events_tx.clone();
        let id = app.id.to_string();
        self.states.get_mut(app.id).unwrap().busy = Some("Finding the latest release...".into());
        thread::spawn(move || {
            let result = install_release(app.id, &tx);
            let _ = tx.send(Event::Done(id, result));
        });
    }

    fn launch(&mut self, app: AppInfo, file: Option<&Path>) {
        let Some(directory) = installed_dir(app.id) else {
            self.toast = Some(format!("{} is not installed yet.", app.name));
            return;
        };
        let Some(exe) = find_executable(&directory, app.id) else {
            self.toast = Some(format!(
                "Could not find {} inside its install folder.",
                app.name
            ));
            return;
        };
        if let Some(root) = &self.prefs.default_project_root {
            if let Err(error) = plugins::prepare_launch(&app, root) {
                self.toast = Some(format!("Could not prepare {} workspace: {error}", app.name));
                return;
            }
        }
        let working_directory = self
            .prefs
            .default_project_root
            .as_ref()
            .map(|root| root.join(app.name).join("Projects"))
            .filter(|path| fs::create_dir_all(path).is_ok())
            .unwrap_or(directory);
        warm_launch_files(exe.clone());
        self.pending_launch = Some(PendingLaunch {
            app: app_by_id(app.id).expect("app exists"),
            executable: exe,
            working_directory,
            file: file.map(Path::to_path_buf),
            started: Instant::now(),
            orbit_apps: random_orbit_apps(Some(app.id)),
        });
    }

    fn draw_launch_splash(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_launch.clone() else {
            return;
        };
        let duration = Duration::from_secs(3);
        let progress = (pending.started.elapsed().as_secs_f32() / duration.as_secs_f32()).min(1.0);
        if progress >= 1.0 {
            self.pending_launch = None;
            let mut command = Command::new(&pending.executable);
            command.current_dir(&pending.working_directory);
            if let Some(root) = &self.prefs.default_project_root {
                if let Err(error) = workspace_bridge::launch_environment(&mut command, root, pending.app) {
                    self.toast = Some(error); return;
                }
            }
            if let Some(path) = pending.file.as_deref() {
                command.arg(path);
            }
            self.toast = match platform::spawn(&mut command) {
                Ok(_) => Some(format!("Opening {}...", pending.app.name)),
                Err(error) => Some(format!("Could not launch {}: {error}", pending.app.name)),
            };
            return;
        }

        let app = pending.app;
        let app_icons: Vec<_> = pending.orbit_apps
            .iter()
            .map(|candidate| (
                self.states.get(candidate.id).and_then(|state| state.icon.as_ref()).map(egui::TextureHandle::id),
                candidate.tint,
                candidate.name.chars().next().unwrap_or('?'),
            ))
            .collect();
        let selected_icon = self.states.get(app.id).and_then(|state| state.icon.as_ref()).map(egui::TextureHandle::id);
        egui::Modal::new(egui::Id::new("app-launch-splash"))
            .backdrop_color(Color32::from_black_alpha(205))
            .frame(egui::Frame::new().fill(Color32::TRANSPARENT).inner_margin(0))
            .show(ctx, |ui| {
                let (card, _) = ui.allocate_exact_size(Vec2::new(700.0, 420.0), egui::Sense::hover());
                paint_suite_splash(&ui.painter_at(card), card, app.tint, selected_icon,
                    app.name, &app.category.to_uppercase(), &format!("{}\nis getting ready.", app.name),
                    app.blurb, &app_icons, progress, self.prefs.reduce_motion);
            });
        ctx.request_repaint_after(Duration::from_millis(16));
    }
    fn draw_startup_splash(&mut self, ctx: &egui::Context, progress: f32) {
        if !self.startup_splash_centered {
            if let Some(center) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(center);
                self.startup_splash_centered = true;
            }
        }
        let logo = self.brand_icon.id();
        let app_icons: Vec<_> = self.orbit_apps
            .iter()
            .map(|app| {
                (
                    self.states
                        .get(app.id)
                        .and_then(|state| state.icon.as_ref())
                        .map(egui::TextureHandle::id),
                    app.tint,
                    app.name.chars().next().unwrap_or('?'),
                )
            })
            .collect();
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::from_rgb(28, 22, 48)).inner_margin(0)).show(ctx, |ui| {
            let rect = ui.max_rect();
            paint_suite_splash(ui.painter(), rect, ACCENT, Some(logo), "ArtCraft Master Suite",
                "ARTCRAFT MASTER SUITE", "Your creative desk\nis getting ready.",
                "Bringing your apps and projects together.", &app_icons, progress, self.prefs.reduce_motion);
        });
    }

    fn draw_resize_handles(&self, ctx: &egui::Context) {
        if !self.startup_splash_finished || ctx.input(|i| i.viewport().maximized.unwrap_or(false)) { return; }
        let bounds = ctx.screen_rect();
        let edge = 6.0;
        let corner = 14.0;
        let x0 = bounds.left();
        let x1 = bounds.right();
        let y0 = bounds.top();
        let y1 = bounds.bottom();
        let zones = [
            (egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x0 + corner, y0 + corner)), egui::ResizeDirection::NorthWest, egui::CursorIcon::ResizeNwSe),
            (egui::Rect::from_min_max(egui::pos2(x1 - corner, y0), egui::pos2(x1, y0 + corner)), egui::ResizeDirection::NorthEast, egui::CursorIcon::ResizeNeSw),
            (egui::Rect::from_min_max(egui::pos2(x0, y1 - corner), egui::pos2(x0 + corner, y1)), egui::ResizeDirection::SouthWest, egui::CursorIcon::ResizeNeSw),
            (egui::Rect::from_min_max(egui::pos2(x1 - corner, y1 - corner), egui::pos2(x1, y1)), egui::ResizeDirection::SouthEast, egui::CursorIcon::ResizeNwSe),
            (egui::Rect::from_min_max(egui::pos2(x0 + corner, y0), egui::pos2(x1 - corner, y0 + edge)), egui::ResizeDirection::North, egui::CursorIcon::ResizeVertical),
            (egui::Rect::from_min_max(egui::pos2(x0 + corner, y1 - edge), egui::pos2(x1 - corner, y1)), egui::ResizeDirection::South, egui::CursorIcon::ResizeVertical),
            (egui::Rect::from_min_max(egui::pos2(x0, y0 + corner), egui::pos2(x0 + edge, y1 - corner)), egui::ResizeDirection::West, egui::CursorIcon::ResizeEast),
            (egui::Rect::from_min_max(egui::pos2(x1 - edge, y0 + corner), egui::pos2(x1, y1 - corner)), egui::ResizeDirection::East, egui::CursorIcon::ResizeEast),
        ];
        for (index, (rect, direction, cursor)) in zones.into_iter().enumerate() {
            egui::Area::new(egui::Id::new(("native-window-resize-handle", index)))
                .order(egui::Order::Foreground)
                .fixed_pos(rect.min)
                .interactable(false)
                .show(ctx, |ui| {
                    let (_, response) = ui.allocate_exact_size(rect.size(), egui::Sense::drag());
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(cursor);
                    }
                    if response.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                    }
                });
        }
    }

    fn remove(&mut self, id: &str) {
        let path = installed_dir(id);
        match path.and_then(|p| fs::remove_dir_all(p).ok()) {
            Some(()) => {
                self.prefs.installed.remove(id);
                if let Some(s) = self.states.get_mut(id) {
                    s.installed = None;
                    s.error = None;
                }
                save_preferences(&self.prefs);
                self.toast = Some(format!(
                    "{} was removed.",
                    app_by_id(id).map(|a| a.name).unwrap_or(id)
                ));
            }
            None => self.toast = Some(
                "The app folder could not be removed. Close it if it is running, then try again."
                    .into(),
            ),
        }
        self.show_remove = None;
    }

    fn rename_project(&mut self, path: &Path, requested_name: &str) {
        let name = requested_name.trim().trim_end_matches([' ', '.']);
        if name.is_empty()
            || name.chars().any(|character| {
                matches!(character, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            })
        {
            self.toast = Some("Enter a project name that is valid for project files.".into());
            return;
        }
        let filename = match path.extension().and_then(|extension| extension.to_str()) {
            Some(extension) => format!("{name}.{extension}"),
            None => name.to_owned(),
        };
        let target = path.with_file_name(filename);
        if target == path {
            self.toast = Some("The project already has that name.".into());
            return;
        }
        if target.exists() {
            self.toast = Some("A file with that name already exists in this folder.".into());
            return;
        }
        match fs::rename(path, &target) {
            Ok(()) => {
                self.project_previews.remove(path);
                for favorite in &mut self.prefs.favorite_projects { if favorite == path { *favorite = target.clone(); } }
                save_preferences(&self.prefs);
                self.toast = Some(format!("Renamed project to {}.", target.file_name().unwrap_or_default().to_string_lossy()));
                self.scan_projects();
            }
            Err(error) => {
                self.toast = Some(format!("Could not rename this project: {error}"));
            }
        }
        self.show_project_rename = None;
    }

    fn delete_project(&mut self, path: &Path) {
        match fs::remove_file(path) {
            Ok(()) => {
                self.project_previews.remove(path);
                self.prefs.favorite_projects.retain(|favorite| favorite != path);
                save_preferences(&self.prefs);
                self.toast = Some("Project file deleted.".into());
                self.scan_projects();
            }
            Err(error) => {
                self.toast = Some(format!("Could not delete this project: {error}"));
            }
        }
        self.show_project_delete = None;
    }

    fn process_events(&mut self) {
        while let Ok(event) = self.events_rx.try_recv() {
            match event {
                Event::PreviewChanges(changed) => {
                    self.preview_check_busy = false;
                    if changed { self.scan_projects(); }
                }
                Event::SuiteUpdateProgress(message) => {
                    self.suite_update_status = message.clone();
                    self.toast = Some(message.clone()); self.persistent_toast = Some(message);
                }
                Event::SuiteUpdateReady(result) => {
                    self.suite_update_busy = false;
                    match result {
                        Ok(Some(update)) => {
                            self.suite_update_status = if self.prefs.automatic_suite_updates{format!("Master Suite {} is ready. Restarting 10 seconds after current work finishes.", update.version)}else{format!("Master Suite {} is ready. Choose Install and restart to apply it.",update.version)};
                            self.toast = Some(self.suite_update_status.clone()); self.persistent_toast = self.toast.clone();
                            self.suite_update_ready = Some((update, Instant::now()));
                        }
                        Ok(None) => { self.suite_update_status = format!("Master Suite {VERSION} is up to date."); }
                        Err(error) => {
                            self.suite_update_status = format!("Master Suite update unavailable: {error}");
                            self.toast = Some(self.suite_update_status.clone()); self.persistent_toast = self.toast.clone();
                        }
                    }
                }
                Event::TrayOpen => self.tray_open_requested = true,
                Event::TrayQuit => self.tray_quit_requested = true,
                Event::ManualRelease(id, result) => {
                    if let Some(state) = self.states.get_mut(&id) {
                        match &result {
                            Ok(version) => {
                                state.latest = Some(version.clone());
                                state.error = None;
                            }
                            Err(error) => state.error = Some(format!("Release check unavailable: {error}")),
                        }
                    }
                    if let Some(dialog) = self.manual_update_check.as_mut().filter(|dialog| dialog.app_id == id) {
                        let installed = self.states.get(&id).and_then(|state| state.installed.as_deref());
                        dialog.status = match result {
                            Ok(version) if installed == Some(version.as_str()) => ManualUpdateStatus::Current(version),
                            Ok(version) => ManualUpdateStatus::Available(version),
                            Err(error) => ManualUpdateStatus::Failed(error),
                        };
                    }
                }
                Event::ReleaseChecksComplete(results) => {
                    { self.release_check_busy = false; }
                    let mut updates = Vec::new();
                    for (id, result) in results {
                        match result {
                            Ok(version) => {
                                if let Some(s) = self.states.get_mut(&id) {
                                    if s.installed.as_deref().is_some_and(|installed| installed != version) {
                                        updates.push(format!("{} {version}", app_by_id(&id).map(|a|a.name).unwrap_or(&id)));
                                    }
                                    s.latest = Some(version);
                                    s.error = None;
                                }
                            }
                            Err(error) => if let Some(s) = self.states.get_mut(&id) {
                                s.error = Some(format!("Release check unavailable: {error}"));
                            }
                        }
                    }
                    if !updates.is_empty() && self.prefs.update_notifications {
                        let message = format!("Updates available: {}", updates.join(", "));
                        self.toast = Some(message.clone());
                        self.persistent_toast = Some(message);
                    }
                }
                Event::Logo(id, texture) => {
                    if let Some(s) = self.states.get_mut(&id) {
                        s.icon = Some(texture);
                    }
                }
                Event::Progress(id, state) => {
                    if let Some(s) = self.states.get_mut(&id) {
                        s.busy = Some(state);
                    }
                }
                Event::Done(id, Ok(version)) => {
                    self.prefs.installed.insert(id.clone(), version.clone());
                    save_preferences(&self.prefs);
                    if let Some(s) = self.states.get_mut(&id) {
                        s.installed = Some(version.clone());
                        s.busy = None;
                        s.error = None;
                    }
                    self.toast = Some(format!(
                        "{} {version} is ready to open.",
                        app_by_id(&id).map(|a| a.name).unwrap_or(&id)
                    ));
                }
                Event::Done(id, Err(error)) => {
                    if let Some(s) = self.states.get_mut(&id) {
                        s.busy = None;
                        s.error = Some(error.clone());
                    }
                    self.toast = Some(error);
                }
                Event::Scan(projects) => {
                    // Refresh textures when saved contents change, not only when paths disappear.
                    self.project_previews.retain(|path, _| {
                        let old = self.projects.iter().find(|p| &p.path == path);
                        let new = projects.iter().find(|p| &p.path == path);
                        matches!((old, new), (Some(a), Some(b)) if a.revision == b.revision && match (&a.preview, &b.preview) { (Some(a), Some(b)) => Arc::ptr_eq(a, b), (None, None) => true, _ => false })
                    });
                    self.projects = projects;
                    self.prefs.scanning = false;
                }
            }
        }
    }

    fn classic_sidebar(&mut self, ctx: &egui::Context) {
        let sidebar_width = if self.sidebar_collapsed { 72.0 } else { 224.0 };
        egui::SidePanel::left("sidebar")
            .exact_width(sidebar_width)
            .frame(
                egui::Frame::new()
                    .fill(theme_rgb(22, 23, 27))
                    .inner_margin(egui::Margin::symmetric(if self.sidebar_collapsed { 8 } else { 15 }, 17)),
            )
            .show(ctx, |ui| {
                if self.sidebar_collapsed { ui.spacing_mut().item_spacing.y = 6.0; }
                ui.horizontal(|ui| {
                    if self.sidebar_collapsed { ui.spacing_mut().item_spacing.x = 3.0; }
                    ui.allocate_ui_with_layout(Vec2::new(ui.available_width() - if self.sidebar_collapsed { 24.0 } else { 38.0 }, 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let logo_size = if self.sidebar_collapsed { 24.0 } else { 26.0 };
                    let (logo_rect, _) = ui.allocate_exact_size(Vec2::splat(logo_size), egui::Sense::hover());
                    ui.painter().image(
                        self.brand_icon.id(),
                        logo_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    if !self.sidebar_collapsed { ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr("ArtCraft"))
                                .size(text_size(16.0))
                                .strong()
                                .color(foreground()),
                        );
                        ui.label(RichText::new(tr("MASTER SUITE")).size(text_size(9.0)).strong().color(muted()));
                    }); }
                    });
                    let toggle_size = if self.sidebar_collapsed { 20.0 } else { 28.0 };
                    let (toggle_rect, toggle_response) = ui.allocate_exact_size(Vec2::splat(toggle_size), egui::Sense::click());
                    let toggle_response = toggle_response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr(if self.sidebar_collapsed { "Expand sidebar" } else { "Collapse sidebar" }));
                    if toggle_response.hovered() { ui.painter().rect_filled(toggle_rect, 6.0, theme_rgb(39, 41, 48)); }
                    let center = toggle_rect.center();
                    let direction = if self.sidebar_collapsed { 1.0 } else { -1.0 };
                    let stroke = egui::Stroke::new(1.4_f32, muted());
                    ui.painter().line_segment([center + Vec2::new(direction * -2.0, -5.0), center + Vec2::new(direction * 3.0, 0.0)], stroke);
                    ui.painter().line_segment([center + Vec2::new(direction * 3.0, 0.0), center + Vec2::new(direction * -2.0, 5.0)], stroke);
                    if toggle_response.clicked() { self.sidebar_collapsed = !self.sidebar_collapsed; self.prefs.compact_sidebar = self.sidebar_collapsed; save_preferences(&self.prefs); }
                });
                ui.add_space(if self.sidebar_collapsed { 20.0 } else { 28.0 });
                if !self.sidebar_collapsed { ui.label(RichText::new(tr("WORKSPACE")).size(text_size(10.0)).strong().color(muted())); ui.add_space(7.0); }
                self.side_link(ui, Page::Home, "Home", None);
                self.side_link(
                    ui,
                    Page::Apps,
                    "App Manager",
                    Some(
                        self.states
                            .values()
                            .filter(|s| s.installed.is_some())
                            .count(),
                    ),
                );
                self.side_link(
                    ui,
                    Page::Projects,
                    "Projects",
                    Some(self.projects.len()),
                );
                self.side_link(ui, Page::YourApps, "Your apps", Some(self.states.values().filter(|s| s.installed.is_some()).count()));
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    self.side_link(ui, Page::Cloud, "Cloud", None);
                    self.side_link(ui, Page::Settings, "Settings", None);
                    ui.add_space(12.0);
                    ui.separator();
                    if self.sidebar_collapsed {
                        let (status_rect, status_response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), egui::Sense::hover());
                        status_response.on_hover_text(tr("Local to this device"));
                        ui.painter().circle_filled(status_rect.center(), 3.5, theme_rgb(89, 204, 135));
                        return;
                    }
                    ui.horizontal(|ui| {
                        let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot.center(), 3.0, theme_rgb(89, 204, 135));
                        ui.label(
                            RichText::new(tr("Local to this device"))
                                .size(text_size(11.0))
                                .color(muted()),
                        );
                    });
                });
            });
    }





    fn side_link(
        &mut self,
        ui: &mut egui::Ui,
        page: Page,
        label: &str,
        count: Option<usize>,
    ) {
        let selected = self.page == page || (matches!(self.page, Page::App(_)) && self.detail_parent == page);
        if self.sidebar_collapsed {
            let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), egui::Sense::click());
            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr(label));
            let selected_t = ui.ctx().animate_bool(response.id.with("selected"), selected);
            let hover_t = ui.ctx().animate_bool(response.id.with("hover"), response.hovered());
            let fill = mix_color(mix_color(theme_rgb(22, 23, 27), theme_rgb(34, 35, 41), hover_t * 0.72), theme_rgb(47, 42, 68), selected_t);
            ui.painter().rect_filled(rect, 9.0, fill);
            let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(18.0));
            paint_navigation_icon(&ui.painter(), icon_rect, page, mix_color(muted(), ACCENT, selected_t));
            if response.clicked() { self.page = page; }
            return;
        }
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 38.0),
            egui::Sense::click(),
        );
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let hover_t = ui.ctx().animate_bool(response.id.with("hover"), response.hovered());
        let selected_t = ui.ctx().animate_bool(response.id.with("selected"), selected);
        let rest = mix_color(theme_rgb(22, 23, 27), theme_rgb(34, 35, 41), hover_t * 0.72);
        let fill = mix_color(rest, theme_rgb(47, 42, 68), selected_t);
        ui.painter().rect_filled(rect, 9.0, fill);
        let mut row = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(Vec2::new(11.0, 0.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let (icon_rect, _) = row.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
        let icon_tint = mix_color(muted(), ACCENT, selected_t);
        paint_navigation_icon(&row.painter(), icon_rect, page, icon_tint);
        row.add_space(10.0);
        row.label(RichText::new(tr(label)).size(text_size(13.0)).color(mix_color(muted(), foreground(), selected_t)));
        if let Some(n) = count {
            row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |row| {
                row.label(RichText::new(tr(n.to_string())).size(text_size(11.0)).color(muted()));
            });
        }
        if response.clicked() {
            self.page = page;
        }
    }

    fn app_logo(&self, ui: &mut egui::Ui, app: &AppInfo, size: f32) {
        let texture = self.states.get(app.id).and_then(|s| s.icon.clone());
        if let Some(texture) = texture {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, 12, app.tint.gamma_multiply(0.18));
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,tr(app.name.chars().next().unwrap_or('?')),
                egui::FontId::proportional(size * 0.52),
                app.tint,
            );
        }
    }

    fn project_thumbnail_at(&mut self, ui: &mut egui::Ui, project: &Project, size: Vec2) {
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
        response.on_hover_text(tr(project.preview_note));
        ui.painter().rect_filled(rect, 7.0, card());
        if let Some(preview) = project.preview.as_ref() {
            let texture = self.project_previews.entry(project.path.clone()).or_insert_with(|| {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [preview.width() as usize, preview.height() as usize],
                    preview.as_raw(),
                );
                ui.ctx().load_texture(
                    format!("project-preview:{}", project.path.display()),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            });
            let image_size = Vec2::new(preview.width() as f32, preview.height() as f32);
            let scale = ((rect.width() - 4.0) / image_size.x)
                .min((rect.height() - 4.0) / image_size.y)
                .min(1.0);
            let image_rect = egui::Rect::from_center_size(rect.center(), image_size * scale);
            ui.painter().image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            ui.painter().rect_stroke(
                rect,
                7.0,
                egui::Stroke::new(1.0_f32, border()),
                egui::StrokeKind::Inside,
            );
        } else if let Some(texture) = self
            .states
            .get(project.app.id)
            .and_then(|state| state.icon.as_ref())
        {
            let badge = egui::Rect::from_center_size(rect.center(), Vec2::splat((size.y - 12.0).min(48.0)));
            ui.painter().image(
                texture.id(),
                badge,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,tr(project.app.name.chars().next().unwrap_or('?')),
                egui::FontId::proportional(text_size(26.0)),
                muted(),
            );
        }
        if project.preview.is_none() && size.y >= 88.0 && size.x >= 160.0 {
            ui.painter().text(egui::pos2(rect.center().x, rect.bottom() - 12.0), egui::Align2::CENTER_CENTER,tr("Preview unavailable"), egui::FontId::proportional(text_size(10.0)), muted());
        }
    }

    fn classic_app_card(&mut self, ui: &mut egui::Ui, app: AppInfo) {
        let state = self.states.get(app.id).cloned().unwrap_or_default();
        egui::Frame::new()
            .fill(mix_color(card(), app.tint, 0.12))
            .stroke(egui::Stroke::new(1.0_f32, mix_color(border(), app.tint, 0.28)))
            .corner_radius(16)
            .inner_margin(17)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    self.app_logo(ui, &app, 48.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(app.name))
                                .size(text_size(16.0))
                                .strong()
                                .color(foreground()),
                        );
                        ui.label(
                            RichText::new(tr(app.category))
                                .size(text_size(10.5))
                                .strong()
                                .color(readable_app_color(app.tint)),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let dot_color = if state.installed.is_some() {
                            theme_rgb(90, 207, 142)
                        } else {
                            theme_rgb(105, 107, 114)
                        };
                        let (dot, _) = ui.allocate_exact_size(Vec2::splat(7.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot.center(), 3.0, dot_color);
                        ui.label(
                            RichText::new(tr(if !app.has_release && state.installed.is_none() {
                                "Coming soon"
                            } else if state.installed.is_some() {
                                "Installed"
                            } else {
                                "Not installed"
                            }))
                            .size(text_size(12.0))
                            .color(muted()),
                        );
                    });
                });
                ui.add_space(10.0);
                ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), 36.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.label(RichText::new(tr(app.blurb)).size(text_size(13.0)).color(muted()));
                });
                ui.add_space(7.0);
                let latest = if !app.has_release && state.installed.is_none() { "No compatible release yet".to_owned() } else { state
                    .latest
                    .as_deref()
                    .map(|v| format!("Latest  v{v}"))
                    .unwrap_or_else(|| if self.checked { "Release not available yet".into() } else { "Check for latest release".into() }) };
                let installed = state
                    .installed
                    .as_deref()
                    .map(|v| format!("Installed  v{v}"))
                    .unwrap_or_else(|| "Official desktop build".into());
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(tr(installed)).size(text_size(11.5)).color(muted()));
                    ui.label(RichText::new(tr(latest)).size(text_size(11.5)).color(muted()));
                });
                if let Some(error) = &state.error {
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(tr(error))
                            .size(text_size(11.0))
                            .color(theme_rgb(255, 156, 135)),
                    );
                }
                if let Some(progress) = &state.busy {
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spinner();
                        ui.label(RichText::new(tr(progress)).size(text_size(12.0)).color(muted()));
                    });
                } else {
                    ui.add_space(9.0);
                    ui.horizontal_wrapped(|ui| {
                        if state.installed.is_some() {
                            if app_primary_button(ui, "Open app", app.tint).clicked() {
                                self.launch(app, None);
                            }
                            if state.latest.is_some() && state.latest != state.installed {
                                if icon_button(ui, ButtonIcon::UpdateArrow, "Install update", app.tint)
                                    .clicked()
                                {
                                    self.install(app);
                                }
                            }
                            if icon_button(ui, ButtonIcon::Delete, "Uninstall app", theme_rgb(213, 83, 93)).clicked() {
                                self.show_remove = Some(app.id.to_owned());
                            }
                            if icon_button(ui, ButtonIcon::Help, "App details", muted()).clicked() {
                                self.page = Page::App(app.id);
                            }
                        } else {
                            if app.has_release {
                                if app_primary_button(ui, "Install", app.tint).clicked() {
                                    self.install(app);
                                }
                            } else {
                                ui.add_enabled(false, egui::Button::new(tr("No compatible release yet")));
                            }
                            if icon_button(ui, ButtonIcon::Help, "App details", muted()).clicked() {
                                self.page = Page::App(app.id);
                            }
                        }
                    });
                }
            });
    }





    fn classic_your_apps_page(&mut self, ui: &mut egui::Ui) {
        self.heading(ui, "YOUR COLLECTION", "Your apps", "Choose an app to open its workspace, manage updates, and pick up your projects.");
        ui.horizontal(|ui| {
            egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(9).inner_margin(10).show(ui, |ui| {
                ui.add(egui::TextEdit::singleline(&mut self.your_apps_search).hint_text(tr("Find an installed app...")).desired_width(260.0).frame(false));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if secondary_button(ui, "Manage apps").clicked() { self.page = Page::Apps; }
            });
        });
        ui.add_space(20.0);
        let installed: Vec<_> = APPS.iter().copied().filter(|a| self.states.get(a.id).is_some_and(|s| s.installed.is_some())).collect();
        if installed.is_empty() {
            settings_section(ui, "Your collection starts here", "Install your first app and it will appear here automatically.", |ui| {
                if primary_button(ui, "Explore apps").clicked() { self.page = Page::Apps; }
            });
            return;
        }
        let query = self.your_apps_search.to_lowercase();
        let apps: Vec<_> = installed.into_iter().filter(|a| format!("{} {}", a.name, a.category).to_lowercase().contains(&query)).collect();
        if apps.is_empty() { ui.label(RichText::new(tr("No apps match your search.")).color(muted())); }
        egui::ScrollArea::vertical().id_salt("your-apps-library").show(ui, |ui| {
            for (group, label) in [(AppGroup::Creative, "Creative apps"), (AppGroup::Office, "Productivity apps")] {
                let group_apps: Vec<_> = apps.iter().filter(|a| a.group == group).collect();
                if group_apps.is_empty() { continue; }
                ui.label(RichText::new(tr(format!("{}   /   {}", label, group_apps.len()))).size(text_size(16.0)).strong());
                ui.add_space(10.0);
                let columns = ((ui.available_width() + 16.0) / 300.0).floor().clamp(1.0, 4.0) as usize;
                for chunk in group_apps.chunks(columns) {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    ui.columns(columns, |uis| {
                        for (index, app) in chunk.iter().enumerate() {
                            let ui = &mut uis[index];
                            let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 178.0), egui::Sense::click());
                            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr(format!("View {}", app.name)));
                            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, app.name));
                            let hover = if self.prefs.reduce_motion { if response.hovered() { 1.0 } else { 0.0 } } else { ui.ctx().animate_bool(response.id, response.hovered()) };
                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 14.0, mix_color(card(), app.tint, 0.10 + hover * 0.09));
                            painter.rect_stroke(rect, 14.0, egui::Stroke::new(1.0_f32, mix_color(border(), app.tint, 0.25 + hover * 0.40)), egui::StrokeKind::Inside);
                            let logo = egui::Rect::from_min_size(rect.min + Vec2::splat(20.0), Vec2::splat(46.0));
                            if let Some(texture) = self.states.get(app.id).and_then(|s| s.icon.as_ref()) {
                                painter.image(texture.id(), logo, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
                            } else {
                                painter.rect_filled(logo, 10.0, app.tint);
                                painter.text(logo.center(), egui::Align2::CENTER_CENTER,tr(&app.name[..1]), egui::FontId::proportional(text_size(22.0)), foreground());
                            }
                            painter.text(rect.min + Vec2::new(80.0, 31.0), egui::Align2::LEFT_CENTER,tr(app.name), egui::FontId::proportional(text_size(18.0)), foreground());
                            painter.text(rect.min + Vec2::new(80.0, 55.0), egui::Align2::LEFT_CENTER,tr(app.category), egui::FontId::proportional(text_size(10.0)), readable_app_color(app.tint));
                            let count = self.projects.iter().filter(|p| p.app.id == app.id).count();
                            let state = &self.states[app.id];
                            let status = if state.busy.is_some() { "Working" } else if state.latest.is_some() && state.latest != state.installed { "Update available" } else { "Installed" };
                            painter.text(rect.min + Vec2::new(20.0, 100.0), egui::Align2::LEFT_CENTER,tr(format!("{}   /   {} project{}", status, count, if count == 1 { "" } else { "s" })), egui::FontId::proportional(text_size(12.0)), muted());
                            painter.text(rect.left_bottom() + Vec2::new(20.0, -25.0), egui::Align2::LEFT_CENTER,tr("View app"), egui::FontId::proportional(text_size(13.0)), foreground());
                            let c = rect.right_bottom() + Vec2::new(-25.0, -25.0);
                            painter.line_segment([c + Vec2::new(-3.0, -5.0), c + Vec2::new(2.0, 0.0)], egui::Stroke::new(1.5_f32, app.tint));
                            painter.line_segment([c + Vec2::new(2.0, 0.0), c + Vec2::new(-3.0, 5.0)], egui::Stroke::new(1.5_f32, app.tint));
                            if response.clicked() { self.detail_parent = Page::YourApps; self.page = Page::App(app.id); }
                        }
                    });
                    ui.add_space(16.0);
                }
                ui.add_space(10.0);
            }
        });
    }

    fn classic_apps_page(&mut self, ui: &mut egui::Ui) {
        self.heading(
            ui,
            "THE APP LIBRARY",
            "One toolkit. Every kind of craft.",
            "Install each app locally, launch it in a click, and keep updates together.",
        );
        ui.horizontal(|ui| {
            for (group, label) in [(AppGroup::Creative, "Creative Apps"), (AppGroup::Office, "Productivity Apps")] {
                let selected = self.app_category == group;
                let color = if selected { Color32::WHITE } else { muted() };
                if ui.add(egui::Button::new(RichText::new(tr(label)).size(text_size(13.0)).strong().color(color)).fill(if selected { ACCENT } else { panel() }).stroke(egui::Stroke::new(1.0_f32, if selected { ACCENT } else { border() })).corner_radius(9).min_size(Vec2::new(150.0, 38.0))).clicked() {
                    self.app_category = group;
                    self.filter = "All apps".into();
                }
            }
        });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            for label in ["All apps", "Installed", "Available updates"] {
                let selected = self.filter == label;
                if ui
                    .selectable_label(selected, RichText::new(tr(label)).size(text_size(12.0)))
                    .clicked()
                {
                    self.filter = label.to_owned();
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, ButtonIcon::UpdateArrow, "Check for releases", muted()).clicked() {
                    self.checked = false;
                    self.check_releases();
                }
            });
        });
        ui.add_space(18.0);
        let filtered: Vec<AppInfo> = APPS
            .iter()
            .copied()
            .filter(|a| a.group == self.app_category)
            .filter(|a| match self.filter.as_str() {
                "Installed" => self.states.get(a.id).is_some_and(|s| s.installed.is_some()),
                "Available updates" => self.states.get(a.id).is_some_and(|s| {
                    s.installed.is_some() && s.latest.is_some() && s.latest != s.installed
                }),
                _ => true,
            })
            .collect();
        if filtered.is_empty() {
            egui::Frame::new().fill(panel()).corner_radius(14).inner_margin(24).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(tr(if self.filter == "Available updates" { "No updates listed" } else { "No installed apps in this category" })).size(text_size(18.0)).strong());
                ui.label(RichText::new(tr("Choose All apps to explore the collection, or check for new releases.")).color(muted()));
            });
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            let available = ui.available_width();
            let columns = if available > 1040.0 {
                3
            } else if available > 680.0 {
                2
            } else {
                1
            };
            ui.spacing_mut().item_spacing.x = 18.0;
            ui.columns(columns, |column_uis| {
                for (index, app) in filtered.iter().enumerate() {
                    let column = index % columns;
                    self.classic_app_card(&mut column_uis[column], *app);
                    column_uis[column].add_space(18.0);
                }
            });
        });
    }





    fn toggle_project_favorite(&mut self, path: &Path) {
        if self.prefs.favorite_projects.iter().any(|p| p == path) { self.prefs.favorite_projects.retain(|p| p != path); }
        else { self.prefs.favorite_projects.push(path.to_path_buf()); }
        save_preferences(&self.prefs);
    }



    fn classic_app_detail(&mut self, ui: &mut egui::Ui, id: &'static str) {
        let Some(app) = app_by_id(id).copied() else {
            self.page = Page::Apps;
            return;
        };
        let state = self.states.get(app.id).cloned().unwrap_or_default();
        ui.horizontal(|ui| {
            if secondary_button(ui, if self.detail_parent == Page::YourApps { "Your apps" } else { "App Manager" }).clicked() {
                self.page = self.detail_parent;
            }
            ui.label(RichText::new(tr("/")).color(muted()));
            ui.label(RichText::new(tr(app.name)).strong().color(foreground()));
        });
        ui.add_space(12.0);
        egui::Frame::new()
            .fill(mix_color(card(), app.tint, 0.18))
            .stroke(egui::Stroke::new(1.0_f32, mix_color(border(), app.tint, 0.55)))
            .corner_radius(16)
            .inner_margin(22)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    self.app_logo(ui, &app, 72.0);
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tr(app.category)).size(text_size(11.0)).strong().color(readable_app_color(app.tint)));
                        ui.label(RichText::new(tr(app.name)).size(text_size(27.0)).strong().color(foreground()));
                        ui.label(RichText::new(tr(app.blurb)).size(text_size(13.0)).color(muted()));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let status = if state.busy.is_some() {
                            "Working"
                        } else if !app.has_release && state.installed.is_none() {
                            "Coming soon"
                        } else if state.installed.is_some() {
                            "Installed"
                        } else {
                            "Not installed"
                        };
                        ui.label(RichText::new(tr(status)).size(text_size(12.0)).color(if state.installed.is_some() {
                            theme_rgb(108, 211, 153)
                        } else {
                            muted()
                        }));
                    });
                });
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    let installed = state.installed.as_deref().map(|v| format!("Installed version: {v}"))
                        .unwrap_or_else(|| "Not installed on this device".into());
                    let latest = if !app.has_release && state.installed.is_none() { "No compatible release yet".to_owned() } else {
                        state.latest.as_deref().map(|v| format!("Latest release: {v}"))
                            .unwrap_or_else(|| "Latest release is checked when you install".into())
                    };
                    ui.label(RichText::new(tr(installed)).size(text_size(12.0)).color(muted()));
                    ui.separator();
                    ui.label(RichText::new(tr(latest)).size(text_size(12.0)).color(muted()));
                });
                if let Some(error) = &state.error {
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr(error)).size(text_size(12.0)).color(theme_rgb(255, 156, 135)));
                }
                if let Some(progress) = &state.busy {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new(tr(progress)).size(text_size(12.0)).color(muted()));
                    });
                }
                ui.add_space(13.0);
                ui.horizontal(|ui| {
                    if state.busy.is_none() {
                        if state.installed.is_some() {
                            if app_primary_button(ui, "Open app", app.tint).clicked() {
                                self.launch(app, None);
                            }
                            if state.latest.is_some() && state.latest != state.installed {
                                if icon_button(ui, ButtonIcon::UpdateArrow, "Install update", app.tint).clicked() {
                                    self.install(app);
                                }
                            }
                            if icon_button(ui, ButtonIcon::Delete, "Uninstall app", theme_rgb(213, 83, 93)).clicked() {
                                self.show_remove = Some(app.id.to_owned());
                            }
                        } else if app.has_release {
                            if app_primary_button(ui, "Install app", app.tint).clicked() {
                                self.install(app);
                            }
                        } else {
                            ui.add_enabled(false, egui::Button::new(tr("No compatible release yet")));
                        }
                        if icon_button(ui, ButtonIcon::Github, "Open source repository on GitHub", muted()).clicked() {
                            open_url(&format!("{REPO}/{}", release_slug(app.id)));
                        }
                        if icon_button(ui, ButtonIcon::Globe, "Open the ArtCraft app website", muted()).clicked() {
                            open_url("https://getartcraft.com/apps");
                        }
                        if icon_button(ui, ButtonIcon::UpdateArrow, "Check for updates", app.tint).clicked() {
                            self.check_app_release(app);
                        }
                    }
                });
            });

        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Recent projects")).size(text_size(17.0)).strong().color(foreground()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.link(tr("View all projects")).clicked() {
                    self.project_filter = app.name.to_owned();
                    self.page = Page::Projects;
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Open a recent file directly in this app.")).size(text_size(12.0)).color(muted()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.allocate_ui_with_layout(Vec2::new(116.0, 36.0), egui::Layout::left_to_right(egui::Align::Center), |ui| self.project_view_controls(ui));
            });
        });
        ui.add_space(8.0);
        let recent: Vec<Project> = self.projects.iter().filter(|p| p.app.id == app.id).take(5).cloned().collect();
        if recent.is_empty() {
            egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(12).inner_margin(18).show(ui, |ui| {
                ui.label(RichText::new(tr(format!("No {} projects found yet.", app.name))).color(muted()));
                if ui.button(tr("Choose project folders")).clicked() {
                    self.page = Page::Projects;
                }
            });
        } else {
            self.project_gallery(ui, recent);
        }
        ui.add_space(18.0);
        ui.label(RichText::new(tr("Supported file types")).size(text_size(15.0)).strong().color(foreground()));
        let formats = app.filetypes.iter().map(|extension| format!(".{extension}")).collect::<Vec<_>>().join("   ");
        ui.label(RichText::new(tr(formats)).size(text_size(12.0)).color(muted()));
    }

    fn empty_projects(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(14).inner_margin(26).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::hover());
                paint_navigation_icon(&ui.painter(), icon_rect, Page::Projects, ACCENT);
                ui.label(RichText::new(tr("Your projects live here.")).size(text_size(17.0)).strong());
                ui.label(RichText::new(tr("Choose a folder to create an app folder for each tool. Projects saved there are found automatically.")).size(text_size(12.0)).color(muted()));
                ui.add_space(7.0);
                if primary_button(ui, "Add a project folder").clicked() { if let Some(path) = rfd::FileDialog::new().pick_folder() { self.add_project_folder(path); } }
            });
        });
    }

    fn project_rows(&mut self, ui: &mut egui::Ui, projects: Vec<Project>) {
        egui::ScrollArea::vertical().show(ui, |ui| self.project_gallery(ui, projects));
    }

    fn project_view_controls(&mut self, ui: &mut egui::Ui) {
        for (view, icon, label) in [
            (ProjectView::List, ButtonIcon::List, "List"),
            (ProjectView::Grid, ButtonIcon::Grid, "Grid"),
            (ProjectView::Waterfall, ButtonIcon::Waterfall, "Waterfall"),
        ] {
            if icon_toggle_button(ui, icon, label, self.prefs.project_view == view).clicked() {
                self.prefs.project_view = view;
                save_preferences(&self.prefs);
            }
        }
    }



    fn project_gallery(&mut self, ui: &mut egui::Ui, projects: Vec<Project>) {
        let view = self.prefs.project_view;
        match view {
                ProjectView::List => for project in projects {
                    self.project_list_row(ui, &project);
                    ui.add_space(3.0);
                },
                ProjectView::Grid => {
                    let columns = project_column_count(ui.available_width());
                    ui.columns(columns, |cols| for (index, project) in projects.iter().enumerate() {
                        self.project_tile(&mut cols[index % columns], project, false);
                        cols[index % columns].add_space(10.0);
                    });
                }
                ProjectView::Waterfall => {
                    let columns = project_column_count(ui.available_width());
                    let mut buckets: Vec<Vec<&Project>> = vec![Vec::new(); columns];
                    let mut heights = vec![0.0_f32; columns];
                    for project in &projects {
                        let index = heights.iter().enumerate().min_by(|a,b| a.1.total_cmp(b.1)).map(|x|x.0).unwrap_or(0);
                        let aspect = project.preview.as_ref().map(|p| p.width() as f32 / p.height().max(1) as f32).unwrap_or(1.5);
                        let thumb_h = (available_tile_width(ui.available_width(), columns) / aspect).clamp(88.0, 178.0);
                        heights[index] += thumb_h + 142.0;
                        buckets[index].push(project);
                    }
                    ui.columns(columns, |cols| for (index, bucket) in buckets.iter().enumerate() {
                        for project in bucket {
                            self.project_tile(&mut cols[index], project, true);
                            cols[index].add_space(10.0);
                        }
                    });
                }
        }
    }




}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.applied_light_mode != Some(self.prefs.light_mode) {
            apply_theme(ctx, self.prefs.light_mode);
            self.applied_light_mode = Some(self.prefs.light_mode);
        }
        update_scrollbar_visibility(ctx);
        #[cfg(target_os = "windows")]
        {
            let needs_tray = self.prefs.minimize_to_tray || self.prefs.start_with_windows;
            if needs_tray && self.tray.is_none() && !self.tray_failed {
                match windows_tray::Tray::new(frame, self.events_tx.clone(), ctx.clone(), self.prefs.minimize_to_tray) {
                    Ok(tray) => self.tray = Some(tray),
                    Err(error) => { self.tray_failed = true; self.toast = Some(error); }
                }
            }
            if let Some(tray) = self.tray.as_mut() { tray.set_minimize(self.prefs.minimize_to_tray); }
            if !needs_tray { self.tray = None; }
            if self.silent_start {
                self.silent_start = false;
                self.startup_splash_finished = true;
                self.startup_splash_initialized = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1240.0, 800.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(920.0, 640.0)));
                // Never hide the only window unless Windows has accepted the tray icon.
                if self.tray.is_none() { ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true)); }
            }
        }

        #[cfg(target_os="linux")]
        {
            let needed=self.prefs.minimize_to_tray||self.prefs.start_with_windows;
            if self.linux_tray.as_ref().is_some_and(|t|!t.available()){self.linux_tray=None;self.tray_failed=true;ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));}
            if needed&&self.linux_tray.is_none()&&!self.tray_failed{match linux_tray::Tray::new(self.events_tx.clone(),ctx.clone()){Ok(tray)=>self.linux_tray=Some(tray),Err(error)=>{self.tray_failed=true;self.toast=Some(error);}}}
            if !needed{if self.linux_tray.take().is_some(){ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));}}
            if self.silent_start{
                self.silent_start=false;self.startup_splash_finished=true;self.startup_splash_initialized=true;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1240.0,800.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(920.0,640.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.linux_tray.is_none()));
            }
            if self.prefs.minimize_to_tray&&self.linux_tray.is_some()&&ctx.input(|i|i.viewport().minimized==Some(true)){ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));}
        }
        #[cfg(target_os="macos")]
        {
            let needed=self.prefs.minimize_to_tray||self.prefs.start_with_windows;
            if needed&&self.mac_tray.is_none()&&!self.tray_failed{match macos::Tray::new(self.events_tx.clone(),ctx.clone()){Ok(tray)=>self.mac_tray=Some(tray),Err(error)=>{self.tray_failed=true;self.toast=Some(error);}}}
            if !needed&&self.mac_tray.take().is_some(){ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));}
            if self.silent_start{
                self.silent_start=false;self.startup_splash_finished=true;self.startup_splash_initialized=true;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1240.0,800.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(920.0,640.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.mac_tray.is_none()));
            }
            if self.prefs.minimize_to_tray&&self.mac_tray.is_some()&&ctx.input(|i|i.viewport().minimized==Some(true)){ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));}
        }
        if !self.window_rounding_applied {
            #[cfg(target_os = "windows")]
            apply_window_rounding(frame);
            self.window_rounding_applied = true;
        }
        if !self.startup_splash_initialized {
            self.startup_splash_started = Instant::now();
            self.startup_splash_initialized = true;
        }
        self.process_events();
        if self.tray_quit_requested {
            #[cfg(target_os = "windows")]
            { self.tray = None; }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.tray_open_requested {
            self.tray_open_requested = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if self.prefs.automatic_suite_updates && self.startup_splash_finished { self.check_suite_update(false); }
        let suite_can_restart = self.prefs.onboarding_complete && self.startup_splash_finished && self.pending_launch.is_none()
            && self.show_project_rename.is_none() && self.show_project_delete.is_none() && self.show_remove.is_none()
            && !self.prefs.scanning && self.states.values().all(|state| state.busy.is_none());
        let suite_can_restart = suite_can_restart && self.properties.is_none();
        if let Some((_, ready_at)) = self.suite_update_ready.as_mut() {
            if !suite_can_restart || !self.prefs.automatic_suite_updates { *ready_at = Instant::now(); }
        }
        if self.prefs.automatic_suite_updates && suite_can_restart && self.suite_update_ready.as_ref().is_some_and(|(_,at)|at.elapsed() >= Duration::from_secs(10)) {
            if let Some((update, _)) = self.suite_update_ready.take() {
                save_preferences(&self.prefs);
                match suite_update::launch(&update) {
                    Ok(()) => { self.tray_quit_requested = true; ctx.send_viewport_cmd(egui::ViewportCommand::Close); return; }
                    Err(error) => { self.suite_update_status = error.clone(); self.toast = Some(error); }
                }
            }
        }
        if self.prefs.automatic_updates { self.check_releases(); }
        // Watch known files independently of the slower folder-discovery interval.
        if !self.preview_check_busy && !self.prefs.scanning && self.last_preview_check.elapsed() >= Duration::from_secs(2) {
            self.last_preview_check = Instant::now();
            self.preview_check_busy = true;
            let files: Vec<_> = self.projects.iter().map(|p|(p.path.clone(),p.revision)).collect();
            let retry = self.projects.iter().any(|p|p.preview.is_none()) && self.last_project_scan.elapsed() >= Duration::from_secs(15);
            let tx = self.events_tx.clone(); let repaint = ctx.clone();
            thread::spawn(move || {
                let changed = retry || files.iter().any(|(path,revision)|project_revision(path) != *revision);
                let _ = tx.send(Event::PreviewChanges(changed)); repaint.request_repaint();
            });
        }
        ctx.request_repaint_after(Duration::from_secs(2));
        if self.prefs.automatic_project_scan && self.last_project_scan.elapsed() >= Duration::from_secs(self.prefs.project_scan_minutes.clamp(1, 60) * 60) {
            self.scan_projects();
        }
        self.load_logos(ctx);
        if self.startup_splash_finished {
        egui::TopBottomPanel::top("custom_titlebar").exact_height(42.0).show_separator_line(false).frame(egui::Frame::new().fill(theme_rgb(25, 25, 30)).inner_margin(egui::Margin::symmetric(12, 4))).show(ctx, |ui| {
            let (bar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), egui::Sense::hover());
            let icon_rect = egui::Rect::from_min_size(bar.left_center() + Vec2::new(0.0, -11.0), Vec2::splat(22.0));
            ui.painter().image(self.brand_icon.id(), icon_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            let title_rect = ui.painter().text(icon_rect.right_center() + Vec2::new(10.0, 0.0), egui::Align2::LEFT_CENTER,tr("ArtCraft Master Suite"), egui::FontId::proportional(text_size(12.0)), theme_rgb(222, 223, 228));
            let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            let beta = egui::Rect::from_min_size(egui::pos2(title_rect.right() + 10.0, bar.center().y - 10.5), Vec2::new(48.0, 21.0));
            ui.painter().rect_filled(beta, 6.0, mix_color(panel(), ACCENT, 0.18));
            ui.painter().text(beta.center(), egui::Align2::CENTER_CENTER, tr("Beta"), egui::FontId::proportional(text_size(10.0)), readable_app_color(ACCENT));
            let controls_width = 138.0;
            let drag_rect = egui::Rect::from_min_max(bar.left_top(), egui::pos2(bar.right() - controls_width, bar.bottom()));
            let drag = ui.interact(drag_rect, egui::Id::new("custom-titlebar-drag"), egui::Sense::click_and_drag());
            if drag.drag_started() {
                #[cfg(target_os = "windows")]
                begin_native_titlebar_drag(frame);
                #[cfg(not(target_os = "windows"))]
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized)); }
            let start = bar.right() - controls_width;
            for (idx, command) in [0_u8, 1_u8, 2_u8].into_iter().enumerate() {
                let rect = egui::Rect::from_min_size(egui::pos2(start + idx as f32 * 46.0, bar.top()), Vec2::new(46.0, 34.0));
                let response = ui.interact(rect, egui::Id::new(("window-control", idx)), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                if response.hovered() { ui.painter().rect_filled(rect.shrink2(Vec2::new(2.0, 1.0)), 7.0, if command == 2 { theme_rgb(177, 58, 67) } else { theme_rgb(49, 50, 58) }); }
                let c = rect.center();
                let stroke = egui::Stroke::new(1.4_f32, theme_rgb(220, 221, 226));
                match command {
                    0 => { ui.painter().line_segment([c + Vec2::new(-5.0, 3.0), c + Vec2::new(5.0, 3.0)], stroke); }
                    1 if maximized => {
                        ui.painter().rect_stroke(egui::Rect::from_center_size(c + Vec2::new(1.5, -1.5), Vec2::new(9.0, 8.0)), 0.5, stroke, egui::StrokeKind::Inside);
                        ui.painter().rect_stroke(egui::Rect::from_center_size(c + Vec2::new(-1.5, 1.5), Vec2::new(9.0, 8.0)), 0.5, stroke, egui::StrokeKind::Inside);
                    }
                    1 => { ui.painter().rect_stroke(egui::Rect::from_center_size(c, Vec2::new(10.0, 9.0)), 0.5, stroke, egui::StrokeKind::Inside); }
                    _ => {
                        ui.painter().line_segment([c + Vec2::new(-4.0, -4.0), c + Vec2::new(4.0, 4.0)], stroke);
                        ui.painter().line_segment([c + Vec2::new(4.0, -4.0), c + Vec2::new(-4.0, 4.0)], stroke);
                    }
                };
                if response.clicked() { match command { 0 => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)), 1 => ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized)), _ => ctx.send_viewport_cmd(egui::ViewportCommand::Close) } }
            }
        });
        }
        if !self.startup_splash_finished {
            let duration = Duration::from_secs(3);
            let progress = (self.startup_splash_started.elapsed().as_secs_f32() / duration.as_secs_f32()).min(1.0);
            if progress < 1.0 {
                self.draw_startup_splash(ctx, progress);
                if !self.startup_splash_window_shown {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    self.startup_splash_window_shown = true;
                }
                self.draw_resize_handles(ctx);
                ctx.request_repaint_after(Duration::from_millis(16));
                return;
            }
            self.startup_splash_finished = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1240.0, 800.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(920.0, 640.0)));
            ctx.request_repaint_after(Duration::from_millis(16));
            return;
        }
        if self.transitioned_page != self.page {
            if matches!(self.page, Page::App(_)) && !matches!(self.transitioned_page, Page::App(_)) {
                self.detail_parent = if self.transitioned_page == Page::Apps { Page::Apps } else { Page::YourApps };
            }
            self.transitioned_page = self.page;
            self.page_transition_started = Instant::now();
        }
        let transition_t = if self.prefs.reduce_motion { 1.0 } else { (self.page_transition_started.elapsed().as_secs_f32() / 0.24).clamp(0.0, 1.0) };
        let transition_ease = transition_t * transition_t * (3.0 - 2.0 * transition_t);
        if self.prefs.classic_sidebar { self.classic_sidebar(ctx); } else { self.modern_sidebar(ctx); }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(ink())
                    .inner_margin(egui::Margin { left: 28, right: 12, top: 28, bottom: 20 }),
            )
            .show(ctx, |ui| match self.page {
                _ => {
                    ui.add_space((1.0 - transition_ease) * 8.0);
                    ui.scope(|ui| {
                        ui.set_opacity(transition_ease);
                        let content_rect = {
                            let rect=ui.available_rect_before_wrap();
                            egui::Rect::from_min_size(rect.min+Vec2::new(((rect.width()-1360.0)*0.5).max(0.0),0.0),Vec2::new(rect.width().min(1360.0),rect.height()))
                        };
                        let mut content_ui=ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
                        let ui=&mut content_ui;
                        match self.page {
                            Page::Home => self.home(ui),
                            Page::Cloud => { egui::ScrollArea::vertical().id_salt("cloud-page").show(ui, |ui| self.cloud_page(ui)); },
                            Page::Apps => self.apps_page(ui),
                            Page::YourApps => self.your_apps_page(ui),
                            Page::Projects => self.projects_page(ui),
                            Page::Settings => { egui::ScrollArea::vertical().id_salt("settings-scroll").show(ui, |ui| { ui.set_width(ui.available_width().min(980.0)); self.settings_page(ui); }); },
                            Page::App(id) => {
                                egui::ScrollArea::vertical().id_salt(("app-workspace",id)).show(ui, |ui| self.app_detail(ui, id));
                            }
                        }
                    });
                    if transition_ease < 1.0 {
                        ctx.request_repaint_after(Duration::from_millis(16));
                    }
                }
            });
        self.plugins.tick();
        self.cloud.tick(&self.projects);
        if !self.prefs.onboarding_complete {
            self.onboarding(ctx);
            ctx.request_repaint_after(Duration::from_millis(200));
            return;
        }
        self.properties_dialog(ctx);
        self.project_dialogs(ctx);
        if self.toast != self.toast_last_message {
            self.toast_last_message = self.toast.clone();
            self.toast_started = self.toast.as_ref().map(|_| Instant::now());
        }
        let toast_is_persistent = self.toast.is_some() && self.toast == self.persistent_toast;
        if !toast_is_persistent && self.toast_started.is_some_and(|started| started.elapsed() >= Duration::from_secs(5)) {
            self.toast = None;
            self.toast_last_message = None;
            self.toast_started = None;
            self.persistent_toast = None;
        }
        if let Some(message) = self.toast.clone() {
            let is_update = toast_is_persistent;
            let is_error = message.to_lowercase().contains("could not") || message.to_lowercase().contains("failed") || message.to_lowercase().contains("error");
            let accent = if is_update { ACCENT } else if is_error { theme_rgb(235, 105, 112) } else { theme_rgb(102, 205, 151) };
            let title = if is_update { "Updates available" } else if is_error { "Something needs attention" } else { "ArtCraft Master Suite" };
            egui::Area::new(egui::Id::new("toast-notification"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-22.0, -22.0))
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(theme_rgb(31, 33, 40))
                        .stroke(egui::Stroke::new(1.0_f32, mix_color(border(), accent, 0.65)))
                        .corner_radius(13)
                        .shadow(egui::epaint::Shadow { offset: [0, 5], blur: 18, spread: 1, color: Color32::from_black_alpha(110) })
                        .inner_margin(egui::Margin::symmetric(14, 12))
                        .show(ui, |ui| {
                            ui.set_max_width(390.0);
                            ui.horizontal(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::hover());
                                ui.painter().circle_filled(icon_rect.center(), 15.0, accent.gamma_multiply(0.18));
                                let center = icon_rect.center();
                                let stroke = egui::Stroke::new(1.8_f32, accent);
                                if is_update {
                                    ui.painter().line_segment([center + Vec2::new(0.0, 6.0), center + Vec2::new(0.0, -5.0)], stroke);
                                    ui.painter().line_segment([center + Vec2::new(-4.0, -1.0), center + Vec2::new(0.0, -5.0)], stroke);
                                    ui.painter().line_segment([center + Vec2::new(4.0, -1.0), center + Vec2::new(0.0, -5.0)], stroke);
                                } else if is_error {
                                    ui.painter().line_segment([center + Vec2::new(0.0, -5.0), center + Vec2::new(0.0, 1.5)], stroke);
                                    ui.painter().circle_filled(center + Vec2::new(0.0, 5.0), 1.0, accent);
                                } else {
                                    ui.painter().line_segment([center + Vec2::new(-5.0, 0.0), center + Vec2::new(-1.0, 4.0)], stroke);
                                    ui.painter().line_segment([center + Vec2::new(-1.0, 4.0), center + Vec2::new(6.0, -5.0)], stroke);
                                }
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(title).size(text_size(12.0)).strong().color(foreground()));
                                    ui.add(egui::Label::new(RichText::new(tr(message)).size(text_size(11.0)).color(theme_rgb(188, 191, 200))).wrap());
                                });
                                if icon_button(ui, ButtonIcon::Close, "Dismiss notification", muted()).clicked() {
                                    self.toast = None;
                                    self.toast_last_message = None;
                                    self.toast_started = None;
                                    self.persistent_toast = None;
                                }
                            });
                        });
                });
        }
        self.draw_manual_update_dialog(ctx);
        self.draw_launch_splash(ctx);
        self.draw_resize_handles(ctx);
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

fn paint_navigation_icon(painter: &egui::Painter, rect: egui::Rect, page: Page, color: Color32) {
    let center = rect.center();
    let stroke = egui::Stroke::new(1.5_f32, color);
    let line = |a, b| painter.line_segment([a, b], stroke);
    match page {
        Page::Home => {
            line(rect.left_top() + Vec2::new(2.0, 7.0), center - Vec2::new(0.0, 5.0));
            line(center - Vec2::new(0.0, 5.0), rect.right_top() - Vec2::new(2.0, -7.0));
            line(rect.left_top() + Vec2::new(4.0, 6.0), rect.left_bottom() + Vec2::new(4.0, -2.0));
            line(rect.right_top() - Vec2::new(4.0, -6.0), rect.right_bottom() - Vec2::new(4.0, 2.0));
            line(rect.left_bottom() + Vec2::new(4.0, -2.0), rect.right_bottom() - Vec2::new(4.0, 2.0));
        }
        Page::Cloud => { cloud_ui::paint_cloud(painter, rect, color); }
        Page::Apps | Page::YourApps => {
            let s = 4.5;
            for offset in [Vec2::new(-3.0, -3.0), Vec2::new(3.0, -3.0), Vec2::new(-3.0, 3.0), Vec2::new(3.0, 3.0)] {
                let c = center + offset;
                painter.rect_stroke(egui::Rect::from_center_size(c, Vec2::splat(s)), 1.0, stroke, egui::StrokeKind::Inside);
            }
        }
        Page::Projects => {
            let left = rect.left() + 3.0;
            let right = rect.right() - 3.0;
            let top = rect.top() + 2.0;
            let bottom = rect.bottom() - 2.0;
            line(egui::pos2(left, top), egui::pos2(right - 3.0, top));
            line(egui::pos2(right - 3.0, top), egui::pos2(right, top + 3.0));
            line(egui::pos2(right, top + 3.0), egui::pos2(right, bottom));
            line(egui::pos2(right, bottom), egui::pos2(left, bottom));
            line(egui::pos2(left, bottom), egui::pos2(left, top));
            line(egui::pos2(left + 3.0, center.y), egui::pos2(right - 3.0, center.y));
            line(egui::pos2(left + 3.0, center.y + 3.0), egui::pos2(right - 3.0, center.y + 3.0));
        }
        Page::Settings => {
            painter.circle_stroke(center, 4.1, stroke);
            painter.circle_stroke(center, 1.7, stroke);
            for index in 0..8 {
                let angle = std::f32::consts::TAU * index as f32 / 8.0;
                let direction = Vec2::new(angle.cos(), angle.sin());
                line(center + direction * 5.0, center + direction * 7.2);
            }
        }
        Page::App(_) => {
            let s = 4.5;
            for offset in [Vec2::new(-3.0, -3.0), Vec2::new(3.0, -3.0), Vec2::new(-3.0, 3.0), Vec2::new(3.0, 3.0)] {
                painter.rect_stroke(egui::Rect::from_center_size(center + offset, Vec2::splat(s)), 1.0, stroke, egui::StrokeKind::Inside);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum ButtonIcon {
    Folder,
    Favorite,
    Rename,
    Delete,
    Refresh,
    UpdateArrow,
    Close,
    List,
    Grid,
    Waterfall,
    Help,
    Github,
    Globe,
}

fn icon_button(
    ui: &mut egui::Ui,
    icon: ButtonIcon,
    tooltip: &str,
    tint: Color32,
) -> egui::Response {
    icon_button_sized(ui, icon, tooltip, tint, 32.0)
}

fn icon_button_sized(
    ui: &mut egui::Ui,
    icon: ButtonIcon,
    tooltip: &str,
    tint: Color32,
    size: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());
    let response = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tr(tooltip));
    let hover_t = ui.ctx().animate_bool(response.id, response.hovered());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        7.0,
        mix_color(Color32::TRANSPARENT, ui.visuals().widgets.hovered.bg_fill, hover_t),
    );
    let center = rect.center();
    let stroke = egui::Stroke::new(1.8_f32, mix_color(tint, Color32::WHITE, hover_t));
    let line = |from, to| painter.line_segment([from, to], stroke);
    match icon {
        ButtonIcon::Folder => {
            let points = vec![center + Vec2::new(-7.0,-5.0), center + Vec2::new(-2.0,-5.0), center + Vec2::new(0.0,-3.0), center + Vec2::new(7.0,-3.0), center + Vec2::new(7.0,6.0), center + Vec2::new(-7.0,6.0)];
            painter.add(egui::Shape::closed_line(points, stroke));
            line(center + Vec2::new(-7.0,-1.0), center + Vec2::new(7.0,-1.0));
        }
        ButtonIcon::Favorite => {
            let points: Vec<_> = (0..10).map(|index| {
                let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::PI / 5.0;
                center + Vec2::new(angle.cos(), angle.sin()) * if index % 2 == 0 { 7.0 } else { 3.2 }
            }).collect();
            painter.add(egui::Shape::closed_line(points, stroke));
        }
        ButtonIcon::Rename => {
            let body = [
                center + Vec2::new(-6.0, 4.2), center + Vec2::new(-4.2, 6.0),
                center + Vec2::new(5.1, -3.3), center + Vec2::new(3.3, -5.1),
            ];
            painter.add(egui::Shape::closed_line(body.to_vec(), stroke));
            line(center + Vec2::new(-6.0, 4.2), center + Vec2::new(-6.7, 6.7));
            line(center + Vec2::new(-6.7, 6.7), center + Vec2::new(-4.2, 6.0));
            line(center + Vec2::new(3.3, -5.1), center + Vec2::new(4.8, -6.6));
            line(center + Vec2::new(4.8, -6.6), center + Vec2::new(6.6, -4.8));
            line(center + Vec2::new(6.6, -4.8), center + Vec2::new(5.1, -3.3));
        }
        ButtonIcon::Delete => {
            line(center + Vec2::new(-6.0, -5.0), center + Vec2::new(6.0, -5.0));
            line(center + Vec2::new(-2.5, -8.0), center + Vec2::new(2.5, -8.0));
            line(center + Vec2::new(-4.5, -3.0), center + Vec2::new(-3.5, 6.0));
            line(center + Vec2::new(4.5, -3.0), center + Vec2::new(3.5, 6.0));
            line(center + Vec2::new(-3.5, 6.0), center + Vec2::new(3.5, 6.0));
            line(center + Vec2::new(-1.5, -2.0), center + Vec2::new(-1.5, 4.0));
            line(center + Vec2::new(1.5, -2.0), center + Vec2::new(1.5, 4.0));
        }
        ButtonIcon::Refresh => {
            windows_ui::toolbar::paint_refresh(painter,center,stroke.color);
        }
        ButtonIcon::UpdateArrow => {
            line(center + Vec2::new(0.0, 7.0), center + Vec2::new(0.0, -6.0));
            line(center + Vec2::new(-5.0, -1.0), center + Vec2::new(0.0, -6.0));
            line(center + Vec2::new(5.0, -1.0), center + Vec2::new(0.0, -6.0));
            line(center + Vec2::new(-6.0, 7.0), center + Vec2::new(6.0, 7.0));
        }
        ButtonIcon::Close => {
            line(center + Vec2::new(-4.5, -4.5), center + Vec2::new(4.5, 4.5));
            line(center + Vec2::new(4.5, -4.5), center + Vec2::new(-4.5, 4.5));
        }
        ButtonIcon::List => {
            for y in [-5.0, 0.0, 5.0] { line(center + Vec2::new(-5.0, y), center + Vec2::new(6.0, y)); }
        }
        ButtonIcon::Grid => {
            for offset in [Vec2::new(-3.2,-3.2), Vec2::new(3.2,-3.2), Vec2::new(-3.2,3.2), Vec2::new(3.2,3.2)] {
                painter.rect_stroke(egui::Rect::from_center_size(center + offset, Vec2::splat(4.0)), 0, stroke, egui::StrokeKind::Inside);
            }
        }
        ButtonIcon::Waterfall => {
            for (x, h) in [(-5.0, 7.0), (0.0, 11.0), (5.0, 8.0)] {
                painter.rect_stroke(egui::Rect::from_min_size(center + Vec2::new(x - 1.5, -h / 2.0), Vec2::new(3.0, h)), 1.0, stroke, egui::StrokeKind::Inside);
            }
        }
        ButtonIcon::Help => {
            painter.circle_stroke(center, 6.5, stroke);
            painter.text(center + Vec2::new(0.0, -0.5), egui::Align2::CENTER_CENTER,tr("?"), egui::FontId::proportional(text_size(12.0)), stroke.color);
            painter.circle_filled(center + Vec2::new(0.0, 4.5), 0.7, stroke.color);
        }
        ButtonIcon::Github => {
            let color = stroke.color;
            let head = vec![
                center + Vec2::new(-5.5, -1.0), center + Vec2::new(-6.0, -6.0),
                center + Vec2::new(-2.0, -4.6), center + Vec2::new(2.0, -4.6),
                center + Vec2::new(6.0, -6.0), center + Vec2::new(5.5, -1.0),
                center + Vec2::new(5.2, 2.5), center + Vec2::new(3.2, 5.2),
                center + Vec2::new(-3.2, 5.2), center + Vec2::new(-5.2, 2.5),
            ];
            painter.add(egui::Shape::convex_polygon(head, color, egui::Stroke::NONE));
            painter.circle_filled(center + Vec2::new(-2.0, 0.0), 0.65, ink());
            painter.circle_filled(center + Vec2::new(2.0, 0.0), 0.65, ink());
            line(center + Vec2::new(-2.2, 4.8), center + Vec2::new(-2.2, 7.0));
            line(center + Vec2::new(2.2, 4.8), center + Vec2::new(2.2, 7.0));
        }
        ButtonIcon::Globe => {
            painter.circle_stroke(center, 7.0, stroke);
            line(center + Vec2::new(-6.0, 0.0), center + Vec2::new(6.0, 0.0));
            line(center + Vec2::new(0.0, -6.0), center + Vec2::new(0.0, 6.0));
            for side in [-1.0, 1.0] {
                let pts = vec![
                    center + Vec2::new(0.0, -6.6),
                    center + Vec2::new(side * 3.2, -3.2),
                    center + Vec2::new(side * 3.8, 0.0),
                    center + Vec2::new(side * 3.2, 3.2),
                    center + Vec2::new(0.0, 6.6),
                ];
                painter.add(egui::Shape::line(pts, stroke));
            }
        }
    }
    response
}

fn icon_toggle_button(ui: &mut egui::Ui, icon: ButtonIcon, tooltip: &str, selected: bool) -> egui::Response {
    let tint = if selected { foreground() } else { muted() };
    let response = icon_button(ui, icon, tooltip, tint);
    if selected {
        ui.painter().rect_stroke(response.rect, 7.0, egui::Stroke::new(1.0_f32, ACCENT), egui::StrokeKind::Inside);
    }
    response
}

fn available_tile_width(total: f32, columns: usize) -> f32 {
    ((total - 12.0 * columns.saturating_sub(1) as f32) / columns.max(1) as f32 - 24.0).max(120.0)
}

fn project_column_count(width: f32) -> usize {
    ((width / 350.0).floor() as usize).clamp(1, 5)
}

fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    polished_button(ui, text, ACCENT, theme_rgb(155, 130, 255), Vec2::new(112.0, 36.0))
}

fn app_primary_button(ui: &mut egui::Ui, text: &str, tint: Color32) -> egui::Response {
    polished_button(ui, text, tint, mix_color(tint, Color32::WHITE, 0.18), Vec2::new(112.0, 36.0))
}

fn danger_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    polished_button(ui, text, theme_rgb(157, 48, 60), theme_rgb(194, 61, 74), Vec2::new(86.0, 36.0))
}

fn polished_button(ui: &mut egui::Ui, text: &str, base: Color32, hover: Color32, minimum: Vec2) -> egui::Response {
    let font = egui::FontId::proportional(text_size(13.0));
    let luminance = (0.2126 * base.r() as f32 + 0.7152 * base.g() as f32 + 0.0722 * base.b() as f32) / 255.0;
    let text_color = if luminance > 0.62 { Color32::from_rgb(18, 19, 22) } else { Color32::WHITE };
    let galley = ui.painter().layout_no_wrap(tr(text.to_owned()), font, text_color);
    let desired = Vec2::new((galley.size().x + 28.0).max(minimum.x), (galley.size().y + 16.0).max(minimum.y));
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover_t = ui.ctx().animate_bool(response.id, response.hovered());
    let down = response.is_pointer_button_down_on();
    let fill = mix_color(base, hover, hover_t);
    let painter = ui.painter();
    if !down {
        painter.rect_filled(rect.translate(Vec2::new(0.0, 2.0)), 9.0, Color32::from_black_alpha(34));
    }
    painter.rect_filled(rect.translate(if down { Vec2::new(0.0, 1.0) } else { Vec2::ZERO }), 9.0, fill);
    painter.rect_stroke(rect, 9.0, egui::Stroke::new(1.0_f32, mix_color(base, Color32::WHITE, hover_t * 0.28)), egui::StrokeKind::Inside);
    let text_pos = rect.center() - galley.size() * 0.5 + if down { Vec2::new(0.0, 1.0) } else { Vec2::ZERO };
    painter.galley(text_pos, galley, text_color);
    response
}

fn mix_color(base: Color32, tint: Color32, amount: f32) -> Color32 {
    let mix = |a: u8, b: u8| (a as f32 * (1.0 - amount) + b as f32 * amount).round() as u8;
    Color32::from_rgb(
        mix(base.r(), tint.r()),
        mix(base.g(), tint.g()),
        mix(base.b(), tint.b()),
    ).gamma_multiply(mix(base.a(), tint.a()) as f32 / 255.0)
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .user_agent(format!("ArtCraftLauncher/{VERSION}"))
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| e.to_string())
}

struct ReleaseInfo {
    version: String,
    package_name: String,
    package_url: String,
    checksums_url: String,
}

// Local IDs stay stable across upstream renames to preserve existing installs.
fn release_slug(id: &str) -> &str {
    match id { "printcraft" => "pdfcraft", _ => id }
}

fn latest_release(client: &Client, id: &str) -> Result<ReleaseInfo, String> {
    let slug = release_slug(id);
    // Select only a published GUI asset for this OS/architecture; never invent URLs.
    let release = windows_releases::latest_app(client, slug)?;
    let tag=release["tag_name"].as_str().ok_or("Release tag is missing")?;
    let version=tag.trim_start_matches('v').to_owned();
    let package_name=platform::package_name(slug,&version)?;
    let assets=release["assets"].as_array().ok_or("Release assets are missing")?;
    let asset=assets.iter().find(|a|a["name"].as_str()==Some(&package_name))
        .ok_or_else(||format!("No compatible {} release is published for {} ({package_name}).",platform::label(),slug))?;
    let checksums=assets.iter().find(|a|a["name"].as_str()==Some("SHA256SUMS.txt")).ok_or("Official release checksum file is missing")?;
    let release_url=format!("https://github.com/storytold/{slug}/releases/download/");
    let package_url=asset["browser_download_url"].as_str().filter(|u|u.starts_with(&release_url)).ok_or("Invalid release download URL")?.to_owned();
    let checksums_url=checksums["browser_download_url"].as_str().filter(|u|u.starts_with(&release_url)).ok_or("Invalid checksum URL")?.to_owned();

    Ok(ReleaseInfo {
        version,
        package_url,
        package_name,
        checksums_url,
    })
}

fn install_release(id: &str, tx: &Sender<Event>) -> Result<String, String> {
    let client = http_client()?;
    let release = latest_release(&client, id)?;
    let tx_progress = tx.clone();
    let _ = tx_progress.send(Event::Progress(
        id.to_owned(),
        format!("Downloading {}...", release.package_name),
    ));
    let mut response = client
        .get(&release.package_url)
        .send()
        .map_err(|e| format!("Download failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Download failed: {e}"))?;
    let total = response
        .content_length()
        .unwrap_or(0)
        .min(512 * 1024 * 1024);
    let mut bytes = Vec::with_capacity(total as usize);
    let mut chunk = [0_u8; 64 * 1024];
    let mut last_percent = 0;
    loop {
        let count = response
            .read(&mut chunk)
            .map_err(|e| format!("Could not read the app package: {e}"))?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > 512 * 1024 * 1024 {
            return Err("The app package is larger than the 512 MB safety limit.".into());
        }
        if total > 0 {
            let percent = ((bytes.len() as u64 * 100 / total).min(100)) as u8;
            if percent >= last_percent + 5 || percent == 100 {
                let _ = tx.send(Event::Progress(
                    id.to_owned(),
                    format!("Downloading {}%...", percent),
                ));
                last_percent = percent;
            }
        }
    }
    if bytes.is_empty() {
        return Err("The release download was empty.".into());
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let checksums = client
        .get(&release.checksums_url)
        .send()
        .map_err(|error| format!("Could not download the official checksum file: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Could not download the official checksum file: {error}"))?
        .text()
        .map_err(|error| format!("Could not read the official checksum file: {error}"))?;
    let expected = checksums
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');
            (name == release.package_name)
                .then(|| hash.to_ascii_lowercase())
        })
        .ok_or_else(|| "GitHub's checksum file does not list this platform package.".to_owned())?;
    if digest != expected {
        return Err(
            "The app package failed its SHA-256 integrity check. The old installation was kept."
                .into(),
        );
    }
    let _ = tx.send(Event::Progress(
        id.to_owned(),
        "Unpacking app files...".into(),
    ));
    let target = installed_dir(id).ok_or("The application data folder was not found")?;
    let parent = target.parent().ok_or("Invalid installation path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(".{id}-installing"));
    if temp.exists() {
        fs::remove_dir_all(&temp).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
    if let Err(error)=platform::unpack(&bytes,&temp,id) { let _=fs::remove_dir_all(&temp);return Err(error); }
    if find_executable(&temp, id).is_none() {
        let _ = fs::remove_dir_all(&temp);
        return Err(
            "The downloaded package did not contain the expected app executable for this platform.".into(),
        );
    }
    let backup = parent.join(format!(".{id}-previous"));
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|e| e.to_string())?;
    }
    if target.exists() {
        fs::rename(&target, &backup)
            .map_err(|e| format!("Close the app before updating it: {e}"))?;
    }
    if let Err(error) = fs::rename(&temp, &target) {
        if backup.exists() {
            let _ = fs::rename(&backup, &target);
        }
        return Err(format!("Could not finish installing the app: {error}"));
    }
    if backup.exists() {
        let _ = fs::remove_dir_all(backup);
    }
    Ok(release.version)
}

fn app_data() -> Option<PathBuf> { platform::data_dir() }
fn installed_dir(id: &str) -> Option<PathBuf> { platform::install_dir(id) }
fn preferences_path() -> Option<PathBuf> {
    app_data().map(|p| p.join("preferences.json"))
}
fn read_preferences() -> Preferences {
    preferences_path()
        .and_then(|p| fs::read(p).ok())
        .and_then(|b| {
            let value: serde_json::Value = serde_json::from_slice(&b).ok()?;
            // Older preferences have no completion flag: show setup once for them too.
            serde_json::from_value(value).ok()
        })
        .unwrap_or_default()
}
fn save_preferences(prefs: &Preferences) {
    if let Some(path) = preferences_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_vec_pretty(prefs) {
            let _ = fs::write(path, data);
        }
    }
}
fn detect_install(app: &AppInfo) -> Option<String> {
    let path = installed_dir(app.id)?;
    if find_executable(&path, app.id).is_some() {
        Some("Unknown".into())
    } else {
        None
    }
}
fn find_executable(root: &Path, id: &str) -> Option<PathBuf> { platform::executable(root,id) }
fn app_by_id(id: &str) -> Option<&'static AppInfo> {
    APPS.iter().find(|app| app.id == id)
}

fn scan_roots(roots: &[PathBuf], previous: &HashMap<PathBuf, Project>) -> Vec<Project> {
    let mut seen = std::collections::HashSet::new();
    let mut projects = vec![];
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(root)
            .max_depth(7)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                let managed_resource = entry.path().join(".artcraft-backups").is_file() || ["plugins", "assets", "exports"].contains(&name.as_str())
                    && entry.path().parent().is_some_and(|parent| parent.join(".artcraft-suite.json").is_file());
                !entry.file_type().is_dir()
                    || (!managed_resource && !name.starts_with('.')
                        && ![
                            "node_modules",
                            "target",
                            "appdata",
                            "windows",
                            "program files",
                            ".git",
                        ]
                        .contains(&name.as_str()))
            })
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let Some(ext) = entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_lowercase)
            else {
                continue;
            };
            let Some(app) = APPS
                .iter()
                .find(|app| app.filetypes.contains(&ext.as_str()))
            else {
                continue;
            };
            let path = entry.path().to_path_buf();
            if !seen.insert(path.clone()) {
                continue;
            }
            let metadata = entry.metadata().ok();
            let size_bytes = metadata.as_ref().map_or(0, |m| m.len());
            let modified = metadata.as_ref().and_then(|m| m.modified().ok()).map(DateTime::<Local>::from);
            let title = entry.file_name().to_string_lossy().to_string();
            let revision = project_revision(&path);
            if let Some(old) = previous.get(&path).filter(|p| p.revision == revision && p.preview.is_some()) {
                projects.push(old.clone());
                continue;
            }
            let native = if size_bytes <= 30 * 1024 * 1024 && matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "tif" | "tiff") {
                image::open(&path).ok().map(|image| image.thumbnail(640, 400).to_rgba8())
            } else if size_bytes <= 2 * 1024 * 1024 * 1024 && matches!(ext.as_str(), "psd" | "psb") {
                project_preview::psd_canvas(&path).or_else(|| psd_thumbnail(&path))
            } else { None };
            let mut decoded = native.map(|image| (image, "Saved canvas preview"))
                .or_else(|| preview_formats::load(&path, &ext));
            if project_revision(&path) != revision { decoded = None; }
            let preview_note = decoded.as_ref().map_or("Preview unavailable for this file", |(_, note)| *note);
            let preview = decoded.map(|(image, _)| Arc::new(image));
            projects.push(Project {
                revision,
                path,
                title,
                app,
                modified,
                size_bytes,
                preview,
                preview_note,
            });
        }
    }
    projects.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    projects.truncate(400);
    projects
}

fn psd_thumbnail(path: &Path) -> Option<image::RgbaImage> {
    let mut file = fs::File::open(path).ok()?;
    let mut header = [0u8; 26];
    file.read_exact(&mut header).ok()?;
    if &header[..4] != b"8BPS" { return None; }
    let mut length = [0u8; 4];
    file.read_exact(&mut length).ok()?;
    file.seek(SeekFrom::Current(u32::from_be_bytes(length) as i64)).ok()?;
    file.read_exact(&mut length).ok()?;
    let resource_len = u32::from_be_bytes(length) as usize;
    if resource_len > 64 * 1024 * 1024 { return None; }
    let mut resources = vec![0; resource_len];
    file.read_exact(&mut resources).ok()?;
    let mut cursor = Cursor::new(resources.as_slice());
    while (cursor.position() as usize).saturating_add(12) <= resources.len() {
        let mut signature = [0; 4];
        cursor.read_exact(&mut signature).ok()?;
        if &signature != b"8BIM" && &signature != b"8B64" { return None; }
        let mut field = [0; 4];
        cursor.read_exact(&mut field[..2]).ok()?;
        let id = u16::from_be_bytes([field[0], field[1]]);
        let mut n = [0; 1]; cursor.read_exact(&mut n).ok()?;
        let pascal_len = n[0] as usize + 1;
        cursor.seek(SeekFrom::Current((pascal_len - 1) as i64 + (pascal_len % 2) as i64)).ok()?;
        cursor.read_exact(&mut field).ok()?;
        let size = u32::from_be_bytes(field) as usize;
        if size > resources.len().saturating_sub(cursor.position() as usize) { return None; }
        if matches!(id, 1033 | 1036) && size >= 28 {
            let start = cursor.position() as usize;
            let block = &resources[start..start + size];
            let format = u32::from_be_bytes(block[0..4].try_into().ok()?);
            let width = u32::from_be_bytes(block[4..8].try_into().ok()?);
            let height = u32::from_be_bytes(block[8..12].try_into().ok()?);
            let data_len = u32::from_be_bytes(block[20..24].try_into().ok()?) as usize;
            if width == 0 || height == 0 || width > 8192 || height > 8192 || data_len > size.saturating_sub(28) { return None; }
            if format == 1 {
                return image::load_from_memory(&block[28..28 + data_len]).ok().map(|i| i.thumbnail(640,400).to_rgba8());
            }
        }
        cursor.seek(SeekFrom::Current(size as i64 + (size % 2) as i64)).ok()?;
    }
    None
}


fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 { return format!("{bytes} B"); }
    let units = ["KB", "MB", "GB", "TB"];
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < units.len() { value /= 1024.0; unit += 1; }
    format!("{value:.1} {}", units[unit])
}

fn open_url(url: &str) {
    #[cfg(target_os="macos")]
    { let _=Command::new("/usr/bin/open").arg(url).spawn(); }
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn();
    }
    #[cfg(target_os = "linux")]
    { let _ = platform::spawn(Command::new("xdg-open").arg(url)); }
}

fn main() -> eframe::Result {
    #[cfg(target_os = "windows")]
    let _instance = match windows_tray::single_instance(std::env::args().any(|arg| arg == "--tray")) {
        Some(instance) => instance,
        None => return Ok(()),
    };
    let icon = image::load_from_memory(include_bytes!("../assets/artcraft-icon.png"))
        .expect("embedded ArtCraft logo is a valid PNG")
        .to_rgba8();
    let icon = egui::IconData {
        width: icon.width(),
        height: icon.height(),
        rgba: icon.into_raw(),
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_resizable(true)
            .with_visible(false)
            .with_title("ArtCraft Master Suite")
            .with_icon(icon)
            .with_inner_size([700.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ArtCraft Master Suite",
        options,
        Box::new(|cc| Ok(Box::new(Launcher::new(cc)))),
    )
}

#[cfg(target_os = "windows")]
fn apply_window_rounding(frame: &eframe::Frame) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    if let Ok(window) = frame.window_handle() {
        if let RawWindowHandle::Win32(handle) = window.as_raw() {
            let preference: u32 = 2;
            unsafe {
                let hwnd = handle.hwnd.get() as windows_sys::Win32::Foundation::HWND;
                let _ = windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
                    hwnd,
                    windows_sys::Win32::Graphics::Dwm::DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                    (&preference as *const u32).cast(),
                    std::mem::size_of::<u32>() as u32,
                );
            }
        }
    }
}

fn settings_section(ui: &mut egui::Ui, title: &str, description: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32, border())).corner_radius(14).inner_margin(20).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(tr(title)).size(text_size(17.0)).strong());
        ui.label(RichText::new(tr(description)).size(text_size(12.0)).color(muted()));
        ui.add_space(10.0);
        content(ui);
    });
    ui.add_space(12.0);
}

fn setting_toggle(ui: &mut egui::Ui, title: &str, description: &str, value: &mut bool) {
    let width = ui.available_width();
    let text_width = (width - 68.0).max(80.0);
    let title_galley = ui.painter().layout(tr(title.to_owned()), egui::FontId::proportional(text_size(14.0)), foreground(), text_width);
    let description_galley = ui.painter().layout(tr(description.to_owned()), egui::FontId::proportional(text_size(12.0)), muted(), text_width);
    let text_height = title_galley.size().y + 5.0 + description_galley.size().y;
    let (row, _) = ui.allocate_exact_size(Vec2::new(width, (text_height + 16.0).max(56.0)), egui::Sense::hover());
    let top = row.center().y - text_height * 0.5;
    let description_y = top + title_galley.size().y + 5.0;
    ui.painter().galley(egui::pos2(row.left(), top), title_galley, foreground());
    ui.painter().galley(egui::pos2(row.left(), description_y), description_galley, muted());
    let rect = egui::Rect::from_center_size(egui::pos2(row.right() - 23.0, row.center().y), Vec2::new(42.0, 24.0));
    let response = ui.interact(rect, ui.id().with(title), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(title);
    if response.clicked() { *value = !*value; }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *value, title));
    ui.painter().rect_filled(rect, 12.0, if *value { ACCENT } else { border() });
    let x = if *value { rect.right() - 12.0 } else { rect.left() + 12.0 };
    ui.painter().circle_filled(egui::pos2(x, rect.center().y), 8.0, Color32::WHITE);
}

fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    polished_button(ui, text, theme_rgb(43, 46, 55), theme_rgb(58, 62, 74), Vec2::new(76.0, 36.0))
}

// Pick once per splash, so textures never shuffle during the animation.
fn random_orbit_apps(exclude: Option<&str>) -> Vec<&'static AppInfo> {
    use std::hash::BuildHasher;
    let random = std::collections::hash_map::RandomState::new();
    let mut apps: Vec<_> = APPS.iter().filter(|app| Some(app.id) != exclude).collect();
    apps.sort_by_key(|app| random.hash_one(app.id));
    apps.truncate(5);
    apps
}

#[cfg(target_os = "windows")]
fn begin_native_titlebar_drag(frame: &eframe::Frame) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{Foundation::{POINT, RECT, HWND}, UI::{Input::KeyboardAndMouse::ReleaseCapture, WindowsAndMessaging::*}};
    let Ok(window) = frame.window_handle() else { return; };
    let RawWindowHandle::Win32(handle) = window.as_raw() else { return; };
    unsafe {
        let hwnd = handle.hwnd.get() as HWND;
        let mut cursor = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut cursor) == 0 { return; }
        // Restore under the pointer before entering the native move loop. This also
        // avoids winit's cached dragging flag getting stuck on a maximized window.
        if IsZoomed(hwnd) != 0 {
            let mut before: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut before);
            let fraction = ((cursor.x - before.left) as f64 / (before.right - before.left).max(1) as f64).clamp(0.0, 1.0);
            let offset_y = (cursor.y - before.top).clamp(8, 36);
            ShowWindow(hwnd, SW_RESTORE);
            let mut restored: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut restored);
            SetWindowPos(hwnd, std::ptr::null_mut(), cursor.x - ((restored.right - restored.left) as f64 * fraction) as i32, cursor.y - offset_y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
        }
        ReleaseCapture();
        let position = ((cursor.y as u32 & 0xffff) << 16) | (cursor.x as u32 & 0xffff);
        PostMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, position as isize);
    }
}

fn dashboard_metric(ui: &mut egui::Ui, label: &str, value: &str, caption: &str, page: Page) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 112.0), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{label}: {value}. {caption}")));
    ui.painter().rect_filled(rect, 12.0, if response.hovered() { card() } else { panel() });
    ui.painter().rect_stroke(rect, 12.0, egui::Stroke::new(1.0_f32, if response.hovered() { theme_rgb(78, 70, 103) } else { border() }), egui::StrokeKind::Inside);
    ui.painter().text(rect.min + Vec2::new(18.0, 20.0), egui::Align2::LEFT_CENTER,tr(label), egui::FontId::proportional(text_size(10.0)), muted());
    ui.painter().text(rect.min + Vec2::new(18.0, 55.0), egui::Align2::LEFT_CENTER, value, egui::FontId::proportional(text_size(28.0)), foreground());
    ui.painter().text(rect.min + Vec2::new(18.0, 89.0), egui::Align2::LEFT_CENTER,tr(caption), egui::FontId::proportional(text_size(11.0)), muted());
    paint_navigation_icon(ui.painter(), egui::Rect::from_center_size(rect.right_top() + Vec2::new(-26.0, 53.0), Vec2::splat(22.0)), page, theme_rgb(169, 147, 238));
    response
}

static LIGHT_THEME: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn light_theme() -> bool { LIGHT_THEME.load(std::sync::atomic::Ordering::Relaxed) }
fn foreground() -> Color32 { if light_theme() { Color32::from_rgb(28, 34, 46) } else { Color32::WHITE } }
fn ink() -> Color32 { if light_theme() { Color32::from_rgb(245, 246, 249) } else { Color32::from_rgb(18, 19, 22) } }
fn panel() -> Color32 { if light_theme() { Color32::from_rgb(255, 255, 255) } else { Color32::from_rgb(27, 29, 34) } }
fn card() -> Color32 { if light_theme() { Color32::from_rgb(235, 238, 244) } else { Color32::from_rgb(34, 36, 42) } }
fn border() -> Color32 { if light_theme() { Color32::from_rgb(212, 218, 229) } else { Color32::from_rgb(46, 48, 56) } }
fn muted() -> Color32 { if light_theme() { Color32::from_rgb(92, 101, 118) } else { Color32::from_rgb(163, 167, 178) } }

// Keep hue in custom-painted neutral surfaces, while lifting their lightness.
fn theme_rgb(r: u8, g: u8, b: u8) -> Color32 {
    if !light_theme() { return Color32::from_rgb(r, g, b); }
    let average = (r as f32 + g as f32 + b as f32) / 3.0;
    if average < 115.0 {
        let base = 251.0 - average * 0.23;
        let channel = |v: u8| (base + (v as f32 - average) * 0.38).clamp(0.0, 255.0) as u8;
        Color32::from_rgb(channel(r), channel(g), channel(b))
    } else if average > 180.0 {
        let channel = |v: u8| (38.0 + (v as f32 - average) * 0.4).clamp(0.0, 255.0) as u8;
        Color32::from_rgb(channel(r), channel(g), channel(b))
    } else { Color32::from_rgb(r, g, b) }
}
fn apply_theme(ctx: &egui::Context, light: bool) {
    LIGHT_THEME.store(light, std::sync::atomic::Ordering::Relaxed);
    let mut style = (*ctx.style()).clone();
    style.visuals = if light { egui::Visuals::light() } else { egui::Visuals::dark() };
    style.visuals.window_fill = panel(); style.visuals.panel_fill = ink();
    style.visuals.extreme_bg_color = ink(); style.visuals.faint_bg_color = panel();
    style.visuals.override_text_color = Some(foreground());
    style.visuals.widgets.noninteractive.bg_fill = panel();
    style.visuals.widgets.inactive.bg_fill = card();
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, border());
    style.visuals.widgets.hovered.bg_fill = mix_color(card(), ACCENT, 0.12);
    style.visuals.widgets.active.bg_fill = mix_color(card(), ACCENT, 0.22);
    // Combo boxes and menu buttons use weak_bg_fill rather than bg_fill.
    style.visuals.widgets.inactive.weak_bg_fill = theme_rgb(43, 46, 55);
    style.visuals.widgets.hovered.weak_bg_fill = theme_rgb(58, 62, 74);
    style.visuals.widgets.active.weak_bg_fill = mix_color(card(), ACCENT, 0.22);
    style.visuals.widgets.open.bg_fill = card();
    style.visuals.widgets.open.weak_bg_fill = theme_rgb(43, 46, 55);
    style.visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT);
    style.visuals.widgets.open.corner_radius = 8.into();
    style.visuals.selection.bg_fill = mix_color(panel(), ACCENT, if light { 0.22 } else { 0.5 });
    for widget in [&mut style.visuals.widgets.inactive, &mut style.visuals.widgets.hovered, &mut style.visuals.widgets.active] { widget.corner_radius = 8.into(); }
    style.spacing.scroll = suite_scroll_style();
    windows_ui::refine_style(&mut style);
    ctx.set_style(style);
}

// Keep the gutter stable; visibility is driven by scrolling, not pointer hover.
fn update_scrollbar_visibility(ctx: &egui::Context) {
    let id = egui::Id::new("suite-last-scroll");
    let (now, scrolling, dragging) = ctx.input(|i| (
        i.time,
        i.raw_scroll_delta != Vec2::ZERO || i.smooth_scroll_delta != Vec2::ZERO
            || [egui::Key::PageUp, egui::Key::PageDown, egui::Key::Home, egui::Key::End,
                egui::Key::ArrowUp, egui::Key::ArrowDown].iter().any(|key| i.key_pressed(*key)),
        i.pointer.primary_down() && i.pointer.delta() != Vec2::ZERO,
    ));
    let mut last = ctx.data(|data| data.get_temp::<f64>(id)).unwrap_or(-100.0);
    if scrolling || dragging {
        last = now;
        ctx.data_mut(|data| data.insert_temp(id, last));
    }
    let visibility = (1.0 - ((now - last - 0.45) / 0.25).max(0.0)).clamp(0.0, 1.0) as f32;
    ctx.style_mut(|style| {
        let scroll = &mut style.spacing.scroll;
        scroll.dormant_handle_opacity = 0.55 * visibility;
        scroll.active_handle_opacity = 0.65 * visibility;
        scroll.interact_handle_opacity = 0.9 * visibility;
        scroll.dormant_background_opacity = 0.0;
        scroll.active_background_opacity = 0.10 * visibility;
        scroll.interact_background_opacity = 0.22 * visibility;
    });
    if visibility > 0.0 { ctx.request_repaint_after(Duration::from_millis(16)); }
}

fn suite_scroll_style() -> egui::style::ScrollStyle {
    egui::style::ScrollStyle {
        floating: true,
        bar_width: 10.0,
        floating_width: 5.0,
        // Keep a permanent gutter so expansion never covers card controls or text.
        floating_allocated_width: 16.0,
        handle_min_length: 40.0,
        bar_inner_margin: 4.0,
        bar_outer_margin: 2.0,
        foreground_color: true,
        dormant_background_opacity: 0.0,
        active_background_opacity: 0.16,
        interact_background_opacity: 0.35,
        dormant_handle_opacity: 0.30,
        active_handle_opacity: 0.55,
        interact_handle_opacity: 0.85,
    }
}

fn readable_app_color(color: Color32) -> Color32 {
    if light_theme() { mix_color(color, Color32::from_rgb(22, 28, 40), 0.42) } else { color }
}

fn reveal_project_path(path: &Path, select: bool) {
    #[cfg(target_os="macos")]
    { let mut command=Command::new("/usr/bin/open");if select{command.arg("-R");}let _=command.arg(path).spawn(); }
    #[cfg(target_os="linux")]
    { platform::reveal_linux(path, select); }
    #[cfg(target_os = "windows")]
    {
        platform::reveal_windows(path,select);
    }
}


fn paint_suite_splash(
    painter: &egui::Painter, rect: egui::Rect, tint: Color32,
    logo: Option<egui::TextureId>, name: &str, category: &str, title: &str,
    description: &str, icons: &[(Option<egui::TextureId>, Color32, char)],
    progress: f32, reduce_motion: bool,
) {
    let smooth = |t: f32| { let t=t.clamp(0.0,1.0); t*t*(3.0-2.0*t) };
    let elapsed = progress * 3.0;
    let reveal = if reduce_motion { 1.0 } else { smooth(elapsed / 0.55) };
    let settled = smooth((progress-0.78)/0.22);
    let dark = mix_color(Color32::from_rgb(24,20,39),tint,0.18);
    let left = egui::Rect::from_min_max(rect.min,egui::pos2(rect.left()+242.0,rect.bottom()));
    painter.rect_filled(rect,18.0,dark);
    painter.rect_filled(left,18.0,Color32::from_rgb(250,250,252));
    painter.rect_filled(egui::Rect::from_min_max(left.left_top()+Vec2::new(18.0,0.0),left.right_bottom()),0.0,Color32::from_rgb(250,250,252));
    painter.rect_stroke(rect,18.0,egui::Stroke::new(1.0_f32,mix_color(dark,tint,0.4)),egui::StrokeKind::Inside);
    let uv=egui::Rect::from_min_max(egui::pos2(0.0,0.0),egui::pos2(1.0,1.0));
    let draw_mark = |p: &egui::Painter, mark: egui::Rect| {
        if let Some(texture)=logo {p.image(texture,mark,uv,Color32::WHITE);}
        else {p.rect_filled(mark,10.0,tint);p.text(mark.center(),egui::Align2::CENTER_CENTER,tr(name.chars().next().unwrap_or('A')),egui::FontId::proportional(mark.height()*0.55),Color32::WHITE);}
    };
    draw_mark(painter,egui::Rect::from_min_size(left.min+Vec2::new(28.0,30.0),Vec2::splat(38.0)));
    let copy=painter.with_clip_rect(left.shrink(22.0));
    let offset=Vec2::new(0.0,if reduce_motion {0.0}else{8.0*(1.0-reveal)});
    copy.text(left.min+Vec2::new(28.0,95.0)+offset,egui::Align2::LEFT_TOP,tr(category),egui::FontId::proportional(text_size(10.0)),mix_color(tint,Color32::BLACK,0.22));
    let title_galley=copy.layout(tr(title),egui::FontId::proportional(text_size(22.0)),Color32::from_rgb(25,26,30),184.0);
    let description_y=(120.0+title_galley.size().y+12.0).max(186.0);
    copy.galley(left.min+Vec2::new(28.0,120.0)+offset,title_galley,Color32::from_rgb(25,26,30));
    let description=copy.layout(tr(description.to_string()),egui::FontId::proportional(text_size(12.0)),Color32::from_rgb(91,94,101),184.0);
    copy.galley(left.min+Vec2::new(28.0,description_y)+offset,description,Color32::from_rgb(91,94,101));
    copy.text(left.min+Vec2::new(28.0,266.0),egui::Align2::LEFT_TOP,tr(if progress<0.8 {"PREPARING YOUR WORKSPACE"}else{"READY TO OPEN"}),egui::FontId::proportional(text_size(10.0)),Color32::from_rgb(124,127,134));
    let track=egui::Rect::from_min_size(left.min+Vec2::new(28.0,290.0),Vec2::new(184.0,5.0));
    painter.rect_filled(track,3.0,Color32::from_rgb(226,226,231));
    let fill=egui::Rect::from_min_size(track.min,Vec2::new(track.width()*smooth(progress),track.height()));
    painter.rect_filled(fill,3.0,tint);
    copy.text(left.left_bottom()+Vec2::new(28.0,-26.0),egui::Align2::LEFT_BOTTOM,tr("A creative workspace by ArtCraft"),egui::FontId::proportional(text_size(11.0)),Color32::from_rgb(133,136,142));

    let artwork=egui::Rect::from_min_max(egui::pos2(left.right(),rect.top()),rect.max);
    let art=painter.with_clip_rect(artwork.shrink(1.0));
    let center=artwork.center()+Vec2::new(0.0,4.0);
    let breathe=if reduce_motion {0.0}else{(elapsed*2.0).sin()*2.0*(1.0-settled)};
    for (radius,amount) in [(150.0,0.025),(138.0,0.045),(112.0,0.19),(78.0,0.30)] {
        art.circle_filled(center,(radius+breathe)*(0.96+0.04*reveal),mix_color(dark,tint,amount));
    }
    art.circle_stroke(center,119.0,egui::Stroke::new(1.0_f32,mix_color(dark,tint,0.32)));
    art.circle_stroke(center,151.0,egui::Stroke::new(1.0_f32,mix_color(dark,tint,0.12)));
    for (index,(texture,color,initial)) in icons.iter().enumerate() {
        let entry=if reduce_motion {1.0}else{smooth((elapsed-0.10-index as f32*0.06)/0.48)};
        let rotation=if reduce_motion {0.0}else{0.28*(elapsed-0.28*settled)};
        let angle=rotation+std::f32::consts::TAU*index as f32/icons.len().max(1) as f32-std::f32::consts::FRAC_PI_2;
        let point=center+Vec2::new(angle.cos(),angle.sin())*(104.0+15.0*entry);
        let tile=egui::Rect::from_center_size(point,Vec2::splat(32.0+6.0*entry));
        art.rect_filled(tile.translate(Vec2::new(0.0,4.0)).expand(3.0),12.0,Color32::from_black_alpha((35.0*entry) as u8));
        art.rect_filled(tile.expand(4.0),12.0,mix_color(dark,*color,0.26*entry));
        art.rect_filled(tile,8.0,color.gamma_multiply(entry));
        if let Some(texture)=texture {art.image(*texture,tile.shrink(1.5),uv,Color32::WHITE.gamma_multiply(entry));}
        else {art.text(tile.center(),egui::Align2::CENTER_CENTER,initial,egui::FontId::proportional(text_size(18.0)),Color32::WHITE.gamma_multiply(entry));}
        art.rect_stroke(tile.expand(4.0),12.0,egui::Stroke::new(1.0_f32,mix_color(dark,*color,0.5*entry)),egui::StrokeKind::Inside);
    }
    draw_mark(&art,egui::Rect::from_center_size(center,Vec2::splat(76.0+4.0*reveal)));
    art.text(artwork.right_top()+Vec2::new(-28.0,26.0),egui::Align2::RIGHT_TOP,tr(format!("WELCOME TO {}",name.to_uppercase())),egui::FontId::proportional(text_size(10.0)),Color32::from_white_alpha(180));
    art.text(artwork.center_bottom()+Vec2::new(0.0,-26.0),egui::Align2::CENTER_BOTTOM,tr("Your tools. Your ideas. Your workspace."),egui::FontId::proportional(text_size(11.0)),Color32::from_white_alpha(195));
}


// Best-effort disk-cache warming only. Never starts the app, loads a DLL, or
// waits on this worker before launching. Keep resource use bounded per splash.
fn warm_launch_files(executable: PathBuf) {
    let _ = thread::Builder::new().name("app-launch-preload".into()).spawn(move || {
        let deadline = Instant::now() + Duration::from_millis(2400);
        let mut files = vec![executable.clone()];
        if let Some(parent) = executable.parent() {
            if let Ok(entries) = fs::read_dir(parent) {
                let mut resources: Vec<_> = entries.take(256).filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| path.extension().and_then(|e| e.to_str()).is_some_and(|e|
                        ["dll", "pak", "bin", "dat"].iter().any(|extension| e.eq_ignore_ascii_case(extension))))
                    .collect();
                resources.sort();
                files.extend(resources.into_iter().take(24));
            }
        }
        let mut buffer = vec![0_u8; 256 * 1024];
        let mut total = 0_usize;
        for path in files {
            if Instant::now() >= deadline || total >= 96 * 1024 * 1024 { break; }
            let Ok(mut file) = fs::File::open(path) else { continue; };
            let mut file_bytes = 0_usize;
            while Instant::now() < deadline && total < 96 * 1024 * 1024 && file_bytes < 32 * 1024 * 1024 {
                match file.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(bytes) => { total += bytes; file_bytes += bytes; }
                }
            }
        }
    });
}


fn project_revision(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hash=std::collections::hash_map::DefaultHasher::new();
    let Ok(metadata)=fs::metadata(path) else {return 0;};
    metadata.len().hash(&mut hash);metadata.modified().ok().hash(&mut hash);metadata.created().ok().hash(&mut hash);
    // Sample the header, middle and tail as well, for saves preserving timestamps.
    if let Ok(mut file)=fs::File::open(path) {
        let mut bytes=[0_u8;4096];
        for offset in [0,metadata.len()/2,metadata.len().saturating_sub(4096)] {
            if file.seek(SeekFrom::Start(offset)).is_ok() {
                if let Ok(count)=file.read(&mut bytes) {bytes[..count].hash(&mut hash);}
            }
        }
    }
    hash.finish()
}

// Shared text-size preference on all desktop platforms.
fn text_size(size: f32) -> f32 { size * windows_ui::text_scale() }
