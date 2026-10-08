#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{fs, io::Write, path::PathBuf, process::Command};

const LAUNCHER: &[u8] = include_bytes!("../../target/release/artcraft-launcher.exe");
const ICON: &[u8] = include_bytes!("../../assets/artcraft-icon.ico");

fn main() {
    if let Err(error) = install() {
        let message = format!("ArtCraft Launcher could not be installed.\n\n{error}");
        #[cfg(target_os = "windows")]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
            let title: Vec<u16> = "ArtCraft Launcher Setup\0".encode_utf16().collect();
            let text: Vec<u16> = format!("{message}\0").encode_utf16().collect();
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
        #[cfg(not(target_os = "windows"))]
        eprintln!("{message}");
    }
}

fn install() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(dirs_fallback)
        .ok_or("Windows local application data folder was not found")?;
    let folder = base.join("Programs").join("ArtCraft Launcher");
    fs::create_dir_all(&folder)?;
    let destination = folder.join("ArtCraft Launcher.exe");
    let icon_path = folder.join("ArtCraft Launcher.ico");
    fs::write(&icon_path, ICON)?;
    let temporary = folder.join("ArtCraft Launcher.installing.exe");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(LAUNCHER)?;
    file.sync_all()?;
    let backup = folder.join("ArtCraft Launcher.previous.exe");
    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    if destination.exists() {
        fs::rename(&destination, &backup)?;
    }
    if let Err(error) = fs::rename(&temporary, &destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, &destination);
        }
        return Err(error.into());
    }
    if backup.exists() {
        let _ = fs::remove_file(backup);
    }

    #[cfg(target_os = "windows")]
    {
        let quoted_exe = destination.to_string_lossy().replace('\'', "''");
        let quoted_folder = folder.to_string_lossy().replace('\'', "''");
        let quoted_icon = icon_path.to_string_lossy().replace('\'', "''");
        let start_menu = PathBuf::from(
            std::env::var_os("APPDATA").ok_or("Windows app data folder was not found")?,
        )
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("ArtCraft Launcher.lnk");
        let quoted_start_menu = start_menu.to_string_lossy().replace('\'', "''");
        let uninstall = format!(
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -Command Remove-Item -LiteralPath '{quoted_folder}' -Recurse -Force; Remove-Item -LiteralPath '{quoted_start_menu}' -Force -ErrorAction SilentlyContinue; $d=[Environment]::GetFolderPath(''Desktop''); Remove-Item -LiteralPath (Join-Path $d 'ArtCraft Launcher.lnk') -Force -ErrorAction SilentlyContinue; Remove-Item -LiteralPath 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ArtCraftLauncher' -Force"
        ).replace('\'', "''");
        let script = format!(
            "$shell = New-Object -ComObject WScript.Shell; $start = Join-Path $env:APPDATA 'Microsoft\\Windows\\Start Menu\\Programs\\ArtCraft Launcher.lnk'; $desktop = Join-Path ([Environment]::GetFolderPath('Desktop')) 'ArtCraft Launcher.lnk'; foreach ($shortcutPath in @($start, $desktop)) {{ $s = $shell.CreateShortcut($shortcutPath); $s.TargetPath = '{quoted_exe}'; $s.WorkingDirectory = Split-Path '{quoted_exe}'; $s.IconLocation = '{quoted_icon},0'; $s.Save() }}; $pinned = Join-Path $env:APPDATA 'Microsoft\\Internet Explorer\\Quick Launch\\User Pinned\\TaskBar'; if (Test-Path $pinned) {{ Get-ChildItem -LiteralPath $pinned -Filter '*.lnk' | ForEach-Object {{ $s = $shell.CreateShortcut($_.FullName); if ($s.TargetPath -ieq '{quoted_exe}') {{ $s.IconLocation = '{quoted_icon},0'; $s.Save() }} }} }}; $key = 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ArtCraftLauncher'; New-Item $key -Force | Out-Null; Set-ItemProperty $key DisplayName 'ArtCraft Launcher'; Set-ItemProperty $key DisplayVersion '0.1.0'; Set-ItemProperty $key Publisher 'ArtCraft Launcher'; Set-ItemProperty $key DisplayIcon '{quoted_icon}'; Set-ItemProperty $key InstallLocation '{quoted_folder}'; $uninstall = '{uninstall}'; Set-ItemProperty $key UninstallString $uninstall"
        );
        let status = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .creation_flags(0x08000000)
            .status()?;
        if !status.success() {
            return Err("Windows could not create the Start menu entry".into());
        }
        let _ = Command::new("cmd.exe")
            .args(["/c", "start", "", destination.to_str().unwrap_or("")])
            .creation_flags(0x08000000)
            .spawn();
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn dirs_fallback() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(target_os = "windows")]
fn dirs_fallback() -> Option<PathBuf> {
    None
}

#[cfg(target_os = "windows")]
trait CreationFlags {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

#[cfg(target_os = "windows")]
impl CreationFlags for Command {
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        std::os::windows::process::CommandExt::creation_flags(self, flags)
    }
}
