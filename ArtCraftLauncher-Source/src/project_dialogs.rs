use super::*;

fn dialog_frame() -> egui::Frame {
    egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32,border()))
        .corner_radius(18).inner_margin(26)
}
fn dialog_heading(ui: &mut egui::Ui, title: &str, description: &str, destructive: bool) {
    ui.set_width(440.0);
    ui.horizontal(|ui| {
        let (rect,_) = ui.allocate_exact_size(Vec2::splat(40.0),egui::Sense::hover());
        let color=if destructive {theme_rgb(230,102,112)} else {ACCENT};
        ui.painter().rect_filled(rect,12.0,color.gamma_multiply(0.14));
        ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,if destructive {"!"}else{"Aa"},egui::FontId::proportional(20.0),color);
        ui.add_space(8.0);
        ui.label(RichText::new(tr(title)).size(23.0).strong().color(foreground()));
    });
    ui.add_space(14.0);
    ui.label(RichText::new(tr(description)).size(13.0).color(muted()));
    ui.add_space(18.0);
}
fn file_summary(ui: &mut egui::Ui, name: &str, path: &Path) {
    egui::Frame::new().fill(card()).corner_radius(10).inner_margin(14).show(ui,|ui| {
        ui.set_width(ui.available_width());
        ui.add(egui::Label::new(RichText::new(name).strong().color(foreground())).truncate()).on_hover_text(name);
        let parent=path.parent().unwrap_or(path).display().to_string();
        ui.add(egui::Label::new(RichText::new(&parent).size(11.0).color(muted())).truncate()).on_hover_text(parent);
    });
}
impl Launcher {
    pub(super) fn project_dialogs(&mut self,ctx:&egui::Context) {
        if let Some(id)=self.show_remove.clone() {
            if let Some(app)=app_by_id(&id) {
                let mut cancel=false;let mut remove=false;
                let modal=egui::Modal::new(egui::Id::new("remove-app-confirmation")).backdrop_color(Color32::from_black_alpha(155)).frame(dialog_frame()).show(ctx,|ui| {
                    dialog_heading(ui,"Uninstall app","Your creative projects are kept in their original folders.",true);
                    ui.label(RichText::new(app.name).size(19.0).color(readable_app_color(app.tint)));
                    ui.add_space(24.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                        remove=danger_button(ui,"Remove app").clicked();
                        cancel=secondary_button(ui,"Cancel").clicked();
                    });
                });
                if remove {self.remove(&id);} else if cancel||modal.should_close() {self.show_remove=None;}
            }
            return;
        }
        let mut rename=None;let mut cancel=false;
        if let Some(draft)=self.show_project_rename.as_mut() {
            let modal=egui::Modal::new(egui::Id::new("rename-project-dialog")).backdrop_color(Color32::from_black_alpha(155)).frame(dialog_frame()).show(ctx,|ui| {
                dialog_heading(ui,"Rename project","Choose a new name for this project file.",false);
                file_summary(ui,&draft.path.file_name().unwrap_or_default().to_string_lossy(),&draft.path);
                ui.add_space(18.0);
                ui.label(RichText::new(tr("File name")).size(12.0).color(muted()));
                let response=ui.add_sized([440.0,40.0],egui::TextEdit::singleline(&mut draft.name).id_salt("project-new-name").margin(Vec2::new(12.0,10.0)));
                if !draft.focused { response.request_focus(); draft.focused=true; }
                ui.label(RichText::new(tr("The file extension stays the same.")).size(11.0).color(muted()));
                let valid=!draft.name.trim().is_empty() && !draft.name.chars().any(|c|"<>:\"/\\|?*".contains(c)||c.is_control());
                ui.add_space(22.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if ui.add_enabled_ui(valid,|ui|primary_button(ui,"Save name")).inner.clicked() {rename=Some((draft.path.clone(),draft.name.clone()));}
                    cancel=secondary_button(ui,"Cancel").clicked();
                });
                if valid && response.lost_focus() && ui.input(|i|i.key_pressed(egui::Key::Enter)) {rename=Some((draft.path.clone(),draft.name.clone()));}
            });
            if let Some((path,name))=rename {self.rename_project(&path,&name);}
            else if cancel||modal.should_close(){self.show_project_rename=None;}
            return;
        }
        let mut delete=None;let mut cancel=false;
        if let Some(project)=self.show_project_delete.as_ref() {
            let modal=egui::Modal::new(egui::Id::new("delete-project-dialog")).backdrop_color(Color32::from_black_alpha(155)).frame(dialog_frame()).show(ctx,|ui| {
                dialog_heading(ui,"Delete project?","This permanently removes the project file from disk.",true);
                file_summary(ui,&project.title,&project.path);
                ui.add_space(24.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if danger_button(ui,"Delete project").clicked(){delete=Some(project.path.clone());}
                    cancel=secondary_button(ui,"Cancel").clicked();
                });
            });
            if let Some(path)=delete {self.delete_project(&path);}
            else if cancel||modal.should_close(){self.show_project_delete=None;}
        }
    }
}
