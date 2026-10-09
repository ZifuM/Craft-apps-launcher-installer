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
    match status {"Copied"=>Color32::from_rgb(88,198,151),"Folder unavailable"|"Needs attention"=>Color32::from_rgb(235,105,112),"Pending copy"|"Syncing"=>ACCENT,_=>muted()}
}
impl Launcher {
    pub(super) fn project_cloud_badge(&mut self, ui:&mut egui::Ui, project:&Project) {
        let status=self.cloud.status(project);let color=status_color(status);
        let (rect,response)=ui.allocate_exact_size(Vec2::splat(28.0),egui::Sense::click());
        paint_cloud(ui.painter(),rect,color);
        if status=="Copied" {ui.painter().text(rect.right_bottom()-Vec2::splat(4.0),egui::Align2::CENTER_CENTER,"✓",egui::FontId::proportional(12.0),color);}
        if response.on_hover_text(format!("{}: {}",tr("Cloud backup"),tr(if status=="Copied"{"Copied to sync folder; upload status is managed by the provider desktop app"}else{status}))).clicked(){self.page=Page::Cloud;}
    }
    pub(super) fn cloud_page(&mut self,ui:&mut egui::Ui){
        use app_screens::{action,text_at};
        app_screens::experimental_banner(ui);
        let width=ui.available_width();
        let (header,_)=ui.allocate_exact_size(Vec2::new(width,76.0),egui::Sense::hover());
        text_at(ui,egui::Rect::from_min_size(header.min,Vec2::new(width-300.0,38.0)),"Cloud",30.0,foreground());
        text_at(ui,egui::Rect::from_min_size(header.min+Vec2::new(0.0,43.0),Vec2::new(width,23.0)),"Your projects. Backed up, version by version.",13.0,muted());
        let sync=egui::Rect::from_min_size(egui::pos2(header.right()-128.0,header.top()+2.0),Vec2::new(128.0,36.0));
        let accounts=sync.translate(Vec2::new(-150.0,0.0));
        if action(ui,accounts,"cloud-accounts","Manage folders",card(),foreground(),!self.cloud.busy).clicked(){self.cloud.tab=3;}
        let can_sync=!self.cloud.busy&&!self.cloud.settings.selected.is_empty()&&self.cloud.settings.targets.iter().any(|p|self.cloud.settings.folders.contains_key(p));
        if action(ui,sync,"cloud-sync",if self.cloud.busy{"Working…"}else{"Back up now"},ACCENT,Color32::WHITE,can_sync).clicked(){self.cloud.sync(&self.projects);}
        ui.add_space(12.0);
        let saved=self.projects.iter().filter(|p|self.cloud.status(p)=="Copied").count();
        let selected=self.cloud.settings.selected.len();let pending=self.projects.iter().filter(|p|self.cloud.settings.selected.contains(&p.path)&&self.cloud.status(p)!="Copied").count();
        surface().show(ui,|ui|{
            ui.set_width(ui.available_width());ui.columns(3,|cols|{
                for (index,(number,label)) in [(saved,"Copied locally"),(pending,"Awaiting backup"),(self.cloud.settings.folders.len(),"Sync folders")].into_iter().enumerate(){
                    cols[index].horizontal(|ui|{ui.label(RichText::new(number.to_string()).size(23.0).color(foreground()));ui.add_space(8.0);ui.label(RichText::new(tr(label)).size(12.0).color(muted()));});
                }
            });
        });ui.add_space(20.0);
        ui.horizontal(|ui|{for (tab,label) in [(0,"Files"),(1,"Saved versions"),(3,"Sync folders"),(2,"Assets")]{
            let (rect,_)=ui.allocate_exact_size(Vec2::new(132.0,36.0),egui::Sense::hover());let active=self.cloud.tab==tab;
            if action(ui,rect,("cloud-nav",tab),label,if active{mix_color(card(),ACCENT,0.18)}else{ink()},if active{foreground()}else{muted()},true).clicked(){self.cloud.tab=tab;if tab==1&&!self.cloud.busy&&!self.cloud.settings.folders.is_empty(){self.cloud.refresh();}}
        }});ui.add_space(22.0);
        if self.cloud.busy||!self.cloud.message.is_empty(){
            egui::Frame::new().fill(mix_color(panel(),ACCENT,0.06)).corner_radius(8).inner_margin(12).show(ui,|ui|{
                ui.set_width(ui.available_width());ui.horizontal_wrapped(|ui|{if self.cloud.busy{ui.spinner();}ui.label(RichText::new(tr(&self.cloud.message)).size(12.0));if self.cloud.busy{if secondary_button(ui,"Cancel").clicked(){self.cloud.cancel();}}else if ui.small_button(tr("Dismiss")).clicked(){self.cloud.message.clear();}});
            });ui.add_space(16.0);
        }
        match self.cloud.tab{
            0=>{
                if self.cloud.settings.folders.is_empty(){
                    surface().show(ui,|ui|{ui.set_width(ui.available_width());ui.label(RichText::new(tr("Choose a home for your backups")).size(19.0));ui.add_space(6.0);ui.label(RichText::new(tr("Choose a folder synced by the Google Drive, Dropbox or OneDrive desktop app.")).color(muted()));ui.add_space(14.0);if primary_button(ui,"Choose sync folder").clicked(){self.cloud.tab=3;}});ui.add_space(20.0);
                }
                self.cloud_projects(ui);
                ui.add_space(12.0);ui.label(RichText::new(format!("{} selected · {}",selected,tr("Original files stay on this device"))).size(11.0).color(muted()));
            },
            1=>self.cloud_history(ui),
            3=>self.cloud_accounts_page(ui),
            _=>{surface().show(ui,|ui|{ui.set_width(ui.available_width());ui.add_space(12.0);ui.heading(tr("A place for your creative assets"));ui.add_space(8.0);ui.label(RichText::new(tr("Asset backup will arrive with asset management. Your project backups are available in Files.")).color(muted()));ui.add_space(16.0);if secondary_button(ui,"View files").clicked(){self.cloud.tab=0;}});}
        }
        self.cloud_dialogs(ui.ctx());
    }
    fn cloud_accounts_page(&mut self,ui:&mut egui::Ui){
        ui.label(RichText::new(tr("Sync folders")).size(20.0));ui.add_space(6.0);
        ui.label(RichText::new(tr("Sign in through each provider’s desktop app, then select its synced folder below.")).color(muted()));ui.add_space(20.0);
        for provider in cloud::PROVIDERS{
            let account=self.cloud.settings.folders.get(&provider).map(|c|c.path.display().to_string());let connected=account.is_some();
            let (r,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),if connected{166.0}else{128.0}),egui::Sense::hover());
            ui.painter().rect_filled(r,10.0,card());ui.painter().rect_stroke(r,10.0,egui::Stroke::new(1.0_f32,border()),egui::StrokeKind::Inside);
            let brand=match provider{cloud::Provider::Google=>Color32::from_rgb(66,133,244),cloud::Provider::Dropbox=>Color32::from_rgb(41,117,255),cloud::Provider::OneDrive=>Color32::from_rgb(0,135,214)};
            let icon=egui::Rect::from_min_size(r.min+Vec2::new(18.0,22.0),Vec2::splat(42.0));ui.painter().rect_filled(icon,11.0,mix_color(card(),brand,0.16));paint_cloud(ui.painter(),icon,brand);
            let button=egui::Rect::from_min_size(egui::pos2(r.right()-136.0,r.top()+25.0),Vec2::new(116.0,34.0));
            let text_right=if connected{button.left()-116.0}else{button.left()-18.0};
            let title=egui::Rect::from_min_max(r.min+Vec2::new(76.0,22.0),egui::pos2(text_right,r.top()+44.0));
            app_screens::text_at(ui,title,provider.name(),16.0,foreground());
            let status=account.unwrap_or_else(||tr("Choose a folder managed by the desktop app"));
            app_screens::text_at(ui,title.translate(Vec2::new(0.0,25.0)),&status,12.0,muted()).on_hover_text(status);
            if app_screens::action(ui,button,(provider,"connection"),if connected{"Disconnect"}else{"Choose folder"},if connected{panel()}else{ACCENT},if connected{foreground()}else{Color32::WHITE},!self.cloud.busy).clicked(){if connected{self.cloud.disconnect_provider=Some(provider);}else{self.cloud.connect(provider);}}
            if connected{
                let open=button.translate(Vec2::new(-124.0,0.0));
                if app_screens::action(ui,open,(provider,"browse"),"Open folder",panel(),foreground(),true).clicked(){self.cloud.open_folder(provider);}
                let mut line=ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(r.min+Vec2::new(76.0,85.0),r.max-Vec2::new(20.0,10.0))));
                let mut enabled=self.cloud.settings.targets.contains(&provider);
                if line.add_enabled(!self.cloud.busy,egui::Checkbox::new(&mut enabled,tr("Use for project backups"))).changed(){self.cloud.settings.targets.retain(|p|*p!=provider);if enabled{self.cloud.settings.targets.push(provider);}self.cloud.save();}
            }
            let controls_y = if connected { r.top()+120.0 } else { r.top()+82.0 };
            let download = egui::Rect::from_min_size(egui::pos2(r.left()+18.0,controls_y),Vec2::new(174.0,32.0));
            let launch = download.translate(Vec2::new(184.0,0.0));
            if app_screens::action(ui,download,(provider,"download-desktop"),"Download desktop app",panel(),foreground(),true).on_hover_text(provider.website()).clicked(){open_url(provider.website());}
            if app_screens::action(ui,launch,(provider,"open-desktop"),"Open desktop app",panel(),foreground(),true).clicked(){
                self.cloud.message = match provider.open_desktop() {
                    Ok(())=>format!("Opening {}. If no window appears, check its icon in the system tray.",provider.name()),
                    Err(error)=>error,
                };
            }
            if connected {
                let change = egui::Rect::from_min_size(egui::pos2(r.right()-136.0,controls_y),Vec2::new(116.0,32.0));
                if app_screens::action(ui,change,(provider,"change-folder"),"Change folder",panel(),foreground(),!self.cloud.busy).clicked(){self.cloud.connect(provider);}
            }
            ui.add_space(12.0);
        }
        ui.add_space(12.0);ui.label(RichText::new(tr("Master Suite confirms local copies only. The provider’s desktop app handles uploads and reports their status.")).size(12.0).color(muted()));
    }
    fn cloud_projects(&mut self,ui:&mut egui::Ui){
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(tr("Project files")).size(20.0));
            if ui.checkbox(&mut self.cloud.settings.automatic,tr("Automatic backup")).on_hover_text(tr("Back up selected files after changes are saved.")).changed(){self.cloud.save();}
        });ui.add_space(16.0);
        ui.add_sized([ui.available_width(),40.0],egui::TextEdit::singleline(&mut self.cloud.search).hint_text(tr("Search files or folders")).margin(Vec2::new(12.0,11.0)));
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui|{for (filter,label) in [(0,"All files"),(1,"Selected"),(2,"Needs attention")]{if ui.selectable_label(self.cloud.view_filter==filter,tr(label)).clicked(){self.cloud.view_filter=filter;}}});ui.add_space(16.0);
        let query=self.cloud.search.trim().to_lowercase();
        let projects:Vec<_>=self.projects.iter().filter(|p|(p.title.to_lowercase().contains(&query)||p.path.to_string_lossy().to_lowercase().contains(&query))&&match self.cloud.view_filter{1=>self.cloud.settings.selected.contains(&p.path),2=>matches!(self.cloud.status(p),"Needs attention"|"Folder unavailable"),_=>true}).cloned().collect();
        if projects.is_empty(){surface().show(ui,|ui|{ui.set_width(ui.available_width());ui.add_space(18.0);ui.label(RichText::new(tr("No files here yet")).size(18.0));ui.add_space(8.0);ui.label(RichText::new(tr("Add a project folder, or change your search and filters.")).color(muted()));ui.add_space(16.0);if secondary_button(ui,"Manage project folders").clicked(){self.projects_tab=true;self.page=Page::Projects;}ui.add_space(10.0);});return;}
        let all=projects.iter().all(|p|self.cloud.settings.selected.contains(&p.path));
        let (head,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),36.0),egui::Sense::hover());
        let mut select=ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(head.min+Vec2::new(15.0,8.0),Vec2::splat(22.0))));let mut select_all=all;
        if select.add_enabled(!self.cloud.busy,egui::Checkbox::without_text(&mut select_all)).on_hover_text(tr("Select all visible files")).changed(){for project in &projects{self.cloud.settings.selected.retain(|p|p!=&project.path);if select_all{self.cloud.settings.selected.push(project.path.clone());}}self.cloud.save();}
        app_screens::text_at(ui,egui::Rect::from_min_size(head.min+Vec2::new(54.0,8.0),Vec2::new(180.0,20.0)),"Name",11.0,muted());
        app_screens::text_at(ui,egui::Rect::from_min_size(egui::pos2(head.right()-158.0,head.top()+8.0),Vec2::new(140.0,20.0)),"Local backup status",11.0,muted());
        for project in projects{
            let selected=self.cloud.settings.selected.contains(&project.path);let status=self.cloud.status(&project);let color=status_color(status);
            let (r,response)=ui.allocate_exact_size(Vec2::new(ui.available_width(),70.0),egui::Sense::hover());
            let fill=if selected{mix_color(panel(),ACCENT,0.05)}else if response.hovered(){card()}else{panel()};ui.painter().rect_filled(r,0.0,fill);ui.painter().line_segment([r.left_bottom(),r.right_bottom()],egui::Stroke::new(1.0_f32,border()));
            let mut check=ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(r.min+Vec2::new(15.0,25.0),Vec2::splat(22.0))));let mut enabled=selected;
            if check.add_enabled(!self.cloud.busy,egui::Checkbox::without_text(&mut enabled)).on_hover_text(format!("{}: {}",tr("Back up"),project.title)).changed(){self.cloud.settings.selected.retain(|p|p!=&project.path);if enabled{self.cloud.settings.selected.push(project.path.clone());}self.cloud.save();}
            let thumb=egui::Rect::from_min_size(r.min+Vec2::new(50.0,15.0),Vec2::splat(40.0));let mut image=ui.new_child(egui::UiBuilder::new().max_rect(thumb));self.project_thumbnail_at(&mut image,&project,thumb.size());
            let title=egui::Rect::from_min_max(r.min+Vec2::new(104.0,15.0),egui::pos2(r.right()-184.0,r.top()+36.0));
            app_screens::text_at(ui,title,&project.title,14.0,foreground()).on_hover_text(project.path.display().to_string());
            app_screens::text_at(ui,title.translate(Vec2::new(0.0,24.0)),format!("{}  ·  {}",project.app.name,format_file_size(project.size_bytes)),11.0,muted());
            let dot=egui::pos2(r.right()-162.0,r.center().y);ui.painter().circle_filled(dot,3.0,color);
            let display=match status{"Not selected"=>"Local only","Not connected"=>"Choose folder","Copied"=>"Copied to folder","Pending copy"=>"Pending backup","Syncing"=>"Working…",_=>status};
            let mut badge=app_screens::text_at(ui,egui::Rect::from_min_size(egui::pos2(r.right()-150.0,r.center().y-9.0),Vec2::new(136.0,22.0)),display,12.0,color);
            for provider in cloud::PROVIDERS{if let Some(error)=self.cloud.errors.get(&(project.path.clone(),provider)){badge=badge.on_hover_text(format!("{}: {error}",provider.name()));}}
        }
    }
    fn cloud_history(&mut self,ui:&mut egui::Ui){
        ui.horizontal_wrapped(|ui|{ui.label(RichText::new(tr("Saved versions")).size(20.0));if ui.add_enabled_ui(!self.cloud.busy&&!self.cloud.settings.folders.is_empty(),|ui|secondary_button(ui,"Refresh")).inner.clicked(){self.cloud.refresh();}});
        ui.add_space(8.0);ui.label(RichText::new(tr("Restore an earlier version as a separate file. Your current project stays untouched.")).color(muted()));ui.add_space(20.0);
        if self.cloud.history.is_empty(){surface().show(ui,|ui|{ui.set_width(ui.available_width());ui.add_space(18.0);ui.label(RichText::new(tr("Your backup history will appear here")).size(18.0));ui.add_space(8.0);ui.label(RichText::new(tr("Choose a sync folder and back up a project to save its first version.")).color(muted()));ui.add_space(16.0);if secondary_button(ui,"View files").clicked(){self.cloud.tab=0;}ui.add_space(10.0);});}
        let mut history=self.cloud.history.clone();history.sort_by(|a,b|b.modified.cmp(&a.modified));
        for remote in history{
            let (r,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),76.0),egui::Sense::hover());ui.painter().rect_filled(r,0.0,panel());ui.painter().line_segment([r.left_bottom(),r.right_bottom()],egui::Stroke::new(1.0_f32,border()));
            let button=egui::Rect::from_min_size(egui::pos2(r.right()-140.0,r.center().y-16.0),Vec2::new(124.0,32.0));
            let title=egui::Rect::from_min_max(r.min+Vec2::new(16.0,15.0),egui::pos2(button.left()-18.0,r.top()+36.0));
            app_screens::text_at(ui,title,remote.original_name(),14.0,foreground()).on_hover_text(remote.original_name());
            let date=DateTime::parse_from_rfc3339(&remote.modified).map(|t|t.with_timezone(&Local).format("%d %b %Y, %H:%M").to_string()).unwrap_or_else(|_|remote.modified.clone());
            app_screens::text_at(ui,title.translate(Vec2::new(0.0,25.0)),format!("{}  ·  {}  ·  {}",remote.provider.name(),date,format_file_size(remote.bytes)),11.0,muted());
            if app_screens::action(ui,button,("restore",remote.provider,&remote.id),"Restore copy",card(),foreground(),!self.cloud.busy).clicked(){if let Some(path)=rfd::FileDialog::new().set_file_name(remote.original_name()).save_file(){self.cloud.restore(remote.clone(),path);}}
        }
    }
    fn cloud_dialogs(&mut self,ctx:&egui::Context){
        if let Some(provider)=self.cloud.disconnect_provider{
            let mut remove=false;let mut cancel=false;let response=egui::Modal::new(egui::Id::new("cloud-disconnect")).frame(surface().inner_margin(26)).show(ctx,|ui|{
                ui.set_width(420.0);ui.heading(format!("{} {}?",tr("Disconnect"),provider.name()));ui.add_space(14.0);ui.label(tr("Master Suite will stop copying backups to this folder. Existing backup files and projects are kept. Your provider remains signed in."));ui.add_space(22.0);ui.horizontal(|ui|{cancel=secondary_button(ui,"Cancel").clicked();remove=danger_button(ui,"Disconnect").clicked();});
            });if remove{self.cloud.disconnect(provider);self.cloud.disconnect_provider=None;}else if cancel||response.should_close(){self.cloud.disconnect_provider=None;}
        }
    }
}
