//! App definitions, persisted settings and project scanning.

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// ───────────────────────────────── app catalogue ──────────────────────────────

pub struct AppDef {
    /// Must match the shortcut's file name (without .lnk).
    pub name: &'static str,
    /// GitHub repository name under github.com/storytold.
    pub repo: &'static str,
    pub tagline: &'static str,
    /// Two-letter badge used when no icon could be loaded.
    pub badge: &'static str,
    pub color: Color32,
}

pub const APPS: [AppDef; 7] = [
    AppDef { name: "PhotoCraft", repo: "photocraft",  tagline: "Image editor",  badge: "Pc", color: Color32::from_rgb(0x2D, 0xA0, 0xF0) },
    AppDef { name: "FilmCraft", repo: "filmcraft",   tagline: "Video editor",   badge: "Fc", color: Color32::from_rgb(0x7A, 0x6C, 0xFF) },
    AppDef { name: "DesignCraft", repo: "designcraft", tagline: "Page layout & publishing",      badge: "Dc", color: Color32::from_rgb(0xFF, 0x4F, 0x7B) },
    AppDef { name: "LightCraft", repo: "lightcraft",  tagline: "Photo library & raw developer",       badge: "Lc", color: Color32::from_rgb(0x36, 0xC5, 0xB0) },
    AppDef { name: "EffectCraft", repo: "effectcraft", tagline: "Motion graphics & VFX",        badge: "Ec", color: Color32::from_rgb(0xB1, 0x6C, 0xFF) },
    AppDef { name: "PrintCraft", repo: "pdfcraft",  tagline: "PDF workbench",      badge: "Pr", color: Color32::from_rgb(0xFF, 0x8A, 0x3D) },
    AppDef { name: "VectorCraft", repo: "vectorcraft", tagline: "Vector illustration",          badge: "Vc", color: Color32::from_rgb(0xFF, 0xB4, 0x00) },
];

// ───────────────────────────────── settings ───────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AppSettings {
    /// Folder scanned for this app's projects.
    pub project_dir: String,
    /// Comma separated extensions, e.g. "psd, png". Empty = every file.
    pub extensions: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub start_menu_dir: String,
    pub apps: HashMap<String, AppSettings>,
    /// Offer pre-release builds (these apps are all early alpha).
    #[serde(default = "yes")]
    pub include_prereleases: bool,
    /// Release tag the launcher itself last installed, per app.
    #[serde(default)]
    pub installed_tags: HashMap<String, String>,
}

fn yes() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        let docs = dirs::document_dir().unwrap_or_else(|| PathBuf::from("."));
        let apps = APPS
            .iter()
            .map(|a| {
                (
                    a.name.to_string(),
                    AppSettings {
                        project_dir: docs.join(a.name).to_string_lossy().into_owned(),
                        extensions: String::new(),
                    },
                )
            })
            .collect();
        Self {
            start_menu_dir: crate::platform::default_start_menu_dir(),
            apps,
            include_prereleases: true,
            installed_tags: HashMap::new(),
        }
    }
}

pub fn data_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("CraftLauncher")
}

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

impl Config {
    pub fn load() -> Self {
        let mut cfg = Config::default();
        if let Ok(text) = std::fs::read_to_string(config_path()) {
            if let Ok(saved) = serde_json::from_str::<Config>(&text) {
                cfg.start_menu_dir = saved.start_menu_dir;
                cfg.include_prereleases = saved.include_prereleases;
                cfg.installed_tags = saved.installed_tags;
                for (k, v) in saved.apps {
                    cfg.apps.insert(k, v);
                }
            }
        }
        cfg
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(data_dir());
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(config_path(), text);
        }
    }
}

// ───────────────────────────────── projects ───────────────────────────────────

#[derive(Clone)]
pub struct Project {
    pub app: usize, // index into APPS
    pub name: String,
    pub path: PathBuf,
    pub modified: SystemTime,
}

const MAX_DEPTH: usize = 5;
const MAX_FILES_PER_APP: usize = 5_000;
const JUNK_EXTENSIONS: [&str; 9] = ["tmp", "bak", "ini", "db", "log", "lnk", "dll", "lock", "cache"];

pub fn scan_projects(cfg: &Config) -> Vec<Project> {
    let mut out = Vec::new();
    for (idx, app) in APPS.iter().enumerate() {
        let Some(s) = cfg.apps.get(app.name) else { continue };
        if s.project_dir.trim().is_empty() {
            continue;
        }
        let exts: Vec<String> = s
            .extensions
            .split(',')
            .map(|e| e.trim().trim_start_matches('.').to_lowercase())
            .filter(|e| !e.is_empty())
            .collect();
        let mut count = 0;
        walk(Path::new(s.project_dir.trim()), 0, &exts, idx, &mut count, &mut out);
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

fn walk(dir: &Path, depth: usize, exts: &[String], app: usize, count: &mut usize, out: &mut Vec<Project>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        if *count >= MAX_FILES_PER_APP {
            return;
        }
        let path = entry.path();
        let fname = entry.file_name().to_string_lossy().into_owned();
        if fname.starts_with('.') || fname.starts_with('~') {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            if depth < MAX_DEPTH {
                walk(&path, depth + 1, exts, app, count, out);
            }
            continue;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let wanted = if exts.is_empty() {
            !JUNK_EXTENSIONS.contains(&ext.as_str())
        } else {
            exts.contains(&ext)
        };
        if !wanted {
            continue;
        }
        *count += 1;
        out.push(Project {
            app,
            name: fname,
            path,
            modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        });
    }
}

pub fn time_ago(t: SystemTime) -> String {
    let s = SystemTime::now().duration_since(t).map(|d| d.as_secs()).unwrap_or(0);
    match s {
        0..=59 => "Just now".into(),
        60..=3599 => format!("{} min ago", s / 60),
        3600..=86_399 => format!("{} h ago", s / 3600),
        86_400..=2_591_999 => {
            let d = s / 86_400;
            if d == 1 { "Yesterday".into() } else { format!("{d} days ago") }
        }
        2_592_000..=31_103_999 => format!("{} mo ago", s / 2_592_000),
        _ => format!("{} yr ago", s / 31_104_000),
    }
}
