//! App manager: finds the newest Windows build of each Craft app on GitHub,
//! downloads it, verifies the checksum and installs it.
//!
//! Releases live at https://github.com/storytold/<appname>/releases and ship
//! `<app>-<version>-windows-x64.msi` plus a `...-windows-x64-portable.zip`.

use eframe::egui::Context;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const OWNER: &str = "storytold";

// ───────────────────────────────── data types ─────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AssetKind {
    Msi,
    PortableZip,
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: Option<String>,
    pub kind: AssetKind,
}

#[derive(Clone, Debug)]
pub struct Release {
    pub tag: String,
    pub html_url: String,
    pub published: String, // YYYY-MM-DD
    pub prerelease: bool,
    pub asset: Option<Asset>,
}

/// Messages sent from worker threads back to the UI.
pub enum Event {
    Release(usize, Result<Release, String>),
    Versions(Vec<Option<String>>),
    Progress(usize, u64, u64),
    Installing(usize, String),
    Finished(usize, Result<String, String>),
}

// ─────────────────────────────── version handling ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    nums: [u64; 3],
    pre: Vec<String>,
}

pub fn parse_version(s: &str) -> Option<Version> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let s = s.split('+').next()?;
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) => (c, p),
        None => (s, ""),
    };
    let mut parts = core.split('.');
    let mut nums = [0u64; 3];
    for n in nums.iter_mut() {
        match parts.next() {
            Some(p) => *n = p.trim().parse().ok()?,
            None => break,
        }
    }
    // A 4th component (MSI style "0.1.1.0") is ignored.
    Some(Version {
        nums,
        pre: if pre.is_empty() { vec![] } else { pre.split('.').map(String::from).collect() },
    })
}

impl Ord for Version {
    fn cmp(&self, o: &Self) -> Ordering {
        match self.nums.cmp(&o.nums) {
            Ordering::Equal => {}
            x => return x,
        }
        match (self.pre.is_empty(), o.pre.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater, // 1.0.0 > 1.0.0-rc.1
            (false, true) => Ordering::Less,
            (false, false) => {
                for (a, b) in self.pre.iter().zip(&o.pre) {
                    let c = match (a.parse::<u64>(), b.parse::<u64>()) {
                        (Ok(x), Ok(y)) => x.cmp(&y),
                        (Ok(_), Err(_)) => Ordering::Less,
                        (Err(_), Ok(_)) => Ordering::Greater,
                        (Err(_), Err(_)) => a.cmp(b),
                    };
                    if c != Ordering::Equal {
                        return c;
                    }
                }
                self.pre.len().cmp(&o.pre.len())
            }
        }
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// `Some(true)` when `latest` is newer than `installed`.
pub fn is_newer(latest: &str, installed: &str) -> Option<bool> {
    Some(parse_version(latest)? > parse_version(installed)?)
}

/// MSI file versions can't express "-rc.4", so when the launcher itself
/// installed a build we trust the tag it recorded (if the numbers agree).
pub fn effective_installed(file_version: Option<&str>, recorded_tag: Option<&str>) -> Option<String> {
    match (file_version, recorded_tag) {
        (Some(f), Some(r)) => match (parse_version(f), parse_version(r)) {
            (Some(a), Some(b)) if a.nums == b.nums => Some(r.to_string()),
            _ => Some(f.to_string()),
        },
        (Some(f), None) => Some(f.to_string()),
        (None, Some(r)) => Some(r.to_string()),
        (None, None) => None,
    }
}

// ───────────────────────────────── GitHub API ─────────────────────────────────

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .user_agent("CraftLauncher/0.1 (+https://getartcraft.com)")
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .build()
}

fn http_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(403 | 429, _) => {
            "GitHub rate limit reached - try again in a little while".into()
        }
        ureq::Error::Status(404, _) => "No releases published yet".into(),
        ureq::Error::Status(c, _) => format!("GitHub returned HTTP {c}"),
        ureq::Error::Transport(t) => format!("Network problem: {t}"),
    }
}

pub fn fetch_latest(repo: &str, include_pre: bool) -> Result<Release, String> {
    let url = format!(
        "https://api.github.com/repos/{OWNER}/{}/releases?per_page=20",
        repo.to_lowercase()
    );
    let v: Value = agent()
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(http_err)?
        .into_json()
        .map_err(|e| e.to_string())?;
    let list = v.as_array().ok_or("Unexpected response from GitHub")?;

    let mut newest_without_asset: Option<Release> = None;
    for rel in list {
        if rel["draft"].as_bool().unwrap_or(false) {
            continue;
        }
        let pre = rel["prerelease"].as_bool().unwrap_or(false);
        if pre && !include_pre {
            continue;
        }
        let r = Release {
            tag: rel["tag_name"].as_str().unwrap_or("").to_string(),
            html_url: rel["html_url"].as_str().unwrap_or("").to_string(),
            published: rel["published_at"].as_str().and_then(|d| d.get(..10)).unwrap_or("").to_string(),
            prerelease: pre,
            asset: pick_asset(rel),
        };
        if r.asset.is_some() {
            return Ok(r);
        }
        if newest_without_asset.is_none() {
            newest_without_asset = Some(r);
        }
    }
    newest_without_asset.ok_or_else(|| "No releases published yet".to_string())
}

fn pick_asset(rel: &Value) -> Option<Asset> {
    let assets = rel["assets"].as_array()?;
    let find = |suffix: &str| {
        assets
            .iter()
            .find(|a| a["name"].as_str().map_or(false, |n| n.to_lowercase().ends_with(suffix)))
    };
    let (a, kind) = if let Some(a) = find("windows-x64.msi") {
        (a, AssetKind::Msi)
    } else if let Some(a) = find("windows-x64-portable.zip") {
        (a, AssetKind::PortableZip)
    } else {
        return None;
    };
    Some(Asset {
        name: a["name"].as_str()?.to_string(),
        url: a["browser_download_url"].as_str()?.to_string(),
        size: a["size"].as_u64().unwrap_or(0),
        sha256: a["digest"]
            .as_str()
            .and_then(|d| d.strip_prefix("sha256:"))
            .map(|d| d.to_lowercase()),
        kind,
    })
}

// ──────────────────────────────── worker threads ──────────────────────────────

pub fn check(idx: usize, repo: &'static str, include_pre: bool, tx: Sender<Event>, ctx: Context) {
    std::thread::spawn(move || {
        let r = fetch_latest(repo, include_pre);
        let _ = tx.send(Event::Release(idx, r));
        ctx.request_repaint();
    });
}

pub fn install(
    idx: usize,
    app: &'static str,
    repo: &'static str,
    rel: Release,
    cancel: Arc<AtomicBool>,
    tx: Sender<Event>,
    ctx: Context,
) {
    std::thread::spawn(move || {
        let res = run_install(idx, app, repo, &rel, &cancel, &tx, &ctx).map(|_| rel.tag.clone());
        let _ = tx.send(Event::Finished(idx, res));
        ctx.request_repaint();
    });
}

fn run_install(
    idx: usize,
    app: &str,
    repo: &str,
    rel: &Release,
    cancel: &AtomicBool,
    tx: &Sender<Event>,
    ctx: &Context,
) -> Result<(), String> {
    let asset = rel.asset.as_ref().ok_or("No Windows installer in this release")?;
    if !asset.url.starts_with("https://github.com/") {
        return Err("Refusing to download from an unexpected address".into());
    }
    let safe_name = Path::new(&asset.name)
        .file_name()
        .ok_or("Bad file name")?
        .to_os_string();
    let dir = std::env::temp_dir().join("CraftLauncher");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(safe_name);

    let mut last = Instant::now();
    download(asset, &file, cancel, |done, total| {
        if last.elapsed() > Duration::from_millis(100) || done == total {
            last = Instant::now();
            let _ = tx.send(Event::Progress(idx, done, total));
            ctx.request_repaint();
        }
    })?;

    let result = match asset.kind {
        AssetKind::Msi => {
            let _ = tx.send(Event::Installing(idx, "Installing - approve the Windows prompt".into()));
            ctx.request_repaint();
            install_msi(&file)
        }
        AssetKind::PortableZip => {
            let _ = tx.send(Event::Installing(idx, "Unpacking".into()));
            ctx.request_repaint();
            install_zip(app, repo, &file)
        }
    };
    let _ = std::fs::remove_file(&file);
    result
}

/// Streams the file to disk, hashing as it goes, and checks the SHA-256 that
/// GitHub publishes for the asset.
fn download(
    asset: &Asset,
    dest: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    let resp = agent().get(&asset.url).call().map_err(http_err)?;
    let total = resp
        .header("Content-Length")
        .and_then(|h| h.parse::<u64>().ok())
        .unwrap_or(asset.size);
    let mut reader = resp.into_reader();
    let mut out = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut done = 0u64;

    loop {
        if cancel.load(AtomicOrdering::Relaxed) {
            drop(out);
            let _ = std::fs::remove_file(dest);
            return Err("Cancelled".into());
        }
        let n = reader.read(&mut buf).map_err(|e| format!("Download interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        hasher.update(&buf[..n]);
        done += n as u64;
        on_progress(done, total.max(done));
    }
    out.flush().map_err(|e| e.to_string())?;
    drop(out);

    if let Some(expected) = &asset.sha256 {
        let actual = format!("{:x}", hasher.finalize());
        if &actual != expected {
            let _ = std::fs::remove_file(dest);
            return Err("Checksum mismatch - the download was corrupted, please retry".into());
        }
    }
    Ok(())
}

// ─────────────────────────────────── installing ───────────────────────────────

#[cfg(windows)]
fn install_msi(file: &Path) -> Result<(), String> {
    let status = Command::new("msiexec")
        .arg("/i")
        .arg(file)
        .args(["/passive", "/norestart"])
        .status()
        .map_err(|e| format!("Couldn't start Windows Installer: {e}"))?;
    match status.code() {
        Some(0) | Some(3010) => Ok(()),
        Some(1602) | Some(1223) => Err("Installation was cancelled".into()),
        Some(1618) => Err("Another installation is in progress - try again shortly".into()),
        Some(c) => Err(format!("Windows Installer failed (code {c})")),
        None => Err("Windows Installer was interrupted".into()),
    }
}

/// Portable zip fallback: unpack under %LOCALAPPDATA% and add a Start Menu shortcut.
#[cfg(windows)]
fn install_zip(app: &str, repo: &str, zip: &Path) -> Result<(), String> {
    let base = std::env::var("LOCALAPPDATA").map_err(|_| "LOCALAPPDATA is not set")?;
    let dest = PathBuf::from(base).join("Programs").join("CraftLauncher").join("Apps").join(app);
    if dest.exists() {
        std::fs::remove_dir_all(&dest)
            .map_err(|_| format!("Couldn't replace the old files - close {app} and try again"))?;
    }
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    let out = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Expand-Archive -LiteralPath $env:CRAFT_ZIP -DestinationPath $env:CRAFT_DIR -Force",
        ])
        .env("CRAFT_ZIP", zip)
        .env("CRAFT_DIR", &dest)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("Couldn't unpack the download".into());
    }

    let exe = find_exe(&dest, &[app, repo], 3).ok_or("Couldn't find the app inside the download")?;
    let programs = std::env::var("APPDATA")
        .map(|a| PathBuf::from(a).join(r"Microsoft\Windows\Start Menu\Programs"))
        .map_err(|_| "APPDATA is not set")?;
    std::fs::create_dir_all(&programs).map_err(|e| e.to_string())?;
    let lnk = programs.join(format!("{app}.lnk"));

    let out = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$s=(New-Object -ComObject WScript.Shell).CreateShortcut($env:CRAFT_LNK); \
             $s.TargetPath=$env:CRAFT_EXE; $s.WorkingDirectory=$env:CRAFT_WD; $s.Save()",
        ])
        .env("CRAFT_LNK", &lnk)
        .env("CRAFT_EXE", &exe)
        .env("CRAFT_WD", exe.parent().unwrap_or(&dest))
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("Unpacked, but couldn't create the Start Menu shortcut".into());
    }
    Ok(())
}

#[cfg(windows)]
fn find_exe(dir: &Path, names: &[&str], depth: u32) -> Option<PathBuf> {
    let wanted: Vec<String> = names.iter().map(|n| format!("{}.exe", n.to_lowercase())).collect();
    let mut subdirs = Vec::new();
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            subdirs.push(p);
        } else if p.file_name().map_or(false, |n| wanted.contains(&n.to_string_lossy().to_lowercase())) {
            return Some(p);
        }
    }
    if depth > 0 {
        for d in subdirs {
            if let Some(f) = find_exe(&d, names, depth - 1) {
                return Some(f);
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn install_msi(_: &Path) -> Result<(), String> {
    Err("Installing is only supported on Windows".into())
}
#[cfg(not(windows))]
fn install_zip(_: &str, _: &str, _: &Path) -> Result<(), String> {
    Err("Installing is only supported on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_like_semver() {
        assert_eq!(is_newer("v0.1.1", "0.1.0"), Some(true));
        assert_eq!(is_newer("v0.1.1-rc.5", "v0.1.1-rc.4"), Some(true));
        assert_eq!(is_newer("v0.1.1-rc.4", "0.1.1"), Some(false)); // final beats rc
        assert_eq!(is_newer("v0.1.1", "0.1.1.0"), Some(false));
        assert_eq!(is_newer("v0.2.0-rc.1", "0.1.9"), Some(true));
        assert_eq!(is_newer("garbage", "0.1.0"), None);
    }
}
