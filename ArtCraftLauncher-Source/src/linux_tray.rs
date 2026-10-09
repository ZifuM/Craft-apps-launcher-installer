//! StatusNotifierItem tray; keep the window visible when the desktop has no tray host.
use super::*;
use ksni::blocking::TrayMethods;
struct Item { tx:Sender<Event>,ctx:egui::Context }
impl Item { fn send(&self,event:Event){let _=self.tx.send(event);self.ctx.request_repaint();} }
impl ksni::Tray for Item {
 fn id(&self)->String { platform::FLATPAK_ID.into() }
 fn title(&self)->String { "ArtCraft Master Suite".into() }
 fn icon_name(&self)->String { platform::FLATPAK_ID.into() }
 fn icon_pixmap(&self)->Vec<ksni::Icon>{
  let image=image::load_from_memory(include_bytes!("../assets/artcraft-icon.png")).unwrap().resize(32,32,image::imageops::FilterType::Triangle).to_rgba8();
  let mut data=Vec::new();for p in image.pixels(){data.extend_from_slice(&[p[3],p[0],p[1],p[2]]);}vec![ksni::Icon{width:image.width() as i32,height:image.height() as i32,data}]
 }
 fn activate(&mut self,_x:i32,_y:i32){self.send(Event::TrayOpen);}
 fn watcher_offline(&self,_reason:ksni::OfflineReason)->bool{self.send(Event::TrayOpen);false}
 fn menu(&self)->Vec<ksni::MenuItem<Self>>{vec![
  ksni::menu::StandardItem{label:tr("Open Master Suite"),activate:Box::new(|this:&mut Self|this.send(Event::TrayOpen)),..Default::default()}.into(),
  ksni::menu::StandardItem{label:tr("Quit"),activate:Box::new(|this:&mut Self|this.send(Event::TrayQuit)),..Default::default()}.into()
 ]}
}
pub struct Tray(ksni::blocking::Handle<Item>);
impl Tray { pub fn new(tx:Sender<Event>,ctx:egui::Context)->Result<Self,String>{Item{tx,ctx}.disable_dbus_name(platform::flatpak()).spawn().map(Self).map_err(|e|format!("System tray is unavailable; Master Suite will remain visible. {e}"))} pub fn available(&self)->bool{!self.0.is_closed()} }
impl Drop for Tray {fn drop(&mut self){self.0.shutdown();}}
pub fn set_startup(enabled:bool)->Result<(),String>{
 let root=if platform::flatpak(){platform::home().ok_or("Cannot locate home")?.join(".config/autostart")}else{platform::config_root().ok_or("Cannot locate login settings")?.join("autostart")};let path=root.join(format!("{}.desktop",platform::FLATPAK_ID));
 if !enabled{if path.exists(){fs::remove_file(path).map_err(|e|e.to_string())?;}return Ok(());}
 let executable=if platform::flatpak(){format!("flatpak run {} --tray",platform::FLATPAK_ID)}else{
  let path=std::env::var_os("APPIMAGE").map(PathBuf::from).or_else(||std::env::current_exe().ok()).ok_or("Cannot locate Master Suite")?;
  let text=path.to_str().ok_or("Executable path is not valid text")?;if text.contains(['\n','\r']){return Err("Executable path contains a newline".into());}
  format!("\"{}\" --tray",text.replace('\\',"\\\\").replace('\"',"\\\"").replace('`',"\\`").replace('$',"\\$").replace('%',"%%"))
 };
 fs::create_dir_all(root).map_err(|e|e.to_string())?;fs::write(path,format!("[Desktop Entry]\nType=Application\nName=ArtCraft Master Suite\nExec={executable}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n")).map_err(|e|e.to_string())
}
