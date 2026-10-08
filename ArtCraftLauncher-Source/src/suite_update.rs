//! Master Suite updates are accepted only from this project's published releases.
use std::{fs, io::{Read, Write}, path::PathBuf, time::Duration};
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
const REPO: &str = "ZifuM/Craft-apps-launcher-installer";
const MAX_DOWNLOAD: u64 = 1024 * 1024 * 1024;
#[derive(Clone)]
pub struct Prepared { pub version: String, pub installer: PathBuf, pub digest: String }
#[derive(Deserialize)]
struct Release { tag_name: String, draft: bool, prerelease: bool, assets: Vec<Asset> }
#[derive(Deserialize)]
struct Asset { name: String, browser_download_url: String, size: u64, digest: Option<String> }
fn version(text: &str) -> Option<[u64;3]> {
    let text=text.trim().trim_start_matches('v');
    let parts:Vec<_>=text.split('.').collect();if !(2..=3).contains(&parts.len()){return None;}
    let mut value=[0;3];for (i,part) in parts.iter().enumerate(){value[i]=part.parse().ok()?;}Some(value)
}
fn approved_url(url:&str)->bool {url.starts_with(&format!("https://github.com/{REPO}/releases/download/"))}
fn valid_hash(hash:&str)->bool {hash.len()==64 && hash.bytes().all(|c|c.is_ascii_hexdigit())}
fn hash_file(path:&std::path::Path)->Result<String,String>{
    let mut file=fs::File::open(path).map_err(|e|e.to_string())?;let mut hash=Sha256::new();let mut buf=[0;64*1024];
    loop{let n=file.read(&mut buf).map_err(|e|e.to_string())?;if n==0{break;}hash.update(&buf[..n]);}Ok(format!("{:x}",hash.finalize()))
}
pub fn prepare(current:&str, mut progress:impl FnMut(String))->Result<Option<Prepared>,String>{
    let client=Client::builder().user_agent("ArtCraft-Master-Suite-Updater").https_only(true).connect_timeout(Duration::from_secs(15)).timeout(Duration::from_secs(300)).build().map_err(|e|e.to_string())?;
    let response=client.get(format!("https://api.github.com/repos/{REPO}/releases/latest")).send().map_err(|e|format!("Could not check GitHub: {e}"))?;
    if response.status()==reqwest::StatusCode::NOT_FOUND{return Ok(None);}
    let release:Release=response.error_for_status().map_err(|e|e.to_string())?.json().map_err(|e|e.to_string())?;
    if release.draft||release.prerelease{return Ok(None);}
    let latest=version(&release.tag_name).ok_or("Release tag must use a stable version such as v2.3.0")?;
    if latest<=version(current).ok_or("Installed version is invalid")?{return Ok(None);}
    let asset=release.assets.iter().find(|a|a.name=="ArtCraftMasterSuite-Setup.exe")
        .or_else(||release.assets.iter().find(|a|matches!(a.name.as_str(),"Windows-64bit.Installer.zip"|"ArtCraftMasterSuite-Windows-x64.zip")))
        .ok_or("The release does not contain a supported Windows installer")?;
    if !approved_url(&asset.browser_download_url)||asset.size==0||asset.size>MAX_DOWNLOAD{return Err("Invalid update download".into());}
    let expected=if let Some(hash)=asset.digest.as_deref().and_then(|s|s.strip_prefix("sha256:")).filter(|s|valid_hash(s)){hash.to_lowercase()}
    else {
        let sums=release.assets.iter().find(|a|a.name=="SHA256SUMS.txt" && a.size<=64*1024 && approved_url(&a.browser_download_url)).ok_or("Release is missing its SHA-256 checksum")?;
        let mut text=String::new();client.get(&sums.browser_download_url).send().and_then(|r|r.error_for_status()).map_err(|e|e.to_string())?.take(65537).read_to_string(&mut text).map_err(|e|e.to_string())?;
        if text.len()>65536{return Err("Checksum file is too large".into());}
        text.lines().find_map(|line|{let mut parts=line.split_whitespace();let hash=parts.next()?;let name=parts.next()?.trim_start_matches('*');(name==asset.name&&valid_hash(hash)).then(||hash.to_lowercase())}).ok_or("No matching installer checksum")?
    };
    let root=std::env::var_os("LOCALAPPDATA").map(PathBuf::from).ok_or("Local application data is unavailable")?.join("ArtCraftLauncher").join("updates");
    fs::create_dir_all(&root).map_err(|e|e.to_string())?;
    let folder=root.join(format!("{}.{}.{}",latest[0],latest[1],latest[2]));fs::create_dir_all(&folder).map_err(|e|e.to_string())?;
    progress(format!("Downloading Master Suite {}...",release.tag_name));
    let partial=folder.join("download.part");
    let result=(||->Result<Prepared,String>{
        let mut response=client.get(&asset.browser_download_url).send().and_then(|r|r.error_for_status()).map_err(|e|e.to_string())?;
        let mut output=fs::File::create(&partial).map_err(|e|e.to_string())?;let mut hash=Sha256::new();let mut total=0u64;let mut buf=[0;64*1024];
        loop{let n=response.read(&mut buf).map_err(|e|e.to_string())?;if n==0{break;}total+=n as u64;if total>asset.size||total>MAX_DOWNLOAD{return Err("Update exceeds its declared size".into());}output.write_all(&buf[..n]).map_err(|e|e.to_string())?;hash.update(&buf[..n]);}
        output.sync_all().map_err(|e|e.to_string())?;drop(output);
        if total!=asset.size||format!("{:x}",hash.finalize())!=expected{return Err("Update checksum did not match. Nothing was installed.".into());}
        let installer=folder.join("ArtCraftMasterSuite-Setup.exe");
        if asset.name.ends_with(".zip"){
            let mut archive=zip::ZipArchive::new(fs::File::open(&partial).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            let candidates:Vec<_>=archive.file_names().filter(|n|n.rsplit('/').next()==Some("ArtCraftMasterSuite-Setup.exe")).map(str::to_owned).collect();
            if candidates.len()!=1{return Err("Update archive must contain one Master Suite installer".into());}
            let mut entry=archive.by_name(&candidates[0]).map_err(|e|e.to_string())?;
            if entry.size()>MAX_DOWNLOAD{return Err("Installer is too large".into());}
            let mut output=fs::File::create(&installer).map_err(|e|e.to_string())?;
            let n=std::io::copy(&mut entry.by_ref().take(MAX_DOWNLOAD+1),&mut output).map_err(|e|e.to_string())?;
            if n>MAX_DOWNLOAD{return Err("Installer is too large".into());}output.sync_all().map_err(|e|e.to_string())?;
        }else{fs::copy(&partial,&installer).map_err(|e|e.to_string())?;}
        let mut magic=[0;2];fs::File::open(&installer).map_err(|e|e.to_string())?.read_exact(&mut magic).map_err(|e|e.to_string())?;
        if magic!=*b"MZ"{return Err("Download is not a Windows installer".into());}
        let digest=hash_file(&installer)?;
        Ok(Prepared{version:release.tag_name,installer,digest})
    })();
    let _=fs::remove_file(partial);result.map(Some)
}
pub fn launch(update:&Prepared)->Result<(),String>{
    if hash_file(&update.installer)?!=update.digest{return Err("Staged installer changed; update cancelled".into());}
    let mut command=std::process::Command::new(&update.installer);
    command.arg("--auto-update").arg("--wait-pid").arg(std::process::id().to_string());
    #[cfg(target_os="windows")] { use std::os::windows::process::CommandExt;command.creation_flags(0x08000000); }
    command.spawn().map_err(|e|format!("Could not start the update: {e}"))?;Ok(())
}
