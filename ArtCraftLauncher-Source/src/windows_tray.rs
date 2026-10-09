use std::{ptr, sync::mpsc::Sender};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::{Foundation::*, System::{LibraryLoader::GetModuleHandleW, Registry::*}, UI::{Shell::*, WindowsAndMessaging::*}};
use crate::Event;
const CALLBACK: u32 = WM_APP + 71;
const SUBCLASS: usize = 0x415254;
fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
struct State { tx: Sender<Event>, ctx: eframe::egui::Context, minimize: bool, icon: HICON, taskbar_created: u32 }
pub struct Tray { hwnd: HWND, state: Box<State> }
impl Tray {
    pub fn new(frame: &eframe::Frame, tx: Sender<Event>, ctx: eframe::egui::Context, minimize: bool) -> Result<Self, String> {
        let window = frame.window_handle().map_err(|e| e.to_string())?;
        let RawWindowHandle::Win32(handle) = window.as_raw() else { return Err("Windows handle unavailable".into()); };
        unsafe {
            let hwnd = handle.hwnd.get() as HWND;
            let icon = LoadImageW(GetModuleHandleW(ptr::null()), wide("IDI_ICON1").as_ptr(), IMAGE_ICON, 32, 32, LR_DEFAULTCOLOR) as HICON;
            if icon.is_null() { return Err("Could not load the system tray icon".into()); }
            let mut state = Box::new(State { tx, ctx, minimize, icon, taskbar_created: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()) });
            if SetWindowSubclass(hwnd, Some(callback), SUBCLASS, (&mut *state as *mut State) as usize) == 0 {
                DestroyIcon(icon); return Err("Could not attach the system tray menu".into());
            }
            if Shell_NotifyIconW(NIM_ADD, &notification(hwnd, icon)) == 0 {
                RemoveWindowSubclass(hwnd, Some(callback), SUBCLASS); DestroyIcon(icon);
                return Err("Windows could not create the tray icon".into());
            }
            Ok(Self { hwnd, state })
        }
    }
    pub fn set_minimize(&mut self, value: bool) { self.state.minimize = value; }
}
impl Drop for Tray {
    fn drop(&mut self) { unsafe {
        Shell_NotifyIconW(NIM_DELETE, &notification(self.hwnd, self.state.icon));
        RemoveWindowSubclass(self.hwnd, Some(callback), SUBCLASS);
        DestroyIcon(self.state.icon);
    } }
}
fn notification(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd; data.uID = 1; data.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
    data.uCallbackMessage = CALLBACK; data.hIcon = icon;
    for (slot, value) in data.szTip.iter_mut().zip(wide("ArtCraft Master Suite")) { *slot = value; }
    data
}
unsafe extern "system" fn callback(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM, _: usize, reference: usize) -> LRESULT {
    unsafe {
        let state = &*(reference as *const State);
        if msg == state.taskbar_created && msg != 0 {
            if Shell_NotifyIconW(NIM_ADD, &notification(hwnd, state.icon)) == 0 {
                ShowWindow(hwnd, SW_RESTORE); SetForegroundWindow(hwnd);
                let _ = state.tx.send(Event::TrayOpen); state.ctx.request_repaint();
            }
        }
        if msg == WM_SIZE && w == SIZE_MINIMIZED as usize && state.minimize {
            ShowWindow(hwnd, SW_HIDE);
        }
        if msg == CALLBACK {
            let action = l as u32;
            if action == WM_LBUTTONUP || action == WM_LBUTTONDBLCLK {
                ShowWindow(hwnd, SW_RESTORE); SetForegroundWindow(hwnd);
                let _ = state.tx.send(Event::TrayOpen); state.ctx.request_repaint();
            } else if action == WM_RBUTTONUP || action == WM_CONTEXTMENU {
                let menu = CreatePopupMenu();
                if !menu.is_null() {
                    AppendMenuW(menu, MF_STRING, 1, wide(&crate::localization::tr("Open ArtCraft Master Suite")).as_ptr());
                    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
                    AppendMenuW(menu, MF_STRING, 2, wide(&crate::localization::tr("Quit")).as_ptr());
                    let mut point = POINT { x: 0, y: 0 }; GetCursorPos(&mut point);
                    SetForegroundWindow(hwnd);
                    let selected = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, ptr::null());
                    DestroyMenu(menu); PostMessageW(hwnd, WM_NULL, 0, 0);
                    if selected == 1 { ShowWindow(hwnd, SW_RESTORE); SetForegroundWindow(hwnd); let _ = state.tx.send(Event::TrayOpen); }
                    if selected == 2 { let _ = state.tx.send(Event::TrayQuit); PostMessageW(hwnd, WM_CLOSE, 0, 0); }
                    state.ctx.request_repaint();
                }
            }
            return 0;
        }
        DefSubclassProc(hwnd, msg, w, l)
    }
}

pub fn set_startup(enabled: bool) -> Result<(), String> {
    let path = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let name = wide("ArtCraftMasterSuite");
    unsafe {
        let mut key = ptr::null_mut();
        let result = RegCreateKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, ptr::null(), 0, KEY_SET_VALUE, ptr::null(), &mut key, ptr::null_mut());
        if result != ERROR_SUCCESS { return Err(format!("Could not change Windows startup settings ({result})")); }
        let result = if enabled {
            let exe = match std::env::current_exe() { Ok(path) => path, Err(e) => { RegCloseKey(key); return Err(e.to_string()); } };
            let command = wide(&format!("\"{}\" --tray", exe.display()));
            RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, command.as_ptr().cast(), (command.len() * 2) as u32)
        } else { RegDeleteValueW(key, name.as_ptr()) };
        RegCloseKey(key);
        if result == ERROR_SUCCESS || (!enabled && result == ERROR_FILE_NOT_FOUND) { Ok(()) }
        else { Err(format!("Could not change Windows startup settings ({result})")) }
    }
}

pub struct Instance(HANDLE);
impl Drop for Instance { fn drop(&mut self) { unsafe { CloseHandle(self.0); } } }
pub fn single_instance(silent: bool) -> Option<Instance> {
    unsafe {
        let handle = windows_sys::Win32::System::Threading::CreateMutexW(ptr::null(), 0, wide("Local\\ArtCraftMasterSuite").as_ptr());
        if handle.is_null() { return Some(Instance(handle)); }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            if !silent {
                let hwnd = FindWindowW(ptr::null(), wide("ArtCraft Master Suite").as_ptr());
                if !hwnd.is_null() { ShowWindow(hwnd, SW_RESTORE); SetForegroundWindow(hwnd); }
            }
            None
        } else { Some(Instance(handle)) }
    }
}
