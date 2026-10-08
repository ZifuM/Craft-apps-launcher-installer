use super::*;
use crate::app_screens::{action, ink_for, text_at};

impl Launcher {
    pub(super) fn app_detail(&mut self, ui: &mut egui::Ui, id: &'static str) {
        if self.prefs.classic_app_screens { self.classic_app_detail(ui, id); return; }
        let Some(app) = app_by_id(id).copied() else { self.page = Page::YourApps; return; };
        let state = self.states.get(id).cloned().unwrap_or_default();
        ui.push_id(id, |ui| {
            ui.horizontal(|ui| {
                if ui.link(if self.detail_parent == Page::YourApps { "Your apps" } else { "App Manager" }).clicked() { self.page = self.detail_parent; }
                ui.label(RichText::new("/  Workspace").color(muted()));
            });
            ui.add_space(18.0);
            let (hero, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 208.0), egui::Sense::hover());
            ui.painter().rect_filled(hero, 16.0, mix_color(panel(), app.tint, 0.18));
            ui.painter().rect_stroke(hero, 16.0, egui::Stroke::new(1.0_f32, mix_color(border(), app.tint, 0.38)), egui::StrokeKind::Inside);
            let logo = egui::Rect::from_min_size(hero.min + Vec2::new(22.0, 26.0), Vec2::splat(64.0));
            let mut logo_ui = ui.new_child(egui::UiBuilder::new().max_rect(logo));
            self.app_logo(&mut logo_ui, &app, 64.0);
            let text_width = (hero.width() - 132.0).max(1.0);
            text_at(ui, egui::Rect::from_min_size(hero.min + Vec2::new(108.0, 24.0), Vec2::new(text_width, 18.0)), app.category, 10.0, readable_app_color(app.tint));
            text_at(ui, egui::Rect::from_min_size(hero.min + Vec2::new(108.0, 46.0), Vec2::new(text_width, 36.0)), app.name, 29.0, foreground());
            text_at(ui, egui::Rect::from_min_size(hero.min + Vec2::new(108.0, 87.0), Vec2::new(text_width, 22.0)), app.blurb, 13.0, muted());
            let installed = state.installed.as_deref().map(|v| format!("Installed  v{v}")).unwrap_or_else(|| "Not installed".into());
            let latest = state.latest.as_deref().map(|v| format!("Latest  v{v}")).unwrap_or_else(|| "Release not checked".into());
            text_at(ui, egui::Rect::from_min_size(hero.min + Vec2::new(22.0, 122.0), Vec2::new(hero.width()-44.0, 20.0)), format!("{installed}   /   {latest}"), 12.0, muted());
            let button = egui::Rect::from_min_size(hero.min + Vec2::new(22.0, 156.0), Vec2::new(132.0, 36.0));
            let enabled = state.busy.is_none() && (state.installed.is_some() || app.has_release);
            let label = if state.busy.is_some() { "Working…" } else if state.installed.is_some() { "Open app" } else if app.has_release { "Install app" } else { "Coming soon" };
            if action(ui, button, "workspace-open", label, app.tint, ink_for(app.tint), enabled).clicked() {
                if state.installed.is_some() { self.launch(app, None); } else { self.install(app); }
            }
            let controls = egui::Rect::from_min_max(hero.min+Vec2::new(168.0,156.0),hero.right_bottom()-Vec2::new(22.0,16.0));
            let mut controls_ui = ui.new_child(egui::UiBuilder::new().max_rect(controls).layout(egui::Layout::left_to_right(egui::Align::Center)));
            controls_ui.add_enabled_ui(state.busy.is_none(), |ui| {
                let update_available = state.installed.is_some() && state.latest.is_some() && state.latest != state.installed;
                if icon_button(ui, ButtonIcon::UpdateArrow, if update_available { "Install available update" } else { "Check for updates" }, readable_app_color(app.tint)).clicked() {
                    if update_available { self.install(app); } else { self.check_app_release(app); }
                }
                if state.installed.is_some() && icon_button(ui, ButtonIcon::Delete, "Uninstall app", theme_rgb(213,83,93)).clicked() { self.show_remove = Some(id.into()); }
            });
            if icon_button(&mut controls_ui, ButtonIcon::Github, "Source repository", muted()).clicked() { open_url(&format!("{REPO}/{}",release_slug(id))); }
            if icon_button(&mut controls_ui, ButtonIcon::Globe, "App website", muted()).clicked() { open_url("https://getartcraft.com/apps"); }
            if let Some(progress) = &state.busy { ui.horizontal(|ui| { ui.spinner(); ui.label(progress); }); }
            if let Some(error) = &state.error { ui.label(RichText::new(error).color(theme_rgb(255,156,135))); }
            ui.add_space(22.0);
            let count = self.projects.iter().filter(|p|p.app.id==id).count();
            let mut tab = *self.workspace_tabs.get(id).unwrap_or(&0);
            let (tabs,_) = ui.allocate_exact_size(Vec2::new(ui.available_width(),44.0),egui::Sense::hover());
            let width = ((tabs.width()-16.0)/3.0).min(190.0);
            for (index,label) in [format!("Projects  ({count})"),"Asset management".into(),"Plugin management".into()].iter().enumerate() {
                let rect=egui::Rect::from_min_size(tabs.min+Vec2::new(index as f32*(width+8.0),0.0),Vec2::new(width,40.0));
                let active=tab==index as u8;
                if action(ui,rect,("workspace-tab",index),label,if active {mix_color(panel(),app.tint,0.16)} else {panel()},if active {foreground()} else {muted()},true).clicked() {tab=index as u8;}
                if active {ui.painter().rect_filled(egui::Rect::from_min_size(rect.left_bottom()+Vec2::new(12.0,2.0),Vec2::new(width-24.0,2.0)),1.0,app.tint);}
            }
            self.workspace_tabs.insert(id.into(),tab);
            ui.add_space(22.0);
            match tab {
                1 => self.workspace_placeholder(ui,&app,false),
                2 => self.workspace_placeholder(ui,&app,true),
                _ => self.workspace_projects(ui,&app,count),
            }
        });
    }

    fn workspace_projects(&mut self, ui:&mut egui::Ui, app:&AppInfo, count:usize) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Project library").size(20.0).color(foreground()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                if ui.link("Browse all projects").clicked() {
                    self.project_filter=app.name.into();self.projects_tab=false;self.project_scope="All projects".into();self.search.clear();self.page=Page::Projects;
                }
            });
        });
        ui.label(RichText::new(format!("{count} projects in your connected folders. Pick up where you left off.")).size(12.0).color(muted()));
        ui.add_space(16.0);
        let (row,_) = ui.allocate_exact_size(Vec2::new(ui.available_width(),40.0),egui::Sense::hover());
        let search_rect=egui::Rect::from_min_max(row.min,egui::pos2(row.right()-138.0,row.bottom()));
        let mut search_ui=ui.new_child(egui::UiBuilder::new().max_rect(search_rect));
        search_ui.add_sized(search_rect.size(),egui::TextEdit::singleline(self.workspace_search.entry(app.id.into()).or_default()).hint_text("Search projects by name or format…").margin(Vec2::new(12.0,10.0)));
        let mut view_ui=ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(egui::pos2(row.right()-122.0,row.top()+2.0),Vec2::new(122.0,36.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        self.project_view_controls(&mut view_ui);
        ui.add_space(16.0);
        let query=self.workspace_search.get(app.id).map(|s|s.trim().to_lowercase()).unwrap_or_default();
        let projects:Vec<_>=self.projects.iter().filter(|p|p.app.id==app.id && (p.title.to_lowercase().contains(&query)||p.path.to_string_lossy().to_lowercase().contains(&query))).cloned().collect();
        if projects.is_empty() {
            egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32,border())).corner_radius(14).inner_margin(28).show(ui,|ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(if query.is_empty(){"Your next project starts here"}else{"No matching projects"}).size(19.0));
                ui.add_space(6.0);
                ui.label(RichText::new(if query.is_empty(){"Connect a project folder to see previews and open your work from this workspace."}else{"Try another name or file format."}).color(muted()));
                ui.add_space(14.0);
                if !query.is_empty() && secondary_button(ui,"Clear search").clicked(){self.workspace_search.remove(app.id);}
                if query.is_empty() && secondary_button(ui,"Manage project folders").clicked(){self.projects_tab=true;self.page=Page::Projects;}
            });
        } else { self.project_gallery(ui,projects); }
        ui.add_space(22.0);
        ui.collapsing("Supported file formats",|ui| {ui.label(RichText::new(app.filetypes.iter().map(|e|format!(".{e}")).collect::<Vec<_>>().join("   ")).color(muted()));});
    }

    fn workspace_placeholder(&mut self,ui:&mut egui::Ui,app:&AppInfo,plugins:bool) {
        egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32,border())).corner_radius(16).inner_margin(26).show(ui,|ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("COMING SOON").size(10.0).strong().color(readable_app_color(app.tint)));
            ui.add_space(10.0);
            ui.label(RichText::new(if plugins {format!("Extend your {} workspace",app.name)}else{format!("A home for your {} assets",app.name)}).size(24.0).color(foreground()));
            ui.add_space(8.0);
            ui.label(RichText::new(if plugins {"Plugin management is planned for a future release. Installing, enabling and updating plugins is not available yet."}else{"Asset management is planned for a future release. Importing and organizing assets is not available yet."}).color(muted()));
            ui.add_space(24.0);
            let sections=if plugins {[("Installed plugins","A dedicated place for your app extensions."),("Discover extensions","Find tools that complement your workflow."),("Updates & compatibility","Review plugin versions and app compatibility.")]} else {[("Your asset library","Keep reusable files together for this app."),("Collections","Organize resources by project or purpose."),("Asset details","Browse previews and resource information.")]};
            // Informational panels only: no pretend install/import actions or data.
            for (title,description) in sections {
                egui::Frame::new().fill(card()).stroke(egui::Stroke::new(1.0_f32,border())).corner_radius(10).inner_margin(18).show(ui,|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(title).size(15.0).color(foreground()));
                    ui.add_space(4.0);
                    ui.label(RichText::new(description).size(12.0).color(muted()));
                });
                ui.add_space(10.0);
            }
        });
    }
}

