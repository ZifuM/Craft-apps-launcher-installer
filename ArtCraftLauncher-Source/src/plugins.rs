use super::*;
use serde_json::{Value,json};
use std::io::Write;
const MAX:u64=32*1024*1024;
#[derive(Clone,Serialize,Deserialize)]
pub struct Entry{pub name:String,pub source:String,pub hash:String,pub stored:PathBuf,pub deployed:PathBuf,pub enabled:bool}
#[derive(Default)]
pub struct Manager{pub link:String,pub message:String,pub busy:bool,pub remove:Option<(String,usize)>,rx:Option<Receiver<Result<String,String>>>}
impl Manager{
    pub fn tick(&mut self){if let Some(rx)=&self.rx{if let Ok(result)=rx.try_recv(){self.message=match result{Ok(s)=>s,Err(e)=>format!("Could not install plugin: {e}")};self.busy=false;self.rx=None;}}}
    pub fn install(&mut self,app:AppInfo,root:PathBuf,source:String,local:bool){
        if self.busy{return;}let(tx,rx)=mpsc::channel();self.rx=Some(rx);self.busy=true;self.message="Preparing plugin…".into();
        thread::spawn(move||{let result=(||{if running(&app)?{return Err(format!("Close {} before installing plugins.",app.name));}let bytes=if local{read_limit(Path::new(&source),MAX*2)?}else{download(&source)?};install_bytes(&app,&root,&source,bytes)})();let _=tx.send(result);});
    }
}
pub fn supported(id:&str)->bool{matches!(id,"photocraft"|"vectorcraft"|"effectcraft")}
fn registry(app:&AppInfo)->Result<PathBuf,String>{Ok(app_data().ok_or("Settings directory unavailable")?.join("plugins").join(format!("{}.json",app.id)))}
pub fn entries(app:&AppInfo)->Result<Vec<Entry>,String>{let path=registry(app)?;if !path.exists(){return Ok(Vec::new());}serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|e|format!("Plugin registry could not be read: {e}"))}
fn save(app:&AppInfo,entries:&[Entry])->Result<(),String>{let path=registry(app)?;fs::create_dir_all(path.parent().unwrap()).map_err(|e|e.to_string())?;atomic(&path,&serde_json::to_vec_pretty(entries).map_err(|e|e.to_string())?)}
fn atomic(path:&Path,bytes:&[u8])->Result<(),String>{let tmp=path.with_extension("suite-new");let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&tmp).map_err(|e|e.to_string())?;file.write_all(bytes).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;drop(file);let result=fs::rename(&tmp,path).map_err(|e|e.to_string());if result.is_err(){let _=fs::remove_file(tmp);}result}
fn read_limit(path:&Path,max:u64)->Result<Vec<u8>,String>{let f=fs::File::open(path).map_err(|e|e.to_string())?;if f.metadata().map_err(|e|e.to_string())?.len()>max{return Err("Plugin package exceeds the size limit.".into());}let mut bytes=Vec::new();f.take(max+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;if bytes.len() as u64>max{return Err("Plugin package exceeds the size limit.".into());}Ok(bytes)}
fn hash(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
fn validate(bytes:&[u8],app:&AppInfo)->Result<(),String>{
    if bytes.len() as u64>MAX{return Err("A plugin exceeds 32 MB.".into());}
    wasmparser::Validator::new().validate_all(bytes).map_err(|e|format!("Invalid WebAssembly module: {e}"))?;
    let mut names=std::collections::HashSet::new();
    for payload in wasmparser::Parser::new(0).parse_all(bytes){match payload.map_err(|e|e.to_string())?{
        wasmparser::Payload::ImportSection(section) if section.count()>0=>return Err("This app only supports plugins without host imports.".into()),
        wasmparser::Payload::ExportSection(section)=>for export in section{let e=export.map_err(|e|e.to_string())?;names.insert(e.name.to_owned());},_=>{}
    }}
    let required:&[&str]=match app.id{"photocraft"=>&["memory","pc_abi_version","pc_manifest","pc_alloc","pc_filter"],"vectorcraft"=>&["memory","vc_abi_version","vc_manifest","vc_alloc","vc_run"],"effectcraft"=>&["memory","ec_api_version","ec_manifest_ptr","ec_manifest_len","ec_alloc","ec_render"],_=>return Err("This app does not have a supported plugin adapter yet.".into())};
    if !required.iter().all(|name|names.contains(*name)){return Err(format!("This module does not match {}'s plugin interface.",app.name));}Ok(())
}
fn request(url:&str)->Result<Vec<u8>,String>{
    let client=Client::builder().user_agent("ArtCraftMasterSuite").timeout(Duration::from_secs(90)).redirect(reqwest::redirect::Policy::custom(|attempt|{
        let host=attempt.url().host_str().unwrap_or("");if attempt.previous().len()<5&&attempt.url().scheme()=="https"&&(host=="github.com"||host=="api.github.com"||host=="raw.githubusercontent.com"||host.ends_with(".githubusercontent.com")){attempt.follow()}else{attempt.error("Unsupported download redirect")}
    })).build().map_err(|e|e.to_string())?;
    let response=client.get(url).send().and_then(|r|r.error_for_status()).map_err(|e|e.to_string())?;if response.content_length().is_some_and(|n|n>MAX*2){return Err("Download exceeds 64 MB.".into());}
    let mut bytes=Vec::new();response.take(MAX*2+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;if bytes.len() as u64>MAX*2{return Err("Download exceeds 64 MB.".into());}Ok(bytes)
}
fn download(input:&str)->Result<Vec<u8>,String>{
    let url=reqwest::Url::parse(input.trim()).map_err(|_|"Enter a GitHub repository or release asset link.")?;
    if url.scheme()!="https"||!url.username().is_empty()||url.password().is_some()||url.port().is_some(){return Err("Use a public HTTPS GitHub link.".into());}
    let host=url.host_str().unwrap_or("");if !["github.com","raw.githubusercontent.com"].contains(&host){return Err("Use a github.com or raw.githubusercontent.com link.".into());}
    let parts:Vec<_>=url.path().trim_matches('/').split('/').collect();
    if host=="github.com"&&parts.len()==2{
        let release:Value=serde_json::from_slice(&request(&format!("https://api.github.com/repos/{}/{}/releases/latest",parts[0],parts[1]))?).map_err(|e|e.to_string())?;
        let assets:Vec<_>=release["assets"].as_array().ok_or("No release assets found.")?.iter().filter(|a|a["name"].as_str().is_some_and(|n|n.ends_with(".wasm")||n.ends_with(".zip"))).collect();
        if assets.len()!=1{return Err("This repository has multiple or no plugin assets. Paste the exact .wasm or .zip release download link.".into());}
        return download(assets[0]["browser_download_url"].as_str().ok_or("Release download URL missing.")?);
    }
    if host=="github.com"&&parts.len()>=5&&parts[2]=="blob"{return request(&format!("https://raw.githubusercontent.com/{}/{}/{}",parts[0],parts[1],parts[3..].join("/")));}
    if host=="raw.githubusercontent.com"||(host=="github.com"&&parts.len()>=6&&parts[2]=="releases"&&parts[3]=="download"){return request(url.as_str());}
    Err("Use a repository, file link, or a release download link for a .wasm or .zip plugin.".into())
}
fn config_path(app:&AppInfo)->Result<PathBuf,String>{
    if app.id=="photocraft"{
        if let Some(path)=std::env::var_os("PHOTOCRAFT_CONFIG_DIR"){return Ok(PathBuf::from(path).join("preferences.json"));}
        if let Some(exe)=installed_dir(app.id).and_then(|d|find_executable(&d,app.id)){if let Some(dir)=exe.parent(){if ["portable.txt","PhotoCraft.portable"].iter().any(|n|dir.join(n).exists()){return Ok(dir.join("PhotoCraftData/preferences.json"));}}}
    }
    let root=platform::config_root().ok_or("Application configuration folder unavailable")?;
    #[cfg(target_os="linux")]
    { return Ok(match app.id {"photocraft"=>root.join("photocraft/preferences.json"),"vectorcraft"=>root.join("vectorcraft/ui.json"),"effectcraft"=>std::env::var_os("EFFECTCRAFT_CONFIG_DIR").map(PathBuf::from).unwrap_or(root.join("effectcraft")).join("prefs.json"),_=>return Err("No plugin adapter is available".into())}); }
    #[cfg(not(target_os="linux"))]
    Ok(match app.id{"photocraft"=>root.join("Photocraft/preferences.json"),"vectorcraft"=>root.join("VectorCraft/ui.json"),"effectcraft"=>std::env::var_os("EFFECTCRAFT_CONFIG_DIR").map(PathBuf::from).unwrap_or(root.join("EffectCraft")).join("prefs.json"),_=>return Err("No plugin adapter is available for this app.".into())})
}
fn json_file(path:&Path)->Result<Value,String>{if path.exists(){let v:Value=serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|e|format!("App settings are unreadable: {e}"))?;if !v.is_object(){return Err("App settings have an unexpected format.".into());}Ok(v)}else{Ok(json!({}))}}
fn save_config(path:&Path,value:&Value)->Result<(),String>{fs::create_dir_all(path.parent().ok_or("Invalid settings path")?).map_err(|e|e.to_string())?;
    let backup=path.with_extension("pre-master-suite.json");if path.exists()&&!backup.exists(){fs::copy(path,&backup).map_err(|e|e.to_string())?;}
    atomic(path,&serde_json::to_vec_pretty(value).map_err(|e|e.to_string())?)
}
fn destination(app:&AppInfo,base:&Path)->Result<PathBuf,String>{
    let path=config_path(app)?;let mut value=json_file(&path)?;
    let key=if app.id=="photocraft"{"/plugIns/additionalPluginsFolder"}else{"/engine_prefs/pluginsFolder"};
    if app.id=="effectcraft"{return Ok(path.parent().unwrap().join("Plug-ins"));}
    let dest=value.pointer(key).and_then(Value::as_str).filter(|s|!s.is_empty()).map(PathBuf::from).unwrap_or_else(||base.join("Plugins"));
    if !dest.is_absolute(){return Err("The app's existing plugin folder is relative. Set an absolute folder in its preferences first.".into());}
    let section=if app.id=="photocraft"{"plugIns"}else{"engine_prefs"};
    if !value[section].is_null()&&!value[section].is_object(){return Err("App plugin preferences have an unexpected format.".into());}
    if app.id=="photocraft"{value["plugIns"]["additionalPluginsFolder"]=json!(dest);value["plugIns"]["useAdditionalPluginsFolder"]=json!(true);}else{value["engine_prefs"]["pluginsFolder"]=json!(dest);}
    save_config(&path,&value)?;Ok(dest)
}
pub fn prepare_launch(app:&AppInfo,root:&Path)->Result<(),String>{
    let base=workspace_bridge::prepare(root,app)?;
    if running(app)?{return Ok(());}
    if app.id=="effectcraft"{
        let path=config_path(app)?;let mut value=json_file(&path)?;if !value["export"].is_null()&&!value["export"].is_object(){return Err("Export settings have an unexpected format.".into());}value["export"]["defaultOutputFolder"]=json!(base.join("Exports"));save_config(&path,&value)?;
    }
    if supported(app.id)&&!entries(app)?.is_empty(){let _=destination(app,&base)?;}
    Ok(())
}
fn install_bytes(app:&AppInfo,root:&Path,source:&str,bytes:Vec<u8>)->Result<String,String>{
    if !supported(app.id){return Err("No compatible plugin loader has been confirmed for this app.".into());}
    if running(app)?{return Err(format!("Close {} before installing plugins.",app.name));}
    let mut modules=Vec::new();
    if bytes.starts_with(b"\0asm"){modules.push((source.rsplit('/').next().unwrap_or("plugin.wasm").rsplit('\\').next().unwrap_or("plugin.wasm").to_owned(),bytes));}
    else{
        let mut archive=zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_|"Choose a .wasm module or a ZIP containing built .wasm modules, not source code.")?;
        if archive.len()>2048{return Err("The archive contains too many entries.".into());}let mut total=0u64;
        for index in 0..archive.len(){let mut file=archive.by_index(index).map_err(|e|e.to_string())?;let path=file.enclosed_name().ok_or("The archive contains an unsafe path.")?.to_owned();
            if file.unix_mode().is_some_and(|m|m&0o170000==0o120000){return Err("Plugin archives cannot contain symbolic links.".into());}
            if file.is_dir()||path.extension().and_then(|s|s.to_str())!=Some("wasm"){continue;}
            total+=file.size();if file.size()>MAX||total>MAX*2||modules.len()>=32{return Err("Plugin archive exceeds the module limit.".into());}
            let mut data=Vec::new();(&mut file).take(MAX+1).read_to_end(&mut data).map_err(|e|e.to_string())?;validate(&data,app)?;modules.push((path.file_name().unwrap().to_string_lossy().into_owned(),data));
        }
    }
    if modules.is_empty(){return Err("No compiled .wasm plugins were found.".into());}
    for (_,data) in &modules{validate(data,app)?;}
    let mut list=entries(app)?;let base=workspace_bridge::prepare(root,app)?;let dest=destination(app,&base)?;fs::create_dir_all(&dest).map_err(|e|e.to_string())?;
    let mut added=0;
    for (name,data) in modules{
        let digest=hash(&data);if list.iter().any(|entry|entry.hash==digest){continue;}
        let filename=format!("suite-{}.wasm",digest);let stored=base.join("Plugins").join(&filename);let deployed=dest.join(&filename);
        for path in [&stored,&deployed]{if path.exists(){if hash(&read_limit(path,MAX)?)!=digest{return Err("A destination file already exists with different contents.".into());}}else{atomic(path,&data)?;}}
        list.push(Entry{name,source:source.into(),hash:digest,stored,deployed,enabled:true});save(app,&list)?;added+=1;
    }
    Ok(format!("Installed {added} plugin(s) for {}. Restart the app to load them. The app performs final compatibility validation.",app.name))
}
pub fn toggle(app:&AppInfo,index:usize)->Result<(),String>{
    if running(app)?{return Err(format!("Close {} before changing plugins.",app.name));}let mut list=entries(app)?;let entry=list.get_mut(index).ok_or("Plugin no longer exists")?;
    let disabled=entry.deployed.with_extension("wasm.disabled");
    let (from,to)=if entry.enabled{(entry.deployed.clone(),disabled)}else{(disabled,entry.deployed.clone())};
    if hash(&read_limit(&from,MAX)?)!=entry.hash{return Err("Plugin was changed outside Master Suite; leaving it untouched.".into());}
    if to.exists(){return Err("The destination already exists; leaving both files untouched.".into());}
    fs::rename(&from,&to).map_err(|e|e.to_string())?;entry.enabled=!entry.enabled;
    if let Err(e)=save(app,&list){let _=fs::rename(to,from);return Err(e);}Ok(())
}
pub fn running(app:&AppInfo)->Result<bool,String>{
    #[cfg(windows)] unsafe{
        use windows_sys::Win32::{System::Diagnostics::ToolHelp::*,Foundation::{CloseHandle,INVALID_HANDLE_VALUE}};
        let snapshot=CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS,0);if snapshot==INVALID_HANDLE_VALUE{return Err("Could not check whether the app is running.".into());}
        let mut entry:PROCESSENTRY32W=std::mem::zeroed();entry.dwSize=std::mem::size_of::<PROCESSENTRY32W>() as u32;let mut next=Process32FirstW(snapshot,&mut entry);let mut found=false;
        while next!=0{let length=entry.szExeFile.iter().position(|c|*c==0).unwrap_or(entry.szExeFile.len());let name=String::from_utf16_lossy(&entry.szExeFile[..length]).to_lowercase();if name==format!("{}.exe",app.id)||name==format!("{}.exe",app.name.to_lowercase()){found=true;break;}next=Process32NextW(snapshot,&mut entry);}CloseHandle(snapshot);Ok(found)
    }
    #[cfg(not(windows))]{
        let mut command=if platform::flatpak(){let mut c=Command::new("flatpak-spawn");c.args(["--host","pgrep"]);c}else{Command::new("pgrep")};
        let output=command.args(["-ix",release_slug(app.id)]).output().map_err(|e|format!("Could not check running apps: {e}"))?;
        match output.status.code(){Some(0)=>Ok(true),Some(1)=>Ok(false),_=>Err("Could not check running apps. Install procps/pgrep before managing plugins.".into())}
    }
}

pub fn uninstall(app:&AppInfo,index:usize)->Result<(),String>{
    if running(app)?{return Err(format!("Close {} before removing plugins.",app.name));}
    let mut list=entries(app)?;let entry=list.get(index).ok_or("Plugin no longer exists")?.clone();
    let deployed=if entry.enabled{entry.deployed.clone()}else{entry.deployed.with_extension("wasm.disabled")};
    let mut paths=vec![deployed];if entry.stored!=entry.deployed&&entry.stored.exists(){paths.push(entry.stored.clone());}
    for path in &paths{
        if !path.file_name().unwrap_or_default().to_string_lossy().starts_with("suite-")||hash(&read_limit(path,MAX)?)!=entry.hash{return Err("A plugin file was modified outside Master Suite; leaving it untouched.".into());}
        if path.with_extension("suite-removed").exists(){return Err("An earlier removal backup exists; leaving it untouched.".into());}
    }
    let mut staged:Vec<(PathBuf,PathBuf)>=Vec::new();
    for path in paths{
        let backup=path.with_extension("suite-removed");
        if let Err(error)=fs::rename(&path,&backup){for (original,backup) in staged.iter().rev(){let _=fs::rename(backup,original);}return Err(error.to_string());}
        staged.push((path,backup));
    }
    list.remove(index);if let Err(error)=save(app,&list){for (original,backup) in staged.iter().rev(){let _=fs::rename(backup,original);}return Err(error);}
    for (_,backup) in staged{let _=fs::remove_file(backup);}Ok(())
}

pub fn relocate_workspace(source: &Path, destination: &Path) -> Result<(), String> {
    let rebase = |path: &mut PathBuf| {
        if let Ok(relative) = path.strip_prefix(source) { *path = destination.join(relative); }
    };
    for app in APPS.iter().filter(|app| supported(app.id)) {
        let mut list = entries(app)?;
        let before = serde_json::to_vec(&list).map_err(|e| e.to_string())?;
        for entry in &mut list { rebase(&mut entry.stored); rebase(&mut entry.deployed); }
        if before != serde_json::to_vec(&list).map_err(|e| e.to_string())? { save(app, &list)?; }
        let config = config_path(app)?;
        if !config.exists() { continue; }
        let mut value = json_file(&config)?;
        let key = if app.id == "photocraft" { "/plugIns/additionalPluginsFolder" } else { "/engine_prefs/pluginsFolder" };
        if let Some(field) = value.pointer_mut(key) {
            if let Some(path) = field.as_str().map(PathBuf::from) {
                if let Ok(relative) = path.strip_prefix(source) { *field = json!(destination.join(relative)); save_config(&config, &value)?; }
            }
        }
    }
    Ok(())
}
