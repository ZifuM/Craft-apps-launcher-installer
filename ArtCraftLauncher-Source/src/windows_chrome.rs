use eframe::egui::{self, Color32};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::{Foundation::HWND, Graphics::Dwm::*};

/// Keep the native caption flat and identical to the app toolbar, including
/// after activation changes or restoring decorations after the startup splash.
pub(crate) fn sync(
    ctx: &egui::Context,
    frame: &eframe::Frame,
    background: Color32,
    foreground: Color32,
    dark: bool,
    decorated: bool,
) {
    let Ok(window) = frame.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = window.as_raw() else {
        return;
    };
    let background = colorref(background);
    let foreground = colorref(foreground);
    let focused = ctx.input(|input| input.viewport().focused.unwrap_or(false));
    let key = (
        handle.hwnd.get(),
        background,
        foreground,
        dark,
        focused,
        decorated,
    );
    let id = egui::Id::new("native-caption-colors");
    if ctx.data(|data| data.get_temp::<(isize, u32, u32, bool, bool, bool)>(id)) == Some(key) {
        return;
    }

    let hwnd = handle.hwnd.get() as HWND;
    // DWM takes COLORREF values, not ARGB. Explicit colors bypass the system
    // accent palette; NONE prevents Mica or wallpaper tinting the caption.
    // Older Windows versions safely ignore attributes they do not support.
    for (attribute, value) in [
        (DWMWA_USE_IMMERSIVE_DARK_MODE, u32::from(dark)),
        (DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_NONE as u32),
        (DWMWA_CAPTION_COLOR, background),
        (DWMWA_TEXT_COLOR, foreground),
        (DWMWA_BORDER_COLOR, background),
    ] {
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                attribute as u32,
                (&value as *const u32).cast(),
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
    ctx.data_mut(|data| data.insert_temp(id, key));
}

fn colorref(color: Color32) -> u32 {
    u32::from(color.r()) | (u32::from(color.g()) << 8) | (u32::from(color.b()) << 16)
}
