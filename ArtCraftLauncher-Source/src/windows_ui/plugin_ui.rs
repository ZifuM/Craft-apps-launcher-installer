use super::*;
impl Launcher {
    pub(super) fn plugins_page(&mut self, ui: &mut egui::Ui, app: &AppInfo) {
        self.v2_plugins(ui, *app);
    }
}
