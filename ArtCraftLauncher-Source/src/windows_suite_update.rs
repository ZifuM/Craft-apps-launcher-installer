//! Master Suite updates are accepted only from this project's published releases.
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};
const REPO: &str = "ZifuM/Craft-apps-launcher-installer";
const MAX_DOWNLOAD: u64 = 1024 * 1024 * 1024;
#[derive(Clone)]
pub struct Prepared {
    pub version: String,
    pub installer: PathBuf,
    pub digest: String,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}
fn version(text: &str) -> Option<semver::Version> {
    crate::windows_releases::version(text)
}
fn approved_url(url: &str) -> bool {
    url.starts_with(&format!("https://github.com/{REPO}/releases/download/"))
}
fn valid_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit())
}
fn hash_file(path: &std::path::Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buf = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn prepare(
    current: &str,
    include_beta: bool,
    mut progress: impl FnMut(String),
) -> Result<Option<Prepared>, String> {
    let client = Client::builder()
        .user_agent("ArtCraft-Master-Suite-Updater")
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    let metadata = crate::windows_releases::suite(&client, REPO, current, include_beta)?;
    let mut releases: Vec<Release> =
        serde_json::from_value(metadata).map_err(|e| format!("Invalid release metadata: {e}"))?;
    let installed = version(current).ok_or("Installed version is invalid")?;
    let names: Vec<String> = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => vec![
            "Windows-x86_64.exe".into(),
            "ArtCraftMasterSuite-Setup.exe".into(),
            "ArtCraftMasterSuite-Windows-x64.zip".into(),
            "Windows-64bit.Installer.zip".into(),
        ],
        ("macos", "x86_64" | "aarch64") => vec![
            "macOS-universal.dmg".into(),
            "ArtCraftMasterSuite-macOS-universal.dmg".into(),
        ],
        ("linux", arch @ ("x86_64" | "aarch64")) => {
            let extension = if crate::platform::flatpak() {
                "flatpak"
            } else {
                "AppImage"
            };
            vec![
                format!("Linux-{arch}.{extension}"),
                format!("ArtCraftMasterSuite-Linux-{arch}.{extension}"),
            ]
        }
        _ => {
            return Err(format!(
                "Suite updates are not available for {}",
                crate::platform::label()
            ));
        }
    };
    releases.retain(|r| {
        !r.draft
            && (include_beta
                || (!r.prerelease && version(&r.tag_name).is_some_and(|v| v.pre.is_empty())))
            && version(&r.tag_name).is_some_and(|v| v.cmp_precedence(&installed).is_gt())
    });
    releases.sort_by(|a, b| {
        version(&b.tag_name)
            .unwrap()
            .cmp_precedence(&version(&a.tag_name).unwrap())
    });
    let newer_releases_exist = !releases.is_empty();
    let Some(release) = releases.into_iter().find(|r| {
        names
            .iter()
            .any(|name| r.assets.iter().any(|a| &a.name == name))
    }) else {
        if newer_releases_exist {
            return Err("A newer release exists, but a compatible package for this platform has not been attached yet. Try again when the release files are available.".into());
        }
        return Ok(None);
    };
    let latest = version(&release.tag_name).ok_or("Release version is invalid")?;
    let asset = names
        .iter()
        .find_map(|name| release.assets.iter().find(|a| &a.name == name))
        .ok_or("Compatible release asset disappeared")?;
    if !approved_url(&asset.browser_download_url) {
        return Err("Invalid update download URL".into());
    }
    let download_size = if asset.size == 0 {
        crate::windows_releases::download_size(&client, &asset.browser_download_url)?
    } else {
        asset.size
    };
    if download_size == 0 || download_size > MAX_DOWNLOAD {
        return Err("Invalid update download".into());
    }
    let expected = if let Some(hash) = asset
        .digest
        .as_deref()
        .and_then(|s| s.strip_prefix("sha256:"))
        .filter(|s| valid_hash(s))
    {
        hash.to_lowercase()
    } else {
        let sums = release
            .assets
            .iter()
            .find(|a| {
                a.name == "SHA256SUMS.txt"
                    && a.size <= 64 * 1024
                    && approved_url(&a.browser_download_url)
            })
            .ok_or("Release is missing its SHA-256 checksum")?;
        let mut text = String::new();
        client
            .get(&sums.browser_download_url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| e.to_string())?
            .take(65537)
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        if text.len() > 65536 {
            return Err("Checksum file is too large".into());
        }
        text.lines()
            .find_map(|line| {
                let mut parts = line.split_whitespace();
                let hash = parts.next()?;
                let name = parts.next()?.trim_start_matches('*');
                (name == asset.name && valid_hash(hash)).then(|| hash.to_lowercase())
            })
            .ok_or("No matching installer checksum")?
    };
    let root = crate::platform::data_dir()
        .ok_or("Application data is unavailable")?
        .join("updates");
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let folder = root.join(latest.to_string());
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    progress(format!("Downloading Master Suite {}...", release.tag_name));
    let partial = folder.join("download.part");
    let result = (|| -> Result<Prepared, String> {
        let mut response = client
            .get(&asset.browser_download_url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| e.to_string())?;
        let mut output = fs::File::create(&partial).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut total = 0u64;
        let mut buf = [0; 64 * 1024];
        loop {
            let n = response.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > download_size || total > MAX_DOWNLOAD {
                return Err("Update exceeds its declared size".into());
            }
            output.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            hash.update(&buf[..n]);
        }
        output.sync_all().map_err(|e| e.to_string())?;
        drop(output);
        if total != download_size || format!("{:x}", hash.finalize()) != expected {
            return Err("Update checksum did not match. Nothing was installed.".into());
        }
        let installer = folder.join(if cfg!(any(target_os = "linux", target_os = "macos")) {
            asset.name.as_str()
        } else {
            "ArtCraftMasterSuite-Setup.exe"
        });
        if asset.name.ends_with(".zip") {
            let mut archive =
                zip::ZipArchive::new(fs::File::open(&partial).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let candidates: Vec<_> = archive
                .file_names()
                .filter(|n| n.rsplit('/').next() == Some("ArtCraftMasterSuite-Setup.exe"))
                .map(str::to_owned)
                .collect();
            if candidates.len() != 1 {
                return Err("Update archive must contain one Master Suite installer".into());
            }
            let mut entry = archive.by_name(&candidates[0]).map_err(|e| e.to_string())?;
            if entry.size() > MAX_DOWNLOAD {
                return Err("Installer is too large".into());
            }
            let mut output = fs::File::create(&installer).map_err(|e| e.to_string())?;
            let n = std::io::copy(&mut entry.by_ref().take(MAX_DOWNLOAD + 1), &mut output)
                .map_err(|e| e.to_string())?;
            if n > MAX_DOWNLOAD {
                return Err("Installer is too large".into());
            }
            output.sync_all().map_err(|e| e.to_string())?;
        } else {
            fs::copy(&partial, &installer).map_err(|e| e.to_string())?;
        }
        #[cfg(target_os = "windows")]
        {
            let mut magic = [0; 2];
            fs::File::open(&installer)
                .map_err(|e| e.to_string())?
                .read_exact(&mut magic)
                .map_err(|e| e.to_string())?;
            if magic != *b"MZ" {
                return Err("Download is not a Windows installer".into());
            }
        }
        #[cfg(target_os = "linux")]
        if !crate::platform::flatpak() {
            let mut header = [0; 20];
            fs::File::open(&installer)
                .map_err(|e| e.to_string())?
                .read_exact(&mut header)
                .map_err(|e| e.to_string())?;
            crate::platform::validate_appimage(&header)?;
        }
        let digest = hash_file(&installer)?;
        Ok(Prepared {
            version: release.tag_name,
            installer,
            digest,
        })
    })();
    let _ = fs::remove_file(partial);
    result.map(Some)
}
pub fn launch(update: &Prepared) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        return launch_linux(update);
    }
    #[cfg(target_os = "macos")]
    {
        if hash_file(&update.installer)? != update.digest {
            return Err("Staged disk image changed; update cancelled".into());
        }
        return crate::macos::launch_update(&update.installer);
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        if hash_file(&update.installer)? != update.digest {
            return Err("Staged installer changed; update cancelled".into());
        }
        let mut command = std::process::Command::new(&update.installer);
        command
            .arg("--auto-update")
            .arg("--wait-pid")
            .arg(std::process::id().to_string());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command
            .spawn()
            .map_err(|e| format!("Could not start the update: {e}"))?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn flatpak_output(args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("flatpak-spawn")
        .args(["--host", "flatpak"])
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
#[cfg(target_os = "linux")]
fn flatpak_installation() -> Result<(String, String), String> {
    let id = crate::platform::FLATPAK_ID;
    let info = fs::read_to_string("/.flatpak-info").map_err(|e| e.to_string())?;
    let app_path = info
        .lines()
        .find_map(|line| line.strip_prefix("app-path="))
        .ok_or("Flatpak installation path is unavailable")?;
    let parts: Vec<_> = app_path.split('/').collect();
    let index = parts
        .windows(2)
        .position(|parts| parts == ["app", id])
        .ok_or("Cannot identify the running Flatpak reference")?;
    let architecture = *parts
        .get(index + 2)
        .ok_or("Flatpak architecture is missing")?;
    let branch = *parts.get(index + 3).ok_or("Flatpak branch is missing")?;
    if architecture != std::env::consts::ARCH || branch != "stable" {
        return Err(
            "Update this Flatpak architecture or custom branch through its software manager."
                .into(),
        );
    }
    let reference = format!("app/{id}/{architecture}/{branch}");
    let scope = if app_path.starts_with("/var/lib/flatpak/") {
        "--system"
    } else if crate::platform::home()
        .is_some_and(|home| std::path::Path::new(app_path).starts_with(home))
    {
        "--user"
    } else {
        return Err(
            "This custom Flatpak installation must be updated through its software manager.".into(),
        );
    };
    Ok((scope.into(), reference))
}
#[cfg(target_os = "linux")]
fn launch_linux(update: &Prepared) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    if hash_file(&update.installer)? != update.digest {
        return Err("Staged update changed; update cancelled".into());
    }
    if crate::platform::flatpak() {
        let (scope, reference) = flatpak_installation()?;
        let installer = update
            .installer
            .to_str()
            .ok_or("Invalid Flatpak bundle path")?;
        // Install the exact checksum-verified release selected by the update channel.
        // Preserve user/system scope and let Flatpak enforce its normal permissions.
        flatpak_output(&[
            "install",
            &scope,
            "--noninteractive",
            "--bundle",
            "--or-update",
            installer,
        ])?;
        let mut command = std::process::Command::new("sh");
        command.args([
            "-c",
            "sleep 2; exec flatpak run \"$1\" \"$2\"",
            "artcraft-update",
            &scope,
            &reference,
        ]);
        crate::platform::spawn(&mut command).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let target=std::env::var_os("APPIMAGE").map(PathBuf::from).ok_or("Automatic replacement is available for AppImage installations. Download the new AppImage for a manually built installation.")?;
    let target = fs::canonicalize(target).map_err(|e| e.to_string())?;
    let staged = target.with_extension("AppImage.new");
    let backup = target.with_extension(format!(
        "AppImage.previous-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    if staged.exists() || backup.exists() {
        return Err("An earlier update backup exists beside the AppImage. Move it elsewhere before updating again.".into());
    }
    let mut source = fs::File::open(&update.installer).map_err(|e| e.to_string())?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged)
        .map_err(|e| format!("AppImage location is not writable: {e}"))?;
    let result = (|| {
        std::io::copy(&mut source, &mut output).map_err(|e| e.to_string())?;
        output.sync_all().map_err(|e| e.to_string())?;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
        if hash_file(&staged)? != update.digest {
            return Err("Copied update checksum failed".into());
        }
        Ok(())
    })();
    drop(output);
    if let Err(e) = result {
        let _ = fs::remove_file(staged);
        return Err(e);
    }
    fs::rename(&target, &backup).map_err(|e| e.to_string())?;
    if let Err(e) = fs::rename(&staged, &target) {
        let _ = fs::rename(&backup, &target);
        return Err(e.to_string());
    }
    let mut command = std::process::Command::new("sh");
    command
        .args(["-c", "sleep 2; exec \"$1\"", "artcraft-update"])
        .arg(&target);
    if let Err(e) = crate::platform::spawn(&mut command) {
        let _ = fs::rename(&target, &staged);
        let _ = fs::rename(&backup, &target);
        return Err(e.to_string());
    }
    // Retain the previous version for manual recovery; never silently delete it.
    Ok(())
}
