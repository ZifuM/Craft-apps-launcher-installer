use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum UiTheme {
    Legacy,
    #[default]
    V2,
}
impl UiTheme {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Legacy => "Legacy",
            Self::V2 => "V2",
        }
    }
}
static ACTIVE_V2: AtomicBool = AtomicBool::new(false);
static RESTART: AtomicBool = AtomicBool::new(false);
pub(crate) fn set_active(theme: UiTheme) {
    ACTIVE_V2.store(theme == UiTheme::V2, Ordering::Relaxed);
}
pub(crate) fn is_v2() -> bool {
    ACTIVE_V2.load(Ordering::Relaxed)
}
pub(crate) fn restart_requested() -> bool {
    RESTART.load(Ordering::Relaxed)
}

impl Launcher {
    pub(crate) fn theme_picker(&mut self, ui: &mut egui::Ui) {
        ui.heading(tr("Interface theme"));
        ui.label(tr(
            "Choose a layout. Theme changes take effect after restarting Master Suite.",
        ));
        ui.add_space(20.0);
        for (theme, description) in [
            (
                UiTheme::Legacy,
                "The existing interface, with sidebar workspaces, app cards and illustrated banners.",
            ),
            (
                UiTheme::V2,
                "A catalog-style interface with category navigation, featured apps and compact app cards. Available in light and dark modes.",
            ),
        ] {
            ui.push_id(theme.name(), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .radio_value(&mut self.prefs.ui_theme, theme, theme.name())
                        .changed()
                    {
                        save_preferences(&self.prefs);
                    }
                    if theme == self.active_theme {
                        ui.label(RichText::new(tr("Active")).small().color(muted()));
                    }
                });
                ui.label(RichText::new(tr(description)).color(muted()));
                ui.add_space(18.0);
            });
        }
        ui.separator();
        ui.add_space(12.0);
        if self.prefs.ui_theme != self.active_theme {
            ui.label(tr(format!(
                "{} will be used after restart.",
                self.prefs.ui_theme.name()
            )));
            let busy = self.cloud.busy
                || self.direct_cloud.busy
                || self.plugins.busy
                || self.pending_launch.is_some()
                || self.suite_update_busy
                || self
                    .folder_move
                    .as_ref()
                    .is_some_and(|operation| operation.running())
                || self.states.values().any(|state| state.busy.is_some());
            ui.horizontal(|ui| {
                let restart = if self.active_theme == UiTheme::V2 {
                    ui.add_enabled(!busy, ui_v2::primary_button("Restart to apply"))
                } else {
                    ui.add_enabled(!busy, egui::Button::new(tr("Restart to apply")))
                };
                if restart.clicked() {
                    save_preferences(&self.prefs);
                    if read_preferences().ui_theme != self.prefs.ui_theme {
                        self.toast = Some("Could not save the selected theme. Check access to your settings folder and try again.".into());
                        return;
                    }
                    self.window_state.flush();
                    RESTART.store(true, Ordering::Relaxed);
                    self.tray_quit_requested = true;
                    ui.ctx().request_repaint();
                }
                if ui.button(tr("Keep current theme")).clicked() {
                    self.prefs.ui_theme = self.active_theme;
                    save_preferences(&self.prefs);
                }
            });
            if busy {
                ui.label(tr(
                    "Finish the current install, backup or move before restarting.",
                ));
            }
        } else {
            ui.label(
                RichText::new(tr(format!(
                    "Current interface: {}",
                    self.active_theme.name()
                )))
                .color(muted()),
            );
        }
    }
}

/// Relaunch the installed package, including sandboxed and AppImage distributions.
pub(crate) fn relaunch() -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        if platform::flatpak() {
            Command::new("flatpak-spawn")
                .args(["--host", "flatpak", "run", platform::FLATPAK_ID])
                .spawn()?;
            return Ok(());
        }
        if let Some(image) = std::env::var_os("APPIMAGE") {
            let mut command = Command::new(image);
            for key in ["LD_LIBRARY_PATH", "LD_PRELOAD", "APPDIR", "APPIMAGE", "OWD"] {
                command.env_remove(key);
            }
            command.spawn()?;
            return Ok(());
        }
    }
    Command::new(std::env::current_exe()?).spawn()?;
    Ok(())
}
