//! App-library layouts use bounded rows so variable descriptions and errors never
//! shift the controls on adjacent cards. The prior screens remain selectable.
use super::*;

/// Shared, wrapping notice for features that are still under development.
pub(super) fn experimental_banner(ui: &mut egui::Ui) {
    let amber = Color32::from_rgb(225, 162, 55);
    egui::Frame::new()
        .fill(mix_color(panel(), amber, 0.09))
        .stroke(egui::Stroke::new(1.0_f32, mix_color(border(), amber, 0.35)))
        .corner_radius(10)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(tr("Experimental")).size(12.0).strong().color(readable_app_color(amber)));
            ui.add_space(5.0);
            ui.add(egui::Label::new(RichText::new(tr("These features are still being developed and may not function as intended.")).size(13.0).color(foreground())).wrap());
        });
    ui.add_space(18.0);
}


pub(super) fn text_at(ui: &mut egui::Ui, rect: egui::Rect, text: impl Into<String>, size: f32, color: Color32) -> egui::Response {
    let mut child=ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
    // Keep the scroll viewport clip: replacing it lets scrolled text paint over the header.
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child.add(egui::Label::new(RichText::new(tr(text.into())).size(size).color(color)).truncate())
}
pub(super) fn action(ui:&mut egui::Ui,rect:egui::Rect,id:impl std::hash::Hash,label:&str,fill:Color32,text:Color32,enabled:bool)->egui::Response{
    let response=ui.interact(rect,ui.id().with(id),if enabled{egui::Sense::click()}else{egui::Sense::hover()});
    let hover=enabled && response.hovered();
    let painter=ui.painter_at(rect.expand(2.0));
    painter.rect_filled(rect,9.0,if hover{mix_color(fill,foreground(),0.08)}else{fill});
    painter.rect_stroke(rect,9.0,egui::Stroke::new(1.0_f32,if response.has_focus(){ACCENT}else{mix_color(fill,text,0.13)}),egui::StrokeKind::Inside);
    let translated=tr(label);
    let mut font_size=13.0;
    let mut galley=painter.layout_no_wrap(translated.clone(),egui::FontId::proportional(font_size),if enabled{text}else{muted()});
    if galley.size().x > rect.width()-16.0 {
        font_size=(font_size*(rect.width()-16.0)/galley.size().x).max(10.0);
        galley=painter.layout_no_wrap(translated.clone(),egui::FontId::proportional(font_size),if enabled{text}else{muted()});
    }
    painter.with_clip_rect(rect.shrink(6.0)).galley(rect.center()-galley.size()*0.5,galley,if enabled{text}else{muted()});
    let response=response.on_hover_text(translated);
    response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,enabled,label));
    if enabled{response.on_hover_cursor(egui::CursorIcon::PointingHand)}else{response}
}
pub(super) fn ink_for(tint:Color32)->Color32{
    // Choose the higher-contrast text color using linear sRGB luminance.
    let linear=|v:u8|{let v=v as f32/255.0;if v<=0.04045{v/12.92}else{((v+0.055)/1.055).powf(2.4)}};
    let luminance=0.2126*linear(tint.r())+0.7152*linear(tint.g())+0.0722*linear(tint.b());
    if luminance>0.179{Color32::BLACK}else{Color32::WHITE}
}

impl Launcher {
    fn library_header(&mut self,ui:&mut egui::Ui,collection:bool){
        let (rect,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),90.0),egui::Sense::hover());
        let text_right=rect.right()-150.0;
        text_at(ui,egui::Rect::from_min_max(rect.min,egui::pos2(text_right,rect.top()+15.0)),if collection{"YOUR WORKSPACE"}else{"EXPLORE THE SUITE"},10.0,muted());
        text_at(ui,egui::Rect::from_min_max(rect.min+Vec2::new(0.0,23.0),egui::pos2(text_right,rect.top()+62.0)),if collection{"Your apps"}else{"App Manager"},30.0,foreground());
        text_at(ui,egui::Rect::from_min_max(rect.min+Vec2::new(0.0,66.0),egui::pos2(rect.right(),rect.bottom())),if collection{"Your installed tools. Open an app or pick up a project in its workspace."}else{"Discover, install and keep your creative toolkit up to date."},13.0,muted());
        let button=egui::Rect::from_min_size(egui::pos2(rect.right()-136.0,rect.top()+24.0),Vec2::new(136.0,36.0));
        if action(ui,button,"switch-library",if collection{"Manage apps"}else{"Your apps"},card(),foreground(),true).clicked(){self.page=if collection{Page::Apps}else{Page::YourApps};}
        ui.add_space(16.0);
    }
    fn library_search(&mut self,ui:&mut egui::Ui,collection:bool){
        let (row,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),40.0),egui::Sense::hover());
        let reserve=if collection{0.0}else{52.0};
        let rect=egui::Rect::from_min_max(row.min,egui::pos2(row.right()-reserve,row.bottom()));
        let mut child=ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.spacing_mut().interact_size.y=40.0;
        let input=if collection{&mut self.your_apps_search}else{&mut self.manager_search};
        child.add_sized(rect.size(),egui::TextEdit::singleline(input).hint_text(tr(if collection{"Search your installed apps..."}else{"Search apps by name or purpose..."})).margin(Vec2::new(14.0,10.0)));
        if !collection{
            let rect=egui::Rect::from_min_size(egui::pos2(row.right()-40.0,row.top()),Vec2::splat(40.0));
            let mut child=ui.new_child(egui::UiBuilder::new().max_rect(rect));
            if icon_button_sized(&mut child,ButtonIcon::UpdateArrow,"Check all apps for updates",ACCENT,40.0).clicked(){self.checked=false;self.check_releases();}
        }
    }
    fn category_selector(&mut self,ui:&mut egui::Ui){
        let gap=12.0;let width=(ui.available_width()-gap)/2.0;
        let (row,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),70.0),egui::Sense::hover());
        for (i,(group,title,description)) in [(AppGroup::Creative,"Creative apps","Image, video, design & sound"),(AppGroup::Office,"Productivity apps","Documents, data & technical work")].iter().enumerate(){
            let rect=egui::Rect::from_min_size(row.min+Vec2::new(i as f32*(width+gap),0.0),Vec2::new(width,70.0));
            let active=self.app_category==*group;let tint=if *group==AppGroup::Creative{ACCENT}else{Color32::from_rgb(54,174,196)};
            let response=ui.interact(rect,ui.id().with(("app-category",i)),egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            ui.painter().rect_filled(rect,12.0,mix_color(panel(),tint,if active{0.16}else if response.hovered(){0.07}else{0.02}));
            ui.painter().rect_stroke(rect,12.0,egui::Stroke::new(1.0_f32,if active{readable_app_color(tint)}else{border()}),egui::StrokeKind::Inside);
            let count=APPS.iter().filter(|app|app.group==*group).count();
            text_at(ui,egui::Rect::from_min_size(rect.min+Vec2::new(16.0,13.0),Vec2::new(width-62.0,22.0)),*title,15.0,foreground());
            text_at(ui,egui::Rect::from_min_size(rect.min+Vec2::new(16.0,40.0),Vec2::new(width-32.0,18.0)),*description,11.0,muted());
            ui.painter().text(rect.right_top()+Vec2::new(-20.0,24.0),egui::Align2::RIGHT_CENTER,tr(count.to_string()),egui::FontId::proportional(14.0),readable_app_color(tint));
            response.widget_info(||egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel,true,active,*title));
            if response.clicked(){self.app_category=*group;}
        }
    }
    pub(super) fn apps_page(&mut self,ui:&mut egui::Ui){
        if self.prefs.classic_app_screens{self.classic_apps_page(ui);return;}
        self.library_header(ui,false);self.category_selector(ui);ui.add_space(16.0);
        self.library_search(ui,false);ui.add_space(12.0);
        let (row,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),34.0),egui::Sense::hover());
        let mut x=row.left();
        for (label,width) in [("All apps",86.0),("Installed",92.0),("Available updates",148.0)]{
            let selected=self.filter==label;
            let rect=egui::Rect::from_min_size(egui::pos2(x,row.top()),Vec2::new(width,34.0));
            if action(ui,rect,("manager-filter",label),label,if selected{mix_color(panel(),ACCENT,0.22)}else{panel()},if selected{foreground()}else{muted()},true).clicked(){self.filter=label.into();}x+=width+8.0;
        }
        let query=self.manager_search.trim().to_lowercase();
        let apps:Vec<_>=APPS.iter().copied().filter(|app|app.group==self.app_category)
            .filter(|app|format!("{} {} {}",app.name,app.category,app.blurb).to_lowercase().contains(&query))
            .filter(|app|{let state=self.states.get(app.id);match self.filter.as_str(){"Installed"=>state.is_some_and(|s|s.installed.is_some()),"Available updates"=>state.is_some_and(|s|s.installed.is_some()&&s.latest.is_some()&&s.latest!=s.installed),_=>true}}).collect();
        ui.add_space(14.0);
        ui.label(RichText::new(tr(format!("{} apps  /  {}",apps.len(),if self.app_category==AppGroup::Creative{"Creative collection"}else{"Productivity collection"}))).size(11.0).color(muted()));ui.add_space(10.0);
        if apps.is_empty(){self.library_empty(ui,false);return;}
        egui::ScrollArea::vertical().id_salt("manager-redesign").show(ui,|ui|self.library_grid(ui,&apps,false));
    }
    pub(super) fn your_apps_page(&mut self,ui:&mut egui::Ui){
        if self.prefs.classic_app_screens{self.classic_your_apps_page(ui);return;}
        self.library_header(ui,true);self.library_search(ui,true);ui.add_space(12.0);
        let (row,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),34.0),egui::Sense::hover());let mut x=row.left();
        for (id,label,width) in [(0,"All your apps",116.0),(1,"Creative",96.0),(2,"Productivity",116.0)]{
            let active=self.collection_group==id;let rect=egui::Rect::from_min_size(egui::pos2(x,row.top()),Vec2::new(width,34.0));
            if action(ui,rect,("collection-filter",id),label,if active{mix_color(panel(),ACCENT,0.22)}else{panel()},if active{foreground()}else{muted()},true).clicked(){self.collection_group=id;}x+=width+8.0;
        }
        ui.add_space(22.0);
        let query=self.your_apps_search.trim().to_lowercase();
        let apps:Vec<_>=APPS.iter().copied().filter(|a|self.states.get(a.id).is_some_and(|s|s.installed.is_some()))
            .filter(|a|self.collection_group==0||(self.collection_group==1&&a.group==AppGroup::Creative)||(self.collection_group==2&&a.group==AppGroup::Office))
            .filter(|a|format!("{} {} {}",a.name,a.category,a.blurb).to_lowercase().contains(&query)).collect();
        if apps.is_empty(){self.library_empty(ui,true);return;}
        egui::ScrollArea::vertical().id_salt("collection-redesign").show(ui,|ui|{
            for (group,label) in [(AppGroup::Creative,"Creative workspace"),(AppGroup::Office,"Productivity workspace")]{
                let group_apps:Vec<_>=apps.iter().copied().filter(|a|a.group==group).collect();if group_apps.is_empty(){continue;}
                ui.label(RichText::new(tr(format!("{label}   /   {}",group_apps.len()))).size(16.0).strong());ui.add_space(12.0);
                self.library_grid(ui,&group_apps,true);ui.add_space(14.0);
            }
        });
    }
    fn library_empty(&mut self,ui:&mut egui::Ui,collection:bool){
        settings_section(ui,if collection{"No apps to show"}else{"No matching apps"},if collection{"Try another search or install an app to start your collection."}else{"Try another search, category or installation filter."},|ui|{
            if secondary_button(ui,if collection{"Browse App Manager"}else{"Clear filters"}).clicked(){
                if collection{self.page=Page::Apps;}else{self.manager_search.clear();self.filter="All apps".into();}
            }
        });
    }
    fn library_grid(&mut self,ui:&mut egui::Ui,apps:&[AppInfo],collection:bool){
        let columns=((ui.available_width()+16.0)/340.0).floor().clamp(1.0,4.0) as usize;
        ui.spacing_mut().item_spacing.x=16.0;
        for row in apps.chunks(columns){
            ui.columns(columns,|columns|{for (i,app) in row.iter().enumerate(){columns[i].push_id(app.id,|ui|self.library_card(ui,*app,collection));}});
            ui.add_space(16.0);
        }
    }
    fn library_card(&mut self,ui:&mut egui::Ui,app:AppInfo,collection:bool){
        let state=self.states.get(app.id).cloned().unwrap_or_default();let installed=state.installed.is_some();
        let updating=installed&&state.latest.is_some()&&state.latest!=state.installed;
        let height=if collection{224.0}else{268.0};
        let (rect,response)=ui.allocate_exact_size(Vec2::new(ui.available_width(),height),egui::Sense::click());
        let hover=if self.prefs.reduce_motion{if response.hovered(){1.0}else{0.0}}else{ui.ctx().animate_bool(response.id,response.hovered())};
        let painter=ui.painter_at(rect);
        painter.rect_filled(rect,14.0,panel());
        let band=egui::Rect::from_min_size(rect.min,Vec2::new(rect.width(),90.0));
        painter.rect_filled(band,egui::CornerRadius{nw:14,ne:14,sw:0,se:0},mix_color(panel(),app.tint,if light_theme(){0.13+hover*0.05}else{0.23+hover*0.07}));
        painter.rect_stroke(rect,14.0,egui::Stroke::new(1.0_f32,mix_color(border(),app.tint,0.20+hover*0.30)),egui::StrokeKind::Inside);
        let logo=egui::Rect::from_min_size(rect.min+Vec2::new(18.0,21.0),Vec2::splat(48.0));
        if let Some(texture)=state.icon.as_ref(){painter.image(texture.id(),logo,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.0,1.0)),Color32::WHITE);}else{painter.rect_filled(logo,10.0,app.tint);painter.text(logo.center(),egui::Align2::CENTER_CENTER,tr(&app.name[..1]),egui::FontId::proportional(24.0),ink_for(app.tint));}
        text_at(ui,egui::Rect::from_min_size(rect.min+Vec2::new(80.0,22.0),Vec2::new(rect.width()-98.0,26.0)),app.name,20.0,foreground());
        text_at(ui,egui::Rect::from_min_size(rect.min+Vec2::new(80.0,53.0),Vec2::new(rect.width()-98.0,18.0)),app.category,10.0,readable_app_color(app.tint));
        let status=if state.busy.is_some(){"Working"}else if state.error.is_some(){"Needs attention"}else if updating{"Update available"}else if installed{"Installed"}else if app.has_release{"Available to install"}else{"Coming soon"};
        let status_color=if state.error.is_some(){theme_rgb(225,112,114)}else if updating{readable_app_color(app.tint)}else{muted()};
        painter.circle_filled(rect.min+Vec2::new(22.0,113.0),3.0,if installed&&state.error.is_none(){theme_rgb(90,207,142)}else{status_color});
        text_at(ui,egui::Rect::from_min_size(rect.min+Vec2::new(33.0,104.0),Vec2::new(rect.width()-51.0,20.0)),status,12.0,status_color);
        let body=if let Some(error)=&state.error{error.clone()}else if let Some(progress)=&state.busy{progress.clone()}else if collection{
            let count=self.projects.iter().filter(|p|p.app.id==app.id).count();format!("{} project{}  /  Version {}",count,if count==1{""}else{"s"},state.installed.as_deref().unwrap_or("—"))
        }else{app.blurb.into()};
        text_at(ui,egui::Rect::from_min_size(rect.min+Vec2::new(18.0,136.0),Vec2::new(rect.width()-36.0,32.0)),body.clone(),12.0,if state.error.is_some(){status_color}else{muted()}).on_hover_text(tr(body));
        if !collection{
            let version=if installed{format!("Installed {}  /  Latest {}",state.installed.as_deref().unwrap_or("—"),state.latest.as_deref().unwrap_or("Not checked"))}else{state.latest.as_ref().map(|v|format!("Latest version {v}")).unwrap_or_else(||if app.has_release{"Desktop app".into()}else{"No compatible release yet".into()})};
            text_at(ui,egui::Rect::from_min_size(rect.min+Vec2::new(18.0,184.0),Vec2::new(rect.width()-36.0,19.0)),version,11.0,muted());
        }
        let y=rect.bottom()-54.0;let left=rect.left()+18.0;
        let primary=egui::Rect::from_min_size(egui::pos2(left,y),Vec2::new(112.0,36.0));
        let enabled=state.busy.is_none()&&(installed||app.has_release);
        if action(ui,primary,"primary",if state.busy.is_some(){"Working…"}else if installed{"Open app"}else if app.has_release{"Install app"}else{"Coming soon"},if enabled{app.tint}else{card()},ink_for(app.tint),enabled).clicked()&&enabled{
            if installed{self.launch(app,None);}else{self.install(app);}
        }
        if collection{
            let workspace=egui::Rect::from_min_max(egui::pos2(left+124.0,y),egui::pos2(rect.right()-18.0,y+36.0));
            if action(ui,workspace,"workspace","Workspace",mix_color(panel(),app.tint,0.10),readable_app_color(app.tint),true).clicked(){self.detail_parent=Page::YourApps;self.page=Page::App(app.id);}
        }else{
            let mut x=rect.right()-18.0-32.0;
            for (icon,tooltip,color,kind) in [(ButtonIcon::Help,"App details",muted(),0),(ButtonIcon::Delete,"Uninstall app",theme_rgb(213,83,93),1),(ButtonIcon::UpdateArrow,if updating{"Install update"}else{"Check for updates"},readable_app_color(app.tint),2)]{
                if kind==1&&!installed{continue;}
                let target=egui::Rect::from_min_size(egui::pos2(x,y+2.0),Vec2::splat(32.0));
                let mut child=ui.new_child(egui::UiBuilder::new().max_rect(target));
                let allowed=kind==0||state.busy.is_none();
                let clicked=child.add_enabled_ui(allowed,|ui|icon_button_sized(ui,icon,tooltip,color,32.0)).inner.clicked();
                if clicked{match kind{0=>{self.detail_parent=Page::Apps;self.page=Page::App(app.id);},1=>self.show_remove=Some(app.id.into()),_=>if updating{self.install(app);}else{self.check_app_release(app);}}}x-=38.0;
            }
        }
        if response.clicked(){self.detail_parent=if collection{Page::YourApps}else{Page::Apps};self.page=Page::App(app.id);}
    }
}
