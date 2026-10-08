//! Everything that touches Windows: finding the shortcuts, pulling icons out of
//! them, and launching apps / projects.

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Raw RGBA image data.
pub struct RgbaImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

// ───────────────────────────── shortcut discovery ─────────────────────────────

/// Default Start Menu folder that holds the shortcuts.
pub fn default_start_menu_dir() -> String {
    let base = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    format!(r"{base}\Microsoft\Windows\Start Menu\Programs")
}

/// The per-user Start Menu folder (searched as a fallback).
fn user_start_menu_dir() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(|a| PathBuf::from(a).join(r"Microsoft\Windows\Start Menu\Programs"))
}

/// Find `<name>.lnk` (or .exe / .url) in the Start Menu folder. Looks directly
/// in the folder first, then up to two levels of sub-folders (apps are often
/// placed in a folder of their own). Falls back to the per-user Start Menu.
pub fn find_shortcut(start_menu: &str, name: &str) -> Option<PathBuf> {
    let mut roots = vec![PathBuf::from(start_menu)];
    if let Some(u) = user_start_menu_dir() {
        roots.push(u);
    }
    roots.iter().find_map(|r| search_dir(r, name, 2))
}

fn search_dir(dir: &Path, name: &str, depth: u32) -> Option<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();

    // Files in this folder first.
    for e in &entries {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let stem = p.file_stem()?.to_string_lossy().to_lowercase();
        let ext = p
            .extension()
            .map(|x| x.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if stem == name.to_lowercase() && matches!(ext.as_str(), "lnk" | "exe" | "url") {
            return Some(p);
        }
    }
    // Then sub-folders.
    if depth > 0 {
        for e in &entries {
            let p = e.path();
            if p.is_dir() {
                if let Some(found) = search_dir(&p, name, depth - 1) {
                    return Some(found);
                }
            }
        }
    }
    None
}

// ─────────────────────────────────── launching ────────────────────────────────

/// Launch an app by "opening" its shortcut, exactly like double-clicking it.
#[cfg(windows)]
pub fn launch_app(shortcut: &Path) -> Result<(), String> {
    Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(shortcut)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Open a project in an app: resolve the shortcut's real target + arguments and
/// start it with the project path appended. If the shortcut can't be resolved
/// we fall back to opening the file with its default Windows handler.
#[cfg(windows)]
pub fn open_project(shortcut: &Path, project: &Path) -> Result<(), String> {
    if let Some((target, args, workdir)) = resolve_shortcut(shortcut) {
        if !target.is_empty() {
            let mut cmd = Command::new(&target);
            if !args.trim().is_empty() {
                cmd.raw_arg(args.trim());
            }
            cmd.arg(project);
            if !workdir.is_empty() && Path::new(&workdir).is_dir() {
                cmd.current_dir(workdir);
            }
            return cmd.spawn().map(|_| ()).map_err(|e| e.to_string());
        }
    }
    Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(project)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Resolve a .lnk into (target, arguments, working dir) using the Windows
/// scripting host (built into every Windows install).
#[cfg(windows)]
fn resolve_shortcut(shortcut: &Path) -> Option<(String, String, String)> {
    if shortcut
        .extension()
        .map(|e| e.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
    {
        return Some((shortcut.to_string_lossy().into_owned(), String::new(), String::new()));
    }
    let script = "$s=(New-Object -ComObject WScript.Shell).CreateShortcut($env:CRAFT_LNK); \
                  $s.TargetPath; $s.Arguments; $s.WorkingDirectory";
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("CRAFT_LNK", shortcut)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut lines = text.lines();
    let target = lines.next().unwrap_or("").trim().to_string();
    let args = lines.next().unwrap_or("").trim().to_string();
    let wd = lines.next().unwrap_or("").trim().to_string();
    Some((target, args, wd))
}

/// Reveal a file in Explorer.
#[cfg(windows)]
pub fn show_in_folder(path: &Path) {
    let _ = Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
}

#[cfg(not(windows))]
pub fn launch_app(_: &Path) -> Result<(), String> {
    Err("Launching apps is only supported on Windows".into())
}
#[cfg(not(windows))]
pub fn open_project(_: &Path, _: &Path) -> Result<(), String> {
    Err("Opening projects is only supported on Windows".into())
}
#[cfg(not(windows))]
pub fn show_in_folder(_: &Path) {}

// ─────────────────────────── installed versions / links ───────────────────────

/// Reads the version stored inside each app's .exe (via its shortcut), without
/// running the app. Returns one entry per input, `None` when unknown.
#[cfg(windows)]
pub fn installed_versions(shortcuts: &[Option<PathBuf>]) -> Vec<Option<String>> {
    let joined = shortcuts
        .iter()
        .map(|s| s.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("|");
    let script = "$ErrorActionPreference='SilentlyContinue'; \
        foreach($p in $env:CRAFT_LNKS.Split('|')){ $v=''; \
          if($p){ $t=$p; \
            if($p -like '*.lnk'){ $t=(New-Object -ComObject WScript.Shell).CreateShortcut($p).TargetPath }; \
            if($t){ $i=(Get-Item -LiteralPath $t).VersionInfo; $v=[string]$i.ProductVersion; \
                    if(-not $v){ $v=[string]$i.FileVersion } } }; \
          [Console]::WriteLine($v) }";
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("CRAFT_LNKS", joined)
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    text.lines()
        .map(|l| {
            let l = l.trim();
            if l.is_empty() { None } else { Some(l.to_string()) }
        })
        .chain(std::iter::repeat(None))
        .take(shortcuts.len())
        .collect()
}

/// Open a web page in the default browser (only GitHub / ArtCraft links).
#[cfg(windows)]
pub fn open_url(url: &str) {
    if url.starts_with("https://github.com/") || url.starts_with("https://getartcraft.com/") {
        let _ = Command::new("explorer").arg(url).spawn();
    }
}

/// Windows "Installed apps" settings page, used for uninstalling.
#[cfg(windows)]
pub fn open_apps_settings() {
    let _ = Command::new("explorer").arg("ms-settings:appsfeatures").spawn();
}

#[cfg(not(windows))]
pub fn installed_versions(s: &[Option<PathBuf>]) -> Vec<Option<String>> {
    vec![None; s.len()]
}
#[cfg(not(windows))]
pub fn open_url(_: &str) {}
#[cfg(not(windows))]
pub fn open_apps_settings() {}

// ───────────────────────────────── icon extraction ────────────────────────────

/// Ask the Windows shell for the highest-resolution icon (up to 256×256) of a
/// file – for a .lnk this is the icon of the app it points to.
#[cfg(windows)]
pub fn shell_icon(path: &Path) -> Option<RgbaImage> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
    use windows::Win32::UI::Controls::IImageList;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX};
    use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

    const SHIL_JUMBO: i32 = 4; // 256 x 256
    const SHIL_EXTRALARGE: i32 = 2; // 48 x 48
    const ILD_TRANSPARENT: u32 = 0x0000_0001;

    unsafe {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut sfi = SHFILEINFOW::default();
        let r = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut sfi as *mut SHFILEINFOW),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        );
        if r == 0 {
            return None;
        }
        for size in [SHIL_JUMBO, SHIL_EXTRALARGE] {
            let Ok(list) = SHGetImageList::<IImageList>(size) else { continue };
            let Ok(hicon) = list.GetIcon(sfi.iIcon, ILD_TRANSPARENT) else { continue };
            let img = hicon_to_rgba(hicon);
            let _ = DestroyIcon(hicon);
            if let Some(img) = img {
                return Some(trim_to_square(img));
            }
        }
        None
    }
}

#[cfg(windows)]
unsafe fn hicon_to_rgba(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<RgbaImage> {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    let mut info = ICONINFO::default();
    GetIconInfo(hicon, &mut info).ok()?;

    let cleanup = |info: &ICONINFO| {
        if !info.hbmColor.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(info.hbmColor.0));
        }
        if !info.hbmMask.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
        }
    };

    let mut bm = BITMAP::default();
    let got = GetObjectW(
        HGDIOBJ(info.hbmColor.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut _ as *mut c_void),
    );
    if got == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
        cleanup(&info);
        return None;
    }
    let (w, h) = (bm.bmWidth, bm.bmHeight);

    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = w;
    bmi.bmiHeader.biHeight = -h; // top-down
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = 0; // BI_RGB

    let mut buf = vec![0u8; (w * h * 4) as usize];
    let hdc = GetDC(HWND::default());
    let lines = GetDIBits(
        hdc,
        info.hbmColor,
        0,
        h as u32,
        Some(buf.as_mut_ptr() as *mut c_void),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    ReleaseDC(HWND::default(), hdc);
    cleanup(&info);
    if lines == 0 {
        return None;
    }

    // BGRA -> RGBA; icons without an alpha channel get full opacity.
    let has_alpha = buf.chunks_exact(4).any(|p| p[3] != 0);
    for p in buf.chunks_exact_mut(4) {
        p.swap(0, 2);
        if !has_alpha {
            p[3] = 255;
        }
    }
    Some(RgbaImage { width: w as usize, height: h as usize, pixels: buf })
}

/// Crop transparent borders and centre the result on a square canvas.
#[cfg(windows)]
fn trim_to_square(img: RgbaImage) -> RgbaImage {
    let (w, h) = (img.width, img.height);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            if img.pixels[(y * w + x) * 4 + 3] > 8 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    if x1 < x0 || y1 < y0 {
        return img;
    }
    let (cw, ch) = (x1 - x0 + 1, y1 - y0 + 1);
    let side = cw.max(ch);
    let mut out = vec![0u8; side * side * 4];
    let (ox, oy) = ((side - cw) / 2, (side - ch) / 2);
    for y in 0..ch {
        for x in 0..cw {
            let s = ((y0 + y) * w + (x0 + x)) * 4;
            let d = ((oy + y) * side + (ox + x)) * 4;
            out[d..d + 4].copy_from_slice(&img.pixels[s..s + 4]);
        }
    }
    RgbaImage { width: side, height: side, pixels: out }
}

#[cfg(not(windows))]
pub fn shell_icon(_: &Path) -> Option<RgbaImage> {
    None
}
