//! Workspace layout and the published Master Suite integration contract.
use super::*;
use serde_json::json;

pub fn prepare(root:&Path,app:&AppInfo)->Result<PathBuf,String>{
    let base=root.join(app.name);
    for folder in ["Projects","Exports","Assets","Plugins"]{fs::create_dir_all(base.join(folder)).map_err(|e|format!("Could not prepare {}: {e}",base.display()))?;}
    let config=json!({"schema":1,"app":app.id,"suiteVersion":VERSION,"projects":base.join("Projects"),"exports":base.join("Exports"),"assets":base.join("Assets"),"plugins":base.join("Plugins"),"preserveExistingDocumentPaths":true});
    let path=base.join(".artcraft-suite.json");let bytes=serde_json::to_vec_pretty(&config).map_err(|e|e.to_string())?;
    if fs::read(&path).ok().as_deref()!=Some(bytes.as_slice()){fs::write(&path,bytes).map_err(|e|e.to_string())?;}
    Ok(base)
}
pub fn launch_environment(command:&mut Command,root:&Path,app:&AppInfo)->Result<(),String>{
    let base=prepare(root,app)?;
    command.env("ARTCRAFT_APP_ID",app.id).env("ARTCRAFT_SUITE_CONFIG",base.join(".artcraft-suite.json"))
        .env("ARTCRAFT_PROJECTS_DIR",base.join("Projects"))
        .env("ARTCRAFT_EXPORTS_DIR",base.join("Exports"))
        .env("ARTCRAFT_ASSETS_DIR",base.join("Assets"))
        .env("ARTCRAFT_PLUGINS_DIR",base.join("Plugins"));
    Ok(())
}
