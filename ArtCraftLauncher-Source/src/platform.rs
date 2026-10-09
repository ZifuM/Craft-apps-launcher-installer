//! OS boundaries shared by the installer, project actions and workspace integrations.
use super::*;
pub const FLATPAK_ID: &str = "io.github.ZifuM.ArtCraftMasterSuite";
pub fn flatpak() -> bool { cfg!(target_os="linux") && std::env::var_os("FLATPAK_ID").is_some() }
pub fn label() -> String { format!("{} / {}",std::env::consts::OS,std::env::consts::ARCH) }
pub fn home() -> Option<PathBuf> { std::env::var_os(if cfg!(windows){"USERPROFILE"}else{"HOME"}).map(PathBuf::from) }
pub fn data_dir() -> Option<PathBuf> {
 #[cfg(target_os="macos")] {return home().map(|p|p.join("Library/Application Support/ArtCraft Master Suite"));}
 #[cfg(windows)] { return std::env::var_os("LOCALAPPDATA").or_else(||std::env::var_os("APPDATA")).map(PathBuf::from).map(|p|p.join("ArtCraftLauncher")); }
 #[cfg(not(any(windows,target_os="macos")))] { std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p|p.is_absolute()).or_else(||home().map(|p|p.join(".local/share"))).map(|p|p.join("artcraft-master-suite")) }
}
pub fn config_root() -> Option<PathBuf> {
 #[cfg(target_os="macos")] {return home().map(|p|p.join("Library/Application Support"));}
 #[cfg(windows)] { return std::env::var_os("APPDATA").map(PathBuf::from); }
 #[cfg(not(any(windows,target_os="macos")))] {
  std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).filter(|p|p.is_absolute()).or_else(||home().map(|p|p.join(".config")))
 }
}
pub fn install_dir(id:&str)->Option<PathBuf> {
 #[cfg(target_os="macos")] {return home().map(|p|p.join("Applications/ArtCraft Apps").join(id));}
 #[cfg(windows)] {return std::env::var_os("LOCALAPPDATA").map(PathBuf::from).map(|p|p.join("Programs/ArtCraft Apps").join(id));}
 #[cfg(not(any(windows,target_os="macos")))] {data_dir().map(|p|p.join("apps").join(id))}
}
pub fn package_name(slug:&str,version:&str)->Result<String,String>{
 let suffix=match (std::env::consts::OS,std::env::consts::ARCH){
  ("windows","x86_64")=>"windows-x64-portable.zip",("windows","aarch64")=>"windows-arm64-portable.zip",("windows","x86")=>"windows-x86-portable.zip",
  ("macos","x86_64"|"aarch64")=>"macos-universal.dmg",
  ("linux","x86_64")=>"linux-x86_64.AppImage",("linux","aarch64")=>"linux-aarch64.AppImage",
  _=>return Err(format!("No supported package target for {}",label())),
 };Ok(format!("{slug}-{version}-{suffix}"))
}
pub fn executable(root:&Path,id:&str)->Option<PathBuf>{
 #[cfg(target_os="macos")] {let _=id;return crate::macos::find_executable(root);}
 #[cfg(target_os="linux")] { let path=root.join(format!("{}.AppImage",release_slug(id)));return path.is_file().then_some(path); }
 #[cfg(not(any(target_os="linux",target_os="macos")))] {
  for name in [format!("{}.exe",release_slug(id)),format!("{id}.exe")] {
   if let Some(entry)=WalkDir::new(root).max_depth(4).into_iter().filter_map(Result::ok).find(|e|e.file_type().is_file()&&e.file_name().to_string_lossy().eq_ignore_ascii_case(&name)){return Some(entry.into_path());}
  }None
 }
}
pub fn unpack(bytes:&[u8],directory:&Path,id:&str)->Result<(),String>{
 #[cfg(target_os="macos")] {let _=id;return crate::macos::unpack(bytes,directory);}
 #[cfg(target_os="linux")] {
  use std::os::unix::fs::PermissionsExt;
  validate_appimage(bytes)?;
  let path=directory.join(format!("{}.AppImage",release_slug(id)));fs::write(&path,bytes).map_err(|e|e.to_string())?;
  fs::set_permissions(path,fs::Permissions::from_mode(0o755)).map_err(|e|e.to_string())?;return Ok(());
 }
 #[cfg(not(any(target_os="linux",target_os="macos")))] {
  let _=id;let mut zip=zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e|e.to_string())?;let mut expanded=0u64;
  if zip.len()>100000{return Err("Package contains too many files".into());}
  for index in 0..zip.len(){
   let mut file=zip.by_index(index).map_err(|e|e.to_string())?;
   let relative=file.enclosed_name().ok_or("Unsafe package path")?;
   if file.unix_mode().is_some_and(|m|m&0o170000==0o120000){return Err("Symbolic links are not supported in Windows packages".into());}
   expanded=expanded.checked_add(file.size()).ok_or("Package size overflow")?;if expanded>4*1024*1024*1024{return Err("Expanded package exceeds 4 GB".into());}
   let output=directory.join(relative);if file.is_dir(){fs::create_dir_all(output).map_err(|e|e.to_string())?;}else{
    fs::create_dir_all(output.parent().ok_or("Invalid package path")?).map_err(|e|e.to_string())?;
    std::io::copy(&mut file,&mut fs::File::create(output).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
   }
  }Ok(())
 }
}
#[cfg(target_os="linux")]
pub fn validate_appimage(bytes:&[u8])->Result<(),String>{
 let machine=if cfg!(target_arch="aarch64"){183u16}else{62u16};
 if bytes.len()<20||&bytes[..4]!=b"\x7fELF"||bytes[4]!=2||bytes[5]!=1||&bytes[8..11]!=b"AI\x02"||u16::from_le_bytes([bytes[18],bytes[19]])!=machine{return Err("Download is not a compatible 64-bit Linux AppImage".into());}Ok(())
}
/// Run managed apps with explicit argv; Flatpak uses approved host integration.
pub fn spawn(command:&mut Command)->std::io::Result<std::process::Child>{
 #[cfg(target_os="linux")] {
  let appimage=Path::new(command.get_program()).extension().is_some_and(|s|s=="AppImage");
  if appimage{command.env("APPIMAGE_EXTRACT_AND_RUN","1");}
  if flatpak(){
   let mut host=Command::new("flatpak-spawn");host.arg("--host");
   if let Some(path)=command.get_current_dir(){host.arg(format!("--directory={}",path.display()));}
   if appimage { for key in ["XDG_CONFIG_HOME","XDG_DATA_HOME","XDG_CACHE_HOME"] {if let Some(value)=std::env::var_os(key){host.arg(format!("--env={key}={}",value.to_string_lossy()));}} }
   for(key,value)in command.get_envs(){if let Some(value)=value{host.arg(format!("--env={}={}",key.to_string_lossy(),value.to_string_lossy()));}}
   host.arg(command.get_program()).args(command.get_args());return host.spawn();
  }
  // Do not leak the suite AppImage's bundled libraries into other desktop apps.
  if std::env::var_os("APPIMAGE").is_some(){for key in ["LD_LIBRARY_PATH","LD_PRELOAD","APPDIR","APPIMAGE","OWD"]{command.env_remove(key);}}
 }
 command.spawn()
}

pub fn reveal_label() -> &'static str {
 if cfg!(target_os="macos") { "Show in Finder" } else if cfg!(target_os="windows") { "Show in File Explorer" } else { "Show in file manager" }
}
