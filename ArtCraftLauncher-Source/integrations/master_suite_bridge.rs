//! App-side adapter for the Master Suite launch contract, schema 1.
//! Integrate this into a Craft app's native file-dialog layer; it is NOT a WASM filter.
//! Existing document paths take priority over these defaults.
use std::path::{Path,PathBuf};

#[derive(Clone,Debug)]
pub struct Workspace {pub projects:PathBuf,pub exports:PathBuf,pub assets:PathBuf,pub plugins:PathBuf}
impl Workspace {
    /// Returns None for ordinary standalone launches or another app's environment.
    pub fn from_launcher(expected_app:&str)->Option<Self>{
        if std::env::var("ARTCRAFT_APP_ID").ok()?.as_str()!=expected_app{return None;}
        let folder=|key:&str|->Option<PathBuf>{let path=PathBuf::from(std::env::var_os(key)?);if path.is_absolute()&&path.is_dir(){Some(path)}else{None}};
        Some(Self{projects:folder("ARTCRAFT_PROJECTS_DIR")?,exports:folder("ARTCRAFT_EXPORTS_DIR")?,assets:folder("ARTCRAFT_ASSETS_DIR")?,plugins:folder("ARTCRAFT_PLUGINS_DIR")?})
    }
    /// Use for a NEW document's Save As dialog only. Never redirect an existing Save.
    pub fn save_directory<'a>(&'a self,current_document:Option<&'a Path>)->&'a Path{
        current_document.and_then(Path::parent).unwrap_or(&self.projects)
    }
    /// Suggest this folder for exports; the user must still be able to choose another.
    pub fn export_directory(&self)->&Path{&self.exports}
}
