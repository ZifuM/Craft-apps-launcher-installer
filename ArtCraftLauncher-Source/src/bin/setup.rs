#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{fs, io::Write, path::PathBuf, process::Command};

const LAUNCHER: &[u8] = include_bytes!("../../target/release/artcraft-launcher.exe");
const ICON: &[u8] = include_bytes!("../../assets/artcraft-icon.ico");

fn main() {
    if let Err(error) = install() {
        let message = format!("ArtCraft Master Suite could not be installed.\n\n{error}");
        #[cfg(target_os = "windows")]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
            let title: Vec<u16> = "ArtCraft Master Suite Setup\0".encode_utf16().collect();
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
        std::process::exit(1);
    }
}

fn install() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    let _setup_guard = setup_lock()?;
    wait_for_launcher()?;
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(dirs_fallback)
        .ok_or("Windows local application data folder was not found")?;
    let folder = base.join("Programs").join("ArtCraft Launcher");
    fs::create_dir_all(&folder)?;
    let destination = folder.join("ArtCraft Launcher.exe");
    #[cfg(target_os = "windows")]
    close_installed_launcher(&folder)?;
    let icon_path = folder.join("ArtCraft Launcher.ico");
    fs::write(&icon_path, ICON)?;
    let temporary = folder.join("ArtCraft Launcher.installing.exe");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(LAUNCHER)?;
    file.sync_all()?;
    // Windows refuses to start an executable that still has a writable handle.
    // Close the staging file before renaming it or launching the installed app.
    drop(file);
    let backup = folder.join("ArtCraft Launcher.previous.exe");
    if backup.exists() {
        retry_file_operation("Remove previous installation", || fs::remove_file(&backup))?;
    }
    if destination.exists() {
        retry_file_operation("Move the existing installation", || fs::rename(&destination, &backup))?;
    }
    if let Err(error) = retry_file_operation("Put the updated application in place", || fs::rename(&temporary, &destination)) {
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
        .join("ArtCraft Master Suite.lnk");
        let quoted_start_menu = start_menu.to_string_lossy().replace('\'', "''");
        let uninstall = format!(
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -Command Remove-Item -LiteralPath '{quoted_folder}' -Recurse -Force; Remove-Item -LiteralPath '{quoted_start_menu}' -Force -ErrorAction SilentlyContinue; $d=[Environment]::GetFolderPath(''Desktop''); Remove-Item -LiteralPath (Join-Path $d 'ArtCraft Master Suite.lnk') -Force -ErrorAction SilentlyContinue; Remove-ItemProperty -LiteralPath 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -Name 'ArtCraftMasterSuite' -ErrorAction SilentlyContinue; Remove-Item -LiteralPath 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ArtCraftLauncher' -Force"
        ).replace('\'', "''");
        let version = env!("CARGO_PKG_VERSION");
        let script = format!(
            "$shell = New-Object -ComObject WScript.Shell; $start = Join-Path $env:APPDATA 'Microsoft\\Windows\\Start Menu\\Programs\\ArtCraft Master Suite.lnk'; $desktop = Join-Path ([Environment]::GetFolderPath('Desktop')) 'ArtCraft Master Suite.lnk'; foreach ($shortcutPath in @($start, $desktop)) {{ $s = $shell.CreateShortcut($shortcutPath); $s.TargetPath = '{quoted_exe}'; $s.WorkingDirectory = Split-Path '{quoted_exe}'; $s.IconLocation = '{quoted_icon},0'; $s.Save() }}; foreach ($legacy in @((Join-Path $env:APPDATA 'Microsoft\\Windows\\Start Menu\\Programs\\ArtCraft Launcher.lnk'), (Join-Path ([Environment]::GetFolderPath('Desktop')) 'ArtCraft Launcher.lnk'))) {{ if (Test-Path -LiteralPath $legacy) {{ $old = $shell.CreateShortcut($legacy); if ($old.TargetPath -ieq '{quoted_exe}') {{ Remove-Item -LiteralPath $legacy -Force }} }} }}; $pinned = Join-Path $env:APPDATA 'Microsoft\\Internet Explorer\\Quick Launch\\User Pinned\\TaskBar'; if (Test-Path $pinned) {{ Get-ChildItem -LiteralPath $pinned -Filter '*.lnk' | ForEach-Object {{ $s = $shell.CreateShortcut($_.FullName); if ($s.TargetPath -ieq '{quoted_exe}') {{ $s.IconLocation = '{quoted_icon},0'; $s.Save() }} }} }}; $key = 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ArtCraftLauncher'; New-Item $key -Force | Out-Null; Set-ItemProperty $key DisplayName 'ArtCraft Master Suite'; Set-ItemProperty $key DisplayVersion '{version}'; Set-ItemProperty $key Publisher 'ArtCraft Master Suite'; Set-ItemProperty $key DisplayIcon '{quoted_icon}'; Set-ItemProperty $key InstallLocation '{quoted_folder}'; $uninstall = '{uninstall}'; Set-ItemProperty $key UninstallString $uninstall"
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
        retry_file_operation("Start Master Suite", || Command::new(&destination).spawn().map(|_| ()))?;
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

// Wait for the old process and its single-instance mutex to close before replacing
// the executable. The installer runs separately, so no locked binary is overwritten.
fn wait_for_launcher() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--wait-pid") {
        let pid: u32 = args.get(index + 1).ok_or("Missing launcher process ID")?.parse()?;
        #[cfg(target_os = "windows")]
        unsafe {
            use windows_sys::Win32::{Foundation::{CloseHandle, WAIT_OBJECT_0}, System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE}};
            let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if handle.is_null() {
                let error = windows_sys::Win32::Foundation::GetLastError();
                if error != windows_sys::Win32::Foundation::ERROR_INVALID_PARAMETER { return Err(format!("Could not wait for Master Suite (Windows error {error})").into()); }
            } else {
                let result = WaitForSingleObject(handle, 60_000);
                CloseHandle(handle);
                if result != WAIT_OBJECT_0 { return Err("Master Suite did not finish closing; update cancelled".into()); }
            }
        }
    }
    Ok(())
}

fn retry_file_operation(label: &str, mut operation: impl FnMut() -> std::io::Result<()>) -> Result<(), Box<dyn std::error::Error>> {
    for attempt in 0..20 {
        match operation() {
            Ok(()) => return Ok(()),
            Err(error) if matches!(error.raw_os_error(), Some(32 | 33)) && attempt < 19 => {
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Err(error) => return Err(format!("{label}: {error}. Close Master Suite and any other setup windows, then try again.").into()),
        }
    }
    unreachable!()
}

#[cfg(target_os = "windows")]
struct SetupHandle(windows_sys::Win32::Foundation::HANDLE);
#[cfg(target_os = "windows")]
impl Drop for SetupHandle {
    fn drop(&mut self) { unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0); } }
}
#[cfg(target_os = "windows")]
fn setup_lock() -> Result<SetupHandle, Box<dyn std::error::Error>> {
    use windows_sys::Win32::{Foundation::{GetLastError, ERROR_ALREADY_EXISTS}, System::Threading::CreateMutexW};
    let name: Vec<u16> = "Local\\ArtCraftMasterSuiteSetup\0".encode_utf16().collect();
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        if handle.is_null() { return Err(std::io::Error::last_os_error().into()); }
        let exists = GetLastError() == ERROR_ALREADY_EXISTS;
        let guard = SetupHandle(handle);
        if exists { return Err("Another Master Suite installer is already open. Close it before trying again.".into()); }
        Ok(guard)
    }
}

#[cfg(target_os = "windows")]
fn close_installed_launcher(folder: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use windows_sys::Win32::{Foundation::*, System::{Diagnostics::ToolHelp::*, Threading::*}, UI::WindowsAndMessaging::*};
    unsafe extern "system" fn close_window(hwnd: HWND, pid: LPARAM) -> BOOL {
        let mut owner = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut owner);
            if owner == pid as u32 { PostMessageW(hwnd, WM_CLOSE, 0, 0); }
        }
        1
    }
    let targets = [folder.join("ArtCraft Launcher.exe"), folder.join("ArtCraft Launcher.previous.exe")];
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE { return Err(std::io::Error::last_os_error().into()); }
        let _snapshot = SetupHandle(snapshot);
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut available = Process32FirstW(snapshot, &mut entry);
        let mut waiting = Vec::new();
        while available != 0 {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, 0, entry.th32ProcessID);
            if !handle.is_null() {
                let guard = SetupHandle(handle);
                let mut name = vec![0u16; 32768]; let mut length = name.len() as u32;
                if QueryFullProcessImageNameW(handle, 0, name.as_mut_ptr(), &mut length) != 0 {
                    let path = String::from_utf16_lossy(&name[..length as usize]);
                    if targets.iter().any(|target| target.to_string_lossy().eq_ignore_ascii_case(&path)) {
                        EnumWindows(Some(close_window), entry.th32ProcessID as LPARAM);
                        waiting.push(guard);
                    }
                }
            }
            available = Process32NextW(snapshot, &mut entry);
        }
        for handle in waiting {
            if WaitForSingleObject(handle.0, 15_000) != WAIT_OBJECT_0 {
                return Err("Master Suite is still running. Finish any active installations, choose Quit from its tray menu, and run setup again.".into());
            }
        }
    }
    Ok(())
}
