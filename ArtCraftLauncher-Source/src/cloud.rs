//! Versioned local backups delivered to folders managed by provider desktop apps.
//! This module never claims that a provider has finished uploading a local copy.
use super::*;
use std::{io::Write,sync::atomic::{AtomicBool,Ordering}};
#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash,Serialize,Deserialize)]
pub enum Provider{Google,Dropbox,OneDrive}
pub const PROVIDERS:[Provider;3]=[Provider::Google,Provider::Dropbox,Provider::OneDrive];
impl Provider{
 pub fn name(self)->&'static str{match self{Self::Google=>"Google Drive",Self::Dropbox=>"Dropbox",Self::OneDrive=>"OneDrive"}}
 pub fn website(self)->&'static str{if cfg!(target_os="linux")&&self==Self::Dropbox{return "https://www.dropbox.com/install-linux";}match self{Self::Google=>"https://support.google.com/drive/answer/10838124",Self::Dropbox=>"https://www.dropbox.com/install",Self::OneDrive=>"https://www.microsoft.com/microsoft-365/onedrive/download"}}
}
impl Provider {
 pub fn open_desktop(self) -> Result<(), String> {
  #[cfg(target_os="linux")] {
   let binary=match self {Self::Dropbox=>"dropbox",Self::Google=>return Err("Google Drive has no official Linux desktop client. Select a folder synced by your chosen Linux sync tool.".into()),Self::OneDrive=>return Err("OneDrive has no official Linux desktop client. Select a folder synced by your chosen Linux sync tool.".into())};
   platform::spawn(Command::new(binary).arg("start")).map_err(|e|format!("Could not open Dropbox: {e}"))?;return Ok(());
  }
  #[cfg(target_os="macos")] {
   let name=match self{Self::Google=>"Google Drive",Self::Dropbox=>"Dropbox",Self::OneDrive=>"OneDrive"};
   let status=Command::new("/usr/bin/open").args(["-a",name]).status().map_err(|e|e.to_string())?;
   return if status.success(){Ok(())}else{Err(format!("Could not open {name}. Install its macOS desktop app first."))};
  }
  #[cfg(not(any(target_os="linux",target_os="macos")))] {

  let mut candidates = Vec::new();
  for variable in ["LOCALAPPDATA", "ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"] {
   let Some(root) = std::env::var_os(variable).map(PathBuf::from) else { continue; };
   match self {
    Self::OneDrive => { candidates.push(root.join("Microsoft/OneDrive/OneDrive.exe")); }
    Self::Dropbox => { candidates.push(root.join("Dropbox/Client/Dropbox.exe")); }
    Self::Google => {
     let base = root.join("Google/Drive File Stream");
     candidates.push(base.join("GoogleDriveFS.exe"));
     if let Ok(entries) = fs::read_dir(&base) {
      let mut versions: Vec<_> = entries.flatten().filter_map(|entry| {
       let name = entry.file_name().to_string_lossy().into_owned();
       let version: Option<Vec<u32>> = name.split('.').map(|part|part.parse().ok()).collect();
       version.map(|version|(version,entry.path().join("GoogleDriveFS.exe")))
      }).collect();
      versions.sort_by(|a,b|b.0.cmp(&a.0));
      candidates.extend(versions.into_iter().map(|(_,path)|path));
     }
    }
   }
  }
  let mut failure = None;
  for path in candidates.into_iter().filter(|path|path.is_file()) {
   match Command::new(&path).spawn() {
    Ok(_) => return Ok(()),
    Err(error) => failure = Some(error.to_string()),
   }
  }
  Err(match failure {
   Some(error) => format!("Could not open {}: {error}",self.name()),
   None => format!("{} was not found in the usual installation folders. Use Download desktop app to install it, or open an existing custom installation from the Windows Start menu.",self.name()),
  })
 }
 }
}

#[derive(Clone,Serialize,Deserialize)]
pub struct Folder{pub path:PathBuf}
#[derive(Clone,Serialize,Deserialize)]
pub struct RemoteFile{pub provider:Provider,pub id:String,pub name:String,pub bytes:u64,pub modified:String,pub digest:String,pub folder:PathBuf}
impl RemoteFile{pub fn original_name(&self)->String{self.name.clone()}}
#[derive(Clone,Serialize,Deserialize)]
pub struct Receipt{pub path:PathBuf,pub revision:u64,pub remote:RemoteFile}
#[derive(Clone,Default,Serialize,Deserialize)]
#[serde(default)]
pub struct CloudSettings{pub selected:Vec<PathBuf>,pub automatic:bool,pub targets:Vec<Provider>,pub receipts:Vec<Receipt>,pub folders:HashMap<Provider,Folder>}
#[derive(Serialize,Deserialize)]
struct Manifest{schema:u32,name:String,file:String,digest:String,bytes:u64,modified:String}
enum Event{Progress(String),Copied(Receipt),History(Vec<RemoteFile>),FileError(PathBuf,Provider,String),Finished(Result<String,String>)}
pub struct Cloud{
 pub settings:CloudSettings,pub history:Vec<RemoteFile>,pub busy:bool,pub message:String,pub errors:HashMap<(PathBuf,Provider),String>,pub search:String,pub tab:u8,pub view_filter:u8,pub disconnect_provider:Option<Provider>,
 tx:Sender<Event>,rx:Receiver<Event>,cancel:Arc<AtomicBool>,last_auto:Instant,save_error:Option<String>,
}
fn settings_path()->Result<PathBuf,String>{Ok(app_data().ok_or("Settings folder unavailable")?.join("cloud/folder-backups.json"))}
fn stopped(cancel:&AtomicBool)->Result<(),String>{if cancel.load(Ordering::Relaxed){Err("Backup operation cancelled.".into())}else{Ok(())}}
fn valid_name(name:&str)->bool{!name.is_empty()&&name!="."&&name!=".."&&!name.chars().any(|c|c.is_control()||"/\\:<>\"|?*".contains(c))&&!name.ends_with(['.',' '])}
fn checked_root(folder:&Path)->Result<PathBuf,String>{if !folder.is_dir(){return Err("The sync folder is unavailable. Open the provider's desktop app or reconnect its drive.".into());}fs::canonicalize(folder).map_err(|e|format!("Cannot access sync folder: {e}"))}
fn inside(folder:&Path,path:&Path)->Result<PathBuf,String>{let root=checked_root(folder)?;let path=fs::canonicalize(path).map_err(|e|e.to_string())?;if !path.starts_with(&root){return Err("Backup path is outside the selected sync folder.".into());}Ok(path)}
fn base(folder:&Path)->Result<PathBuf,String>{checked_root(folder)?;let dir=folder.join("ArtCraft Master Suite");fs::create_dir_all(&dir).map_err(|e|e.to_string())?;inside(folder,&dir)?;fs::write(dir.join(".artcraft-backups"),b"ArtCraft versioned backups v1").map_err(|e|e.to_string())?;Ok(dir)}
fn temp_name()->String{format!(".partial-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos())}
struct Staged(PathBuf);
impl Drop for Staged{fn drop(&mut self){let _=fs::remove_file(self.0.join("backup.json"));let _=fs::remove_file(self.0.join("project.data"));let _=fs::remove_dir(&self.0);}}
impl Cloud{
 pub fn new()->Self{
  let(tx,rx)=mpsc::channel();let mut message=String::new();let settings=match settings_path().ok().filter(|p|p.exists()){
   Some(path)=>match fs::read(path).ok().and_then(|b|serde_json::from_slice(&b).ok()){Some(v)=>v,None=>{message="Saved backup settings could not be read. Re-select your sync folders.".into();CloudSettings::default()}},None=>CloudSettings::default()};
  Self{settings,history:Vec::new(),busy:false,message,errors:HashMap::new(),search:String::new(),tab:0,view_filter:0,disconnect_provider:None,tx,rx,cancel:Arc::new(AtomicBool::new(false)),last_auto:Instant::now(),save_error:None}
 }
 fn persist(&self)->Result<(),String>{let path=settings_path()?;fs::create_dir_all(path.parent().unwrap()).map_err(|e|e.to_string())?;let temp=path.with_extension("new");fs::write(&temp,serde_json::to_vec_pretty(&self.settings).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;fs::rename(temp,path).map_err(|e|e.to_string())}
 pub fn save(&mut self){if let Err(e)=self.persist(){self.message=format!("Could not save backup settings: {e}");self.save_error=Some(self.message.clone());self.settings.automatic=false;}else{self.save_error=None;}}
 pub fn connect(&mut self,p:Provider){
  if self.busy{return;}let mut picker=rfd::FileDialog::new().set_title(format!("Choose your {} synced folder",p.name()));if let Some(path)=self.settings.folders.get(&p).map(|f|f.path.clone()).or_else(||detect(p)){picker=picker.set_directory(path);}
  if let Some(path)=picker.pick_folder(){let result=(||{let resolved=checked_root(&path)?;
    if self.settings.folders.iter().any(|(other,f)|*other!=p&&fs::canonicalize(&f.path).ok().is_some_and(|r|resolved.starts_with(&r)||r.starts_with(&resolved))){return Err("Choose a separate folder for each provider; overlapping destinations would duplicate the same backup.".into());}
    base(&path)?;let previous=self.settings.clone();self.settings.folders.insert(p,Folder{path});if !self.settings.targets.contains(&p){self.settings.targets.push(p);}self.settings.receipts.retain(|r|r.remote.provider!=p);
    if let Err(e)=self.persist(){self.settings=previous;return Err(e);}Ok(())})();match result{Ok(())=>{self.history.retain(|r|r.provider!=p);self.errors.retain(|(_,provider),_|*provider!=p);self.message=format!("{} folder selected. Make sure its desktop app is signed in and syncing. No files have been backed up yet.",p.name());},Err(e)=>self.message=e}}
 }
 pub fn disconnect(&mut self,p:Provider){if self.busy{return;}let old=self.settings.clone();self.settings.folders.remove(&p);self.settings.targets.retain(|v|*v!=p);self.settings.receipts.retain(|r|r.remote.provider!=p);if let Err(e)=self.persist(){self.settings=old;self.message=e;return;}self.history.retain(|r|r.provider!=p);self.errors.retain(|(_,provider),_|*provider!=p);self.message="Folder disconnected. Existing backup files have been kept.".into();}
 pub fn open_folder(&self,p:Provider){if let Some(folder)=self.settings.folders.get(&p){reveal_project_path(&folder.path.join("ArtCraft Master Suite"),false);}}
 pub fn cancel(&self){self.cancel.store(true,Ordering::Relaxed);}
 fn begin(&mut self,message:&str)->Option<(Sender<Event>,Arc<AtomicBool>)>{if self.busy{return None;}self.busy=true;self.message=message.into();self.cancel=Arc::new(AtomicBool::new(false));Some((self.tx.clone(),self.cancel.clone()))}
 pub fn status(&self,p:&Project)->&'static str{
  if !self.settings.selected.contains(&p.path){return "Not selected";}if self.settings.targets.is_empty(){return "Not connected";}
  for provider in &self.settings.targets{let Some(folder)=self.settings.folders.get(provider)else{return "Not connected";};if !folder.path.is_dir(){return "Folder unavailable";}if self.errors.contains_key(&(p.path.clone(),*provider)){return "Needs attention";}}
  if self.busy{return "Syncing";}
  if self.settings.targets.iter().all(|provider|self.settings.receipts.iter().any(|r|r.path==p.path&&r.revision==p.revision&&r.remote.provider==*provider&&self.settings.folders.get(provider).is_some_and(|f|f.path==r.remote.folder)&&fs::metadata(&r.remote.id).is_ok_and(|m|m.len()==r.remote.bytes))){"Copied"}else{"Pending copy"}
 }
 pub fn tick(&mut self,projects:&[Project]){
  while let Ok(event)=self.rx.try_recv(){match event{
   Event::Progress(s)=>self.message=s,Event::Copied(r)=>{self.errors.remove(&(r.path.clone(),r.remote.provider));self.settings.receipts.retain(|old|old.path!=r.path||old.remote.provider!=r.remote.provider);self.settings.receipts.push(r);self.save();},
   Event::History(files)=>self.history=files,Event::FileError(path,p,e)=>{self.errors.insert((path,p),e);},
   Event::Finished(result)=>{self.busy=false;self.last_auto=Instant::now();self.message=match result{Ok(message)=>if let Some(error)=&self.save_error{error.clone()}else if self.errors.is_empty(){message}else{"Some files could not be copied. Check their status and retry.".into()},Err(e)=>e};}
  }}
  let delay=if self.errors.is_empty(){30}else{120};
  if self.settings.automatic&&!self.busy&&self.last_auto.elapsed()>=Duration::from_secs(delay){self.last_auto=Instant::now();if projects.iter().any(|p|self.settings.selected.contains(&p.path)&&matches!(self.status(p),"Pending copy"|"Needs attention")){self.sync(projects);}}
 }
 pub fn sync(&mut self,projects:&[Project]){
  let projects:Vec<_>=projects.iter().filter(|p|self.settings.selected.contains(&p.path)).map(|p|(p.path.clone(),p.app.name.to_owned())).collect();
  let folders:Vec<_>=self.settings.targets.iter().filter_map(|p|self.settings.folders.get(p).map(|f|(*p,f.clone()))).collect();if projects.is_empty()||folders.is_empty(){self.message="Select projects and choose a sync folder first.".into();return;}
  let Some((tx,cancel))=self.begin("Copying project backups…")else{return;};self.errors.clear();
  thread::spawn(move||{let result=(||{for (provider,folder) in &folders{for (path,app) in &projects{stopped(&cancel)?;let _=tx.send(Event::Progress(format!("{} · {}",provider.name(),path.file_name().unwrap_or_default().to_string_lossy())));match backup(*provider,&folder.path,path,app,&cancel){Ok(r)=>{let _=tx.send(Event::Copied(r));},Err(e)=>{let _=tx.send(Event::FileError(path.clone(),*provider,e));}}}}let mut history=Vec::new();for(p,f)in &folders{history.extend(list(*p,&f.path,&cancel)?);}let _=tx.send(Event::History(history));Ok("Copies are in your sync folders. Check the provider desktop app for upload status.".into())})();let _=tx.send(Event::Finished(result));});
 }
 pub fn refresh(&mut self){let folders=self.settings.folders.clone();let Some((tx,cancel))=self.begin("Reading saved versions…")else{return;};thread::spawn(move||{let result=(||{let mut files=Vec::new();for(p,f)in folders{files.extend(list(p,&f.path,&cancel)?);}let _=tx.send(Event::History(files));Ok("Saved versions refreshed from your sync folders.".into())})();let _=tx.send(Event::Finished(result));});}
 pub fn restore(&mut self,remote:RemoteFile,destination:PathBuf){
  if !self.settings.folders.get(&remote.provider).is_some_and(|f|f.path==remote.folder){self.message="Choose the sync folder that owns this backup first.".into();return;}
  let Some((tx,cancel))=self.begin("Restoring a separate copy…")else{return;};thread::spawn(move||{let result=(||{let source=inside(&remote.folder,Path::new(&remote.id))?;let mut input=fs::File::open(source).map_err(|e|e.to_string())?;let mut output=fs::OpenOptions::new().write(true).create_new(true).open(&destination).map_err(|e|format!("Choose a new file name. Existing files are never overwritten: {e}"))?;
    let copied=copy_hash(&mut input,&mut output,&cancel).and_then(|(size,digest)|{if size!=remote.bytes||digest!=remote.digest{Err("Backup integrity check failed. The copy was not restored.".into())}else{output.sync_all().map_err(|e|e.to_string())}});drop(output);if copied.is_err(){let _=fs::remove_file(&destination);}copied.map(|()|"Restored a separate copy. Your original project is unchanged.".into())})();let _=tx.send(Event::Finished(result));});
 }
}
fn detect(provider:Provider)->Option<PathBuf>{
 if provider==Provider::OneDrive{for name in ["OneDrive","OneDriveConsumer","OneDriveCommercial"]{if let Some(p)=std::env::var_os(name).map(PathBuf::from).filter(|p|p.is_dir()){return Some(p);}}}
 if provider==Provider::Dropbox{for env in ["LOCALAPPDATA","APPDATA"]{if let Some(base)=std::env::var_os(env){let p=PathBuf::from(base).join("Dropbox/info.json");if let Ok(data)=fs::read(p){if let Ok(value)=serde_json::from_slice::<serde_json::Value>(&data){for account in ["personal","business"]{if let Some(path)=value[account]["path"].as_str().map(PathBuf::from).filter(|p|p.is_dir()){return Some(path);}}}}}}}
 let home=platform::home()?;let names:&[&str]=match provider{Provider::Google=>&["Google Drive","My Drive"],Provider::Dropbox=>&["Dropbox"],Provider::OneDrive=>&["OneDrive"]};names.iter().map(|n|home.join(n)).find(|p|p.is_dir())
}
fn copy_hash(input:&mut fs::File,output:&mut fs::File,cancel:&AtomicBool)->Result<(u64,String),String>{let mut hash=Sha256::new();let mut size=0;let mut buffer=vec![0u8;1024*1024];loop{stopped(cancel)?;let n=input.read(&mut buffer).map_err(|e|e.to_string())?;if n==0{break;}output.write_all(&buffer[..n]).map_err(|e|e.to_string())?;hash.update(&buffer[..n]);size+=n as u64;}Ok((size,format!("{:x}",hash.finalize())))}
fn verify(path:&Path,size:u64,digest:&str,cancel:&AtomicBool)->Result<(),String>{let mut input=fs::File::open(path).map_err(|e|e.to_string())?;if input.metadata().map_err(|e|e.to_string())?.len()!=size{return Err("An existing backup is incomplete. It was left untouched.".into());}let mut hash=Sha256::new();let mut buffer=vec![0u8;1024*1024];loop{stopped(cancel)?;let n=input.read(&mut buffer).map_err(|e|e.to_string())?;if n==0{break;}hash.update(&buffer[..n]);}if format!("{:x}",hash.finalize())!=digest{return Err("An existing backup has changed. It was left untouched.".into());}Ok(())}
fn backup(provider:Provider,folder:&Path,source:&Path,app:&str,cancel:&AtomicBool)->Result<Receipt,String>{
 let root=base(folder)?;let source_abs=fs::canonicalize(source).map_err(|e|e.to_string())?;if source_abs.starts_with(fs::canonicalize(&root).map_err(|e|e.to_string())?){return Err("Backup files cannot be backed up into themselves.".into());}
 let revision=project_revision(source);let id=format!("{:x}",Sha256::digest(source_abs.to_string_lossy().as_bytes()));let parent=root.join("Projects").join(app).join(&id[..24]);fs::create_dir_all(&parent).map_err(|e|e.to_string())?;inside(folder,&parent)?;
 let stage=Staged(parent.join(temp_name()));fs::create_dir(&stage.0).map_err(|e|e.to_string())?;
 let mut input=fs::File::open(source).map_err(|e|e.to_string())?;let expected=input.metadata().map_err(|e|e.to_string())?.len();let mut output=fs::OpenOptions::new().write(true).create_new(true).open(stage.0.join("project.data")).map_err(|e|e.to_string())?;
 let(size,digest)=copy_hash(&mut input,&mut output,cancel)?;output.sync_all().map_err(|e|e.to_string())?;drop(output);drop(input);
 if size!=expected||project_revision(source)!=revision{return Err("The project changed while copying. Save it and retry.".into());}
 let name=source.file_name().unwrap_or_default().to_string_lossy().to_string();if !valid_name(&name){return Err("The file name cannot be used in a backup.".into());}
 let target=parent.join(&digest);let filename=format!("file-{name}");let file=target.join(&filename);
 let modified=if target.exists(){let checked=inside(folder,&file)?;verify(&checked,size,&digest,cancel)?;let manifest=read_manifest(&target.join("backup.json"))?;manifest.modified}else{
  let modified=Local::now().to_rfc3339();let manifest=Manifest{schema:1,name:name.clone(),file:filename.clone(),digest:digest.clone(),bytes:size,modified:modified.clone()};
  fs::write(stage.0.join("backup.json"),serde_json::to_vec_pretty(&manifest).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;stopped(cancel)?;
  fs::rename(stage.0.join("project.data"),stage.0.join(&filename)).map_err(|e|e.to_string())?;
  if let Err(e)=fs::rename(&stage.0,&target){let _=fs::rename(stage.0.join(&filename),stage.0.join("project.data"));return Err(e.to_string());}modified
 };
 Ok(Receipt{path:source.into(),revision,remote:RemoteFile{provider,id:file.display().to_string(),name,bytes:size,modified,digest,folder:folder.into()}})
}
fn read_manifest(path:&Path)->Result<Manifest,String>{let input=fs::File::open(path).map_err(|e|e.to_string())?;if input.metadata().map_err(|e|e.to_string())?.len()>65536{return Err("Backup metadata is too large.".into());}let mut bytes=Vec::new();input.take(65537).read_to_end(&mut bytes).map_err(|e|e.to_string())?;let m:Manifest=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;if m.schema!=1||!valid_name(&m.file)||m.digest.len()!=64||!m.digest.bytes().all(|b|b.is_ascii_hexdigit()){return Err("Invalid backup metadata.".into());}Ok(m)}
fn list(provider:Provider,folder:&Path,cancel:&AtomicBool)->Result<Vec<RemoteFile>,String>{checked_root(folder)?;let root=folder.join("ArtCraft Master Suite/Projects");if !root.exists(){return Ok(Vec::new());}inside(folder,&root)?;let mut files=Vec::new();for entry in WalkDir::new(&root).max_depth(5).follow_links(false).into_iter().filter_entry(|e|!e.file_name().to_string_lossy().starts_with('.')){stopped(cancel)?;let entry=entry.map_err(|e|e.to_string())?;if !entry.file_type().is_file()||entry.file_name()!="backup.json"{continue;}let m=read_manifest(entry.path())?;let path=entry.path().parent().ok_or("Invalid backup path")?.join(&m.file);let path=inside(folder,&path)?;if !path.is_file(){continue;}files.push(RemoteFile{provider,id:path.display().to_string(),name:m.name,bytes:m.bytes,modified:m.modified,digest:m.digest,folder:folder.into()});if files.len()>100000{return Err("Backup history exceeds 100,000 versions.".into());}}Ok(files)}
