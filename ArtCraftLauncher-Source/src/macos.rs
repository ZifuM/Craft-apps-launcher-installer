//! Native macOS package, launch-agent and bundle integration.
use super::*;

pub const BUNDLE_ID: &str = "io.github.ZifuM.ArtCraftMasterSuite";

fn output(command: &mut Command) -> Result<Vec<u8>, String> {
    let result = command.output().map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).trim().to_owned());
    }
    Ok(result.stdout)
}

fn bundle_info(bundle: &Path) -> Result<serde_json::Value, String> {
    let bytes = output(Command::new("/usr/bin/plutil").args(["-convert", "json", "-o", "-"]).arg(bundle.join("Contents/Info.plist")))?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub fn bundle_executable(bundle: &Path) -> Result<PathBuf, String> {
    let info = bundle_info(bundle)?;
    if info["CFBundlePackageType"] != "APPL" { return Err("Package is not a macOS application.".into()); }
    let name = info["CFBundleExecutable"].as_str().ok_or("Application executable is missing")?;
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." { return Err("Invalid application executable name".into()); }
    let binary = bundle.join("Contents/MacOS").join(name);
    let root = fs::canonicalize(bundle).map_err(|e| e.to_string())?;
    let actual = fs::canonicalize(&binary).map_err(|e| e.to_string())?;
    if !actual.starts_with(root) || !actual.is_file() { return Err("Invalid application executable path".into()); }
    Ok(binary)
}

pub fn find_executable(root: &Path) -> Option<PathBuf> {
    let mut apps = fs::read_dir(root).ok()?.flatten().map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "app") && p.is_dir());
    let app = apps.next()?;
    if apps.next().is_some() { return None; }
    bundle_executable(&app).ok()
}

struct Mounted { work: PathBuf, mount: PathBuf, attached: bool }
impl Drop for Mounted {
    fn drop(&mut self) {
        if self.attached && !Command::new("/usr/bin/hdiutil").arg("detach").arg(&self.mount).status().is_ok_and(|s| s.success()) {
            // Do not recursively remove a mount that could not be detached.
            return;
        }
        let _ = fs::remove_dir_all(&self.work);
    }
}

pub fn install_dmg(dmg: &Path, destination: &Path, suite: bool) -> Result<PathBuf, String> {
    let work = platform::data_dir().ok_or("Application data unavailable")?.join("package-work");
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let work = work.join(format!("{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
    fs::create_dir(&work).map_err(|e| e.to_string())?;
    let mut mounted = Mounted { mount: work.join("volume"), work, attached: false };
    output(Command::new("/usr/bin/hdiutil").args(["attach", "-readonly", "-nobrowse", "-noautoopen", "-mountpoint"]).arg(&mounted.mount).arg(dmg))?;
    mounted.attached = true;
    let apps: Vec<_> = fs::read_dir(&mounted.mount).map_err(|e| e.to_string())?.flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "app")).collect();
    if apps.len() != 1 { return Err("Disk image must contain exactly one application.".into()); }
    let app = &apps[0];
    let executable = bundle_executable(app)?;
    let architecture = if cfg!(target_arch = "aarch64") { "arm64" } else { "x86_64" };
    output(Command::new("/usr/bin/lipo").args(["-verify_arch", architecture]).arg(&executable))?;
    if suite {
        if bundle_info(app)?["CFBundleIdentifier"] != BUNDLE_ID { return Err("Update contains a different application.".into()); }
        output(Command::new("/usr/bin/codesign").args(["--verify", "--deep", "--strict"]).arg(app))?;
    }
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    let target = destination.join(app.file_name().ok_or("Invalid application name")?);
    if target.exists() { return Err("Staged application already exists.".into()); }
    output(Command::new("/usr/bin/ditto").arg(app).arg(&target))?;
    bundle_executable(&target)?;
    Ok(target)
}

pub fn unpack(bytes: &[u8], destination: &Path) -> Result<(), String> {
    use std::io::Write;
    let dmg = destination.join("download.dmg");
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&dmg).map_err(|e| e.to_string())?;
    file.write_all(bytes).and_then(|_| file.sync_all()).map_err(|e| e.to_string())?;
    drop(file);
    let result = install_dmg(&dmg, destination, false).map(|_| ());
    let _ = fs::remove_file(dmg);
    result
}

fn xml(text: &str) -> String { text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;") }

pub fn set_startup(enabled: bool) -> Result<(), String> {
    let root = platform::home().ok_or("Home folder unavailable")?.join("Library/LaunchAgents");
    let path = root.join(format!("{BUNDLE_ID}.plist"));
    if !enabled { if path.exists() { fs::remove_file(path).map_err(|e| e.to_string())?; } return Ok(()); }
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    if executable.starts_with("/Volumes") || executable.to_string_lossy().contains("/AppTranslocation/") {
        return Err("Move Master Suite to Applications and open it from there before enabling login startup.".into());
    }
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    fs::write(path, format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>Label</key><string>{BUNDLE_ID}</string><key>ProgramArguments</key><array><string>{}</string><string>--tray</string></array><key>RunAtLoad</key><true/><key>ProcessType</key><string>Interactive</string></dict></plist>"#, xml(&executable.to_string_lossy()))).map_err(|e| e.to_string())
}

pub struct Tray { _icon: tray_icon::TrayIcon }
impl Tray {
    pub fn new(tx: Sender<Event>, ctx: egui::Context) -> Result<Self, String> {
        use tray_icon::{TrayIconBuilder, Icon, menu::{Menu, MenuItem, MenuEvent}};
        let menu = Menu::new();
        let open = MenuItem::new(tr("Open Master Suite"), true, None);
        let quit = MenuItem::new(tr("Quit"), true, None);
        menu.append_items(&[&open, &quit]).map_err(|e| e.to_string())?;
        let open_id = open.id().clone(); let quit_id = quit.id().clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == open_id { let _ = tx.send(Event::TrayOpen); }
            if event.id == quit_id { let _ = tx.send(Event::TrayQuit); }
            ctx.request_repaint();
        }));
        let image = image::load_from_memory(include_bytes!("../assets/artcraft-icon.png")).map_err(|e| e.to_string())?.resize(22, 22, image::imageops::FilterType::Lanczos3).into_rgba8();
        let icon = Icon::from_rgba(image.as_raw().clone(), image.width(), image.height()).map_err(|e| e.to_string())?;
        let icon = TrayIconBuilder::new().with_tooltip("ArtCraft Master Suite").with_icon(icon).with_menu(Box::new(menu)).build().map_err(|e| e.to_string())?;
        Ok(Self { _icon: icon })
    }
}

pub fn launch_update(dmg: &Path) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let target = executable.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).ok_or("Run Master Suite from its installed .app bundle to update it.")?;
    if target.starts_with("/Volumes") || target.to_string_lossy().contains("/AppTranslocation/") { return Err("Move Master Suite to Applications before updating.".into()); }
    let parent = target.parent().ok_or("Application parent folder unavailable")?;
    let staging = parent.join(format!(".ArtCraft-update-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
    fs::create_dir(&staging).map_err(|e| format!("Application folder is not writable. Install the DMG manually: {e}"))?;
    let new = match install_dmg(dmg, &staging, true) { Ok(app) => app, Err(e) => { let _ = fs::remove_dir_all(&staging); return Err(e); } };
    let backup = staging.join("Previous.app");
    let script = staging.join("finish-update.sh");
    fs::write(&script, r#"#!/bin/sh
set -eu
pid="$1"; target="$2"; staged="$3"; backup="$4"
count=0
while kill -0 "$pid" 2>/dev/null; do
  count=$((count + 1)); [ "$count" -lt 120 ] || exit 1
  sleep 1
done
/usr/bin/codesign --verify --deep --strict "$staged" || exit 1
/bin/mv "$target" "$backup"
if ! /bin/mv "$staged" "$target"; then /bin/mv "$backup" "$target"; exit 1; fi
/usr/bin/open "$target"
"#).map_err(|e| e.to_string())?;
    Command::new("/bin/sh").arg(script).arg(std::process::id().to_string()).arg(target).arg(new).arg(backup)
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map_err(|e| e.to_string())?;
    Ok(())
}
