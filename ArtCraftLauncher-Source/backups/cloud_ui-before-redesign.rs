use super::*;

pub(super) fn paint_cloud(p: &egui::Painter, r: egui::Rect, color: Color32) {
    let c=r.center(); let stroke=egui::Stroke::new(1.5_f32,color);
    let points=[(-7.0,5.0),(-9.0,2.0),(-8.0,-1.0),(-5.0,-2.0),(-4.0,-5.0),(0.0,-7.0),(4.0,-5.0),(5.0,-2.0),(8.0,-1.0),(9.0,2.0),(7.0,5.0),(-7.0,5.0)];
    p.add(egui::Shape::line(points.iter().map(|(x,y)|c+Vec2::new(*x,*y)).collect(),stroke));
}
fn surface() -> egui::Frame {
    egui::Frame::new().fill(card()).stroke(egui::Stroke::new(1.0_f32,border())).corner_radius(12).inner_margin(16)
}
fn status_color(status:&str)->Color32 {
    match status {"Synced"=>Color32::from_rgb(88,198,151),"Needs attention"=>Color32::from_rgb(235,105,112),"Pending upload"|"Syncing"=>ACCENT,_=>muted()}
}
impl Launcher {
    pub(super) fn project_cloud_badge(&mut self, ui:&mut egui::Ui, project:&Project) {
        let status=self.cloud.status(project);let color=status_color(status);
        let (rect,response)=ui.allocate_exact_size(Vec2::splat(28.0),egui::Sense::click());
        paint_cloud(ui.painter(),rect,color);
        if status=="Synced" {ui.painter().text(rect.right_bottom()-Vec2::splat(4.0),egui::Align2::CENTER_CENTER,"✓",egui::FontId::proportional(12.0),color);}
        if response.on_hover_text(format!("{}: {}",tr("Cloud backup"),tr(status))).clicked(){self.page=Page::Cloud;}
    }
    fn cloud_account_card(&mut self,ui:&mut egui::Ui,provider:cloud::Provider,expanded:bool){
        let connected=self.cloud.credentials.contains_key(&provider);
        let (r,_) = ui.allocate_exact_size(Vec2::new(ui.available_width(),if expanded{136.0}else{100.0}),egui::Sense::hover());
        ui.painter().rect_filled(r,12.0,card());
        ui.painter().rect_stroke(r,12.0,egui::Stroke::new(1.0_f32,border()),egui::StrokeKind::Inside);
        let title=egui::Rect::from_min_size(r.min+Vec2::new(14.0,12.0),Vec2::new(r.width()-28.0,22.0));
        app_screens::text_at(ui,title,provider.name(),15.0,foreground());
        let status=self.cloud.credentials.get(&provider).map(|c|c.account.clone()).unwrap_or_else(||tr(if provider.configured(){"Not connected"}else{"Not enabled in this build yet"}));
        app_screens::text_at(ui,egui::Rect::from_min_size(r.min+Vec2::new(14.0,35.0),Vec2::new(r.width()-28.0,18.0)),&status,11.0,muted()).on_hover_text(status);
        let action=egui::Rect::from_min_size(r.min+Vec2::new(14.0,62.0),Vec2::new((r.width()-28.0).min(132.0),26.0));
        if app_screens::action(ui,action,(provider,"connect"),if connected{"Disconnect"}else{"Connect"},if connected{panel()}else{ACCENT},if connected{foreground()}else{Color32::WHITE},!self.cloud.busy&&(connected||provider.configured())).clicked(){
            if connected{self.cloud.disconnect(provider);}else{self.cloud.connect(provider);}
        }
        if connected{
            let open=egui::Rect::from_min_max(egui::pos2(action.right()+6.0,action.top()),egui::pos2(r.right()-14.0,action.bottom()));
            if app_screens::action(ui,open,(provider,"open"),"Open",panel(),foreground(),true).clicked(){open_url(provider.website());}
            let mut row=ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(r.min+Vec2::new(14.0,103.0),r.max-Vec2::new(14.0,8.0))));
            let mut selected=self.cloud.settings.targets.contains(&provider);
            if row.add_enabled(!self.cloud.busy,egui::Checkbox::new(&mut selected,tr("Back up to this account"))).changed(){self.cloud.settings.targets.retain(|p|*p!=provider);if selected{self.cloud.settings.targets.push(provider);}self.cloud.save();}
        }
    }
    pub(super) fn cloud_page(&mut self, ui:&mut egui::Ui) {
        ui.label(RichText::new(tr("YOUR CONNECTED WORKSPACE")).size(10.0).color(muted()));
        ui.add_space(8.0);
        ui.heading(RichText::new(tr("Cloud")).size(30.0));
        ui.label(RichText::new(tr("Back up selected projects to your own cloud account.")).color(muted()));
        ui.add_space(22.0);
        let expanded=!self.cloud.credentials.is_empty();
        if ui.available_width()>=780.0 {
            ui.columns(3,|cols|{for (index,provider) in cloud::PROVIDERS.into_iter().enumerate(){self.cloud_account_card(&mut cols[index],provider,expanded);}});
        }else{for provider in cloud::PROVIDERS{self.cloud_account_card(ui,provider,expanded);ui.add_space(8.0);}}
        ui.add_space(20.0);
        if !self.cloud.message.is_empty()||self.cloud.busy {
            surface().show(ui,|ui|{ui.set_width(ui.available_width());ui.horizontal_wrapped(|ui|{if self.cloud.busy{ui.spinner();}ui.label(tr(&self.cloud.message));if self.cloud.busy&&secondary_button(ui,"Cancel").clicked(){self.cloud.cancel();}});});ui.add_space(16.0);
        }
        ui.horizontal(|ui|{for (tab,label) in [(0,"Projects"),(1,"Backup history"),(2,"Assets")]{
            let (rect,_)=ui.allocate_exact_size(Vec2::new(148.0,38.0),egui::Sense::hover());let active=self.cloud.tab==tab;
            if app_screens::action(ui,rect,("cloud-tab",tab),label,if active{mix_color(card(),ACCENT,0.26)}else{card()},if active{foreground()}else{muted()},true).clicked(){self.cloud.tab=tab;}
        }});
        ui.add_space(22.0);
        match self.cloud.tab {
            0=>self.cloud_projects(ui),
            1=>self.cloud_history(ui),
            _=>{surface().show(ui,|ui|{ui.set_width(ui.available_width());ui.heading(tr("Asset backups"));ui.add_space(8.0);ui.label(RichText::new(tr("Coming with asset management. No assets are uploaded yet.")).color(muted()));});}
        }
    }
    fn cloud_projects(&mut self,ui:&mut egui::Ui){
        surface().show(ui,|ui|{
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(tr("Project backups")).size(18.0));
                ui.label(RichText::new(format!("{} / {}",self.cloud.settings.selected.len(),self.projects.len())).size(12.0).color(muted()));
            });
            ui.add_space(8.0);
            ui.label(RichText::new(tr("Each saved version is backed up separately. Restoring creates a separate local copy.")).size(12.0).color(muted()));
            ui.add_space(18.0);
            ui.add_sized([ui.available_width(),38.0],egui::TextEdit::singleline(&mut self.cloud.search).hint_text(tr("Search projects")).margin(Vec2::new(12.0,10.0)));
            ui.add_space(14.0);
            ui.horizontal_wrapped(|ui|{
                let can_sync=!self.cloud.busy&&!self.cloud.settings.selected.is_empty()&&self.cloud.settings.targets.iter().any(|p|self.cloud.credentials.contains_key(p));
                if ui.add_enabled_ui(can_sync,|ui|primary_button(ui,"Sync now")).inner.clicked(){self.cloud.sync(&self.projects);}
                ui.add_space(8.0);
                if ui.checkbox(&mut self.cloud.settings.automatic,tr("Automatically back up selected projects")).changed(){self.cloud.save();}
            });
        });
        ui.add_space(20.0);
        let query=self.cloud.search.to_lowercase();let projects:Vec<_>=self.projects.iter().filter(|p|p.title.to_lowercase().contains(&query)).cloned().collect();
        if projects.is_empty(){surface().show(ui,|ui|{ui.set_width(ui.available_width());ui.heading(tr("No projects to show"));ui.label(RichText::new(tr("Add a project folder or try another search.")).color(muted()));if secondary_button(ui,"Projects").clicked(){self.page=Page::Projects;}});}
        for project in projects{
            let status=self.cloud.status(&project);let color=status_color(status);
            let (r,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),80.0),egui::Sense::hover());
            ui.painter().rect_filled(r,10.0,card());ui.painter().rect_stroke(r,10.0,egui::Stroke::new(1.0_f32,border()),egui::StrokeKind::Inside);
            let mut check=ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(r.min+Vec2::new(14.0,29.0),Vec2::new(22.0,22.0))));
            let mut selected=self.cloud.settings.selected.contains(&project.path);
            if check.add_enabled(!self.cloud.busy,egui::Checkbox::without_text(&mut selected)).on_hover_text(format!("{}: {}",tr("Cloud backup"),project.title)).changed(){self.cloud.settings.selected.retain(|p|p!=&project.path);if selected{self.cloud.settings.selected.push(project.path.clone());}self.cloud.save();}
            let thumb=egui::Rect::from_min_size(r.min+Vec2::new(48.0,16.0),Vec2::splat(48.0));
            let mut thumbnail=ui.new_child(egui::UiBuilder::new().max_rect(thumb));self.project_thumbnail_at(&mut thumbnail,&project,thumb.size());
            let badge=egui::Rect::from_min_size(egui::pos2(r.right()-162.0,r.center().y-15.0),Vec2::new(148.0,30.0));
            let title=egui::Rect::from_min_max(r.min+Vec2::new(110.0,19.0),egui::pos2(badge.left()-14.0,r.top()+39.0));
            app_screens::text_at(ui,title,&project.title,14.0,foreground()).on_hover_text(&project.title);
            app_screens::text_at(ui,title.translate(Vec2::new(0.0,24.0)),project.path.display().to_string(),11.0,muted()).on_hover_text(project.path.display().to_string());
            ui.painter().rect_filled(badge,8.0,mix_color(card(),color,0.10));
            let badge_text=egui::Rect::from_min_size(badge.min+Vec2::new(10.0,6.0),badge.size()-Vec2::new(20.0,8.0));
            let mut response=app_screens::text_at(ui,badge_text,tr(status),12.0,color);
            for provider in cloud::PROVIDERS {if let Some(error)=self.cloud.errors.get(&(project.path.clone(),provider)){response=response.on_hover_text(format!("{}: {error}",provider.name()));}}
            ui.add_space(8.0);
        }
        let missing=self.cloud.settings.selected.iter().filter(|path|!self.projects.iter().any(|p|&p.path==*path)).count();
        if missing>0{ui.label(format!("{missing} selected files are not in the current project library. Reconnect their project folders to back them up."));}
    }
    fn cloud_history(&mut self,ui:&mut egui::Ui){
        surface().show(ui,|ui|{ui.set_width(ui.available_width());ui.horizontal_wrapped(|ui|{ui.label(RichText::new(tr("Saved versions")).size(18.0));if ui.add_enabled_ui(!self.cloud.busy,|ui|secondary_button(ui,"Refresh")).inner.clicked(){self.cloud.refresh();}});ui.add_space(8.0);ui.label(RichText::new(tr("Restore a separate copy without replacing your current project.")).size(12.0).color(muted()));});
        ui.add_space(16.0);
        if self.cloud.history.is_empty(){surface().show(ui,|ui|{ui.set_width(ui.available_width());ui.label(tr("Refresh to load your saved cloud versions."));});}
        for remote in self.cloud.history.clone(){
            let (r,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),76.0),egui::Sense::hover());ui.painter().rect_filled(r,10.0,card());ui.painter().rect_stroke(r,10.0,egui::Stroke::new(1.0_f32,border()),egui::StrokeKind::Inside);
            let button=egui::Rect::from_min_size(egui::pos2(r.right()-144.0,r.center().y-17.0),Vec2::new(128.0,34.0));
            let title=egui::Rect::from_min_max(r.min+Vec2::new(16.0,16.0),egui::pos2(button.left()-16.0,r.top()+36.0));
            app_screens::text_at(ui,title,remote.original_name(),14.0,foreground()).on_hover_text(remote.original_name());
            app_screens::text_at(ui,title.translate(Vec2::new(0.0,24.0)),format!("{} · {} · {}",remote.provider.name(),remote.modified,format_file_size(remote.bytes)),11.0,muted());
            if app_screens::action(ui,button,("restore",&remote.id),"Restore copy",panel(),foreground(),!self.cloud.busy).clicked(){if let Some(path)=rfd::FileDialog::new().set_file_name(remote.original_name()).save_file(){self.cloud.restore(remote.clone(),path);}}
            ui.add_space(8.0);
        }
    }
}
