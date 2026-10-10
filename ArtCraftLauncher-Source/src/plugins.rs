use super::*;
use serde_json::{Value, json};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
mod github;
mod packages;
pub use github::Download;
const MAX: u64 = 32 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub source: String,
    pub hash: String,
    pub stored: PathBuf,
    pub deployed: PathBuf,
    pub enabled: bool,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub disabled: Option<PathBuf>,
}
enum Event {
    Progress(String),
    Choices(Vec<Download>),
    Finished(Result<String, String>),
}
#[derive(Default)]
pub struct Manager {
    pub link: String,
    pub message: String,
    pub busy: bool,
    pub failed: bool,
    pub remove: Option<(String, usize)>,
    pub choices: Vec<Download>,
    pub target: Option<(AppInfo, PathBuf)>,
    pub search: String,
    rx: Option<Receiver<Event>>,
}
impl Manager {
    pub fn tick(&mut self) {
        let events: Vec<_> = self
            .rx
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for event in events {
            match event {
                Event::Progress(message) => self.message = message,
                Event::Choices(choices) => {
                    self.choices = choices;
                    self.busy = false;
                    self.rx = None;
                    self.message = "Choose a compiled plugin download from this release.".into();
                }
                Event::Finished(result) => {
                    self.failed = result.is_err();
                    self.message =
                        result.unwrap_or_else(|e| format!("Could not install plugin: {e}"));
                    self.busy = false;
                    self.rx = None;
                }
            }
        }
    }
    pub fn install(&mut self, app: AppInfo, root: PathBuf, source: String, local: bool) {
        self.start(app, root, source, local, None);
    }
    pub fn choose(&mut self, download: Download) {
        if let Some((app, root)) = self.target.clone() {
            self.start(app, root, download.url.clone(), false, Some(download));
        }
    }
    fn start(
        &mut self,
        app: AppInfo,
        root: PathBuf,
        source: String,
        local: bool,
        chosen: Option<Download>,
    ) {
        if self.busy {
            return;
        }
        self.choices.clear();
        self.failed = false;
        self.target = Some((app, root.clone()));
        self.busy = true;
        self.message = if local {
            "Checking plugin package…"
        } else {
            "Looking up GitHub download…"
        }
        .into();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        thread::spawn(move || {
            let result = (|| {
                if !supported(app.id) {
                    return Err("This app has no supported plugin installer yet.".into());
                }
                if running(&app)? {
                    return Err(format!("Close {} before installing plugins.", app.name));
                }
                let (packages, origin) = if local {
                    (packages::from_local(Path::new(&source), &app)?, source)
                } else {
                    let download = if let Some(download) = chosen {
                        download
                    } else {
                        let mut choices = github::resolve(&source, &app)?;
                        if choices.len() > 1 {
                            let _ = tx.send(Event::Choices(choices));
                            return Ok(None);
                        }
                        choices.remove(0)
                    };
                    let _ = tx.send(Event::Progress(format!("Downloading {}…", download.name)));
                    let bytes = github::fetch(&download)?;
                    let _ = tx.send(Event::Progress(
                        "Checking compatibility and installation folders…".into(),
                    ));
                    (
                        packages::from_bytes(&download.name, bytes, &app)?,
                        download.url,
                    )
                };
                let _ = tx.send(Event::Progress(format!(
                    "Installing plugins for {}…",
                    app.name
                )));
                install_packages(&app, &root, &origin, packages).map(Some)
            })();
            match result {
                Ok(None) => {}
                Ok(Some(message)) => {
                    let _ = tx.send(Event::Finished(Ok(message)));
                }
                Err(error) => {
                    let _ = tx.send(Event::Finished(Err(error)));
                }
            }
        });
    }
}
pub fn supported(id: &str) -> bool {
    matches!(
        id,
        "photocraft" | "vectorcraft" | "effectcraft" | "soundcraft"
    )
}
pub fn formats(id: &str) -> &'static str {
    if id == "soundcraft" {
        if cfg!(target_os = "macos") {
            "CLAP, VST3 and Audio Unit bundles · ZIP packages"
        } else {
            "CLAP and VST3 plugins · ZIP packages"
        }
    } else {
        "WebAssembly (.wasm) · ZIP packages"
    }
}
pub fn documentation(app: &AppInfo) -> String {
    let suffix = if matches!(app.id, "photocraft" | "vectorcraft" | "effectcraft") {
        "/blob/main/docs/plugins.md"
    } else {
        "#readme"
    };
    format!(
        "https://github.com/storytold/{}{suffix}",
        release_slug(app.id)
    )
}
pub fn locations(app: &AppInfo, root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    if app.id == "soundcraft" {
        let mut formats = vec!["clap", "vst3"];
        if cfg!(target_os = "macos") {
            formats.push("component");
        }
        return formats
            .into_iter()
            .map(|format| Ok((format.to_uppercase(), audio_folder(format)?)))
            .collect();
    }
    Ok(vec![(
        "WebAssembly".into(),
        destination(app, &root.join(app.name), false)?,
    )])
}
fn audio_folder(format: &str) -> Result<PathBuf, String> {
    let name = match format {
        "clap" => "CLAP",
        "vst3" => "VST3",
        "component" if cfg!(target_os = "macos") => "Components",
        _ => return Err("Unsupported audio plugin format on this platform.".into()),
    };
    #[cfg(windows)]
    {
        return std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("Programs/Common").join(name))
            .ok_or_else(|| "Local application folder is unavailable.".into());
    }
    #[cfg(target_os = "macos")]
    {
        return platform::home()
            .map(|p| p.join("Library/Audio/Plug-Ins").join(name))
            .ok_or_else(|| "Home folder is unavailable.".into());
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = name;
        platform::home()
            .map(|p| p.join(format!(".{format}")))
            .ok_or_else(|| "Home folder is unavailable.".into())
    }
}
fn registry(app: &AppInfo) -> Result<PathBuf, String> {
    Ok(app_data()
        .ok_or("Settings directory unavailable")?
        .join("plugins")
        .join(format!("{}.json", app.id)))
}
pub fn entries(app: &AppInfo) -> Result<Vec<Entry>, String> {
    let path = registry(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Plugin registry could not be read: {e}"))
}
fn save(app: &AppInfo, entries: &[Entry]) -> Result<(), String> {
    let path = registry(app)?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    atomic(
        &path,
        &serde_json::to_vec_pretty(entries).map_err(|e| e.to_string())?,
    )
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension(format!("suite-new-{}", std::process::id()));
    let mut created = false;
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        created = true;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&tmp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() && created {
        let _ = fs::remove_file(&tmp);
    }
    result
}
fn read_limit(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    if f.metadata().map_err(|e| e.to_string())?.len() > max {
        return Err("Plugin package exceeds the size limit.".into());
    }
    let mut bytes = Vec::new();
    f.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err("Plugin package exceeds the size limit.".into());
    }
    Ok(bytes)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn validate(bytes: &[u8], app: &AppInfo) -> Result<(), String> {
    use wasmparser::{ExternalKind, Payload, ValType::*};
    if bytes.len() as u64 > MAX {
        return Err("A WebAssembly plugin exceeds 32 MB.".into());
    }
    let mut features = wasmparser::WasmFeatures::default();
    features.remove(
        wasmparser::WasmFeatures::SIMD
            | wasmparser::WasmFeatures::RELAXED_SIMD
            | wasmparser::WasmFeatures::MEMORY64
            | wasmparser::WasmFeatures::THREADS
            | wasmparser::WasmFeatures::COMPONENT_MODEL,
    );
    wasmparser::Validator::new_with_features(features)
        .validate_all(bytes)
        .map_err(|e| format!("Unsupported WebAssembly module: {e}"))?;
    let mut types = Vec::new();
    let mut functions = Vec::new();
    let mut exports = HashMap::new();
    let mut memories = 0;
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        match payload.map_err(|e| e.to_string())? {
            Payload::Version { encoding, .. } if encoding != wasmparser::Encoding::Module => {
                return Err("Choose a core WebAssembly plugin module.".into());
            }
            Payload::ImportSection(section) if section.count() > 0 => {
                return Err(
                    "Craft WebAssembly plugins cannot import host functions or WASI.".into(),
                );
            }
            Payload::TypeSection(section) => {
                for ty in section.into_iter_err_on_gc_types() {
                    types.push(ty.map_err(|e| e.to_string())?);
                }
            }
            Payload::FunctionSection(section) => {
                for ty in section {
                    functions.push(ty.map_err(|e| e.to_string())?);
                }
            }
            Payload::MemorySection(section) => {
                memories = section.count();
                for memory in section {
                    let memory = memory.map_err(|e| e.to_string())?;
                    let max = if app.id == "effectcraft" { 16384 } else { 8192 };
                    if memory.initial > max || memory.shared {
                        return Err("Plugin memory exceeds the app's limits.".into());
                    }
                }
            }
            Payload::ExportSection(section) => {
                for export in section {
                    let export = export.map_err(|e| e.to_string())?;
                    exports.insert(export.name.to_owned(), (export.kind, export.index));
                }
            }
            _ => {}
        }
    }
    let identified = if exports.contains_key("pc_filter") {
        "photocraft"
    } else if exports.contains_key("vc_run") {
        "vectorcraft"
    } else if exports.contains_key("ec_render") {
        "effectcraft"
    } else {
        "unknown"
    };
    if identified != app.id {
        return Err(format!(
            "This plugin targets {identified}, not {}. Open the matching app's Plugins tab.",
            app.name
        ));
    }
    if memories != 1 || !matches!(exports.get("memory"), Some((ExternalKind::Memory, 0))) {
        return Err("The plugin must export its single linear memory as 'memory'.".into());
    }
    let required: Vec<(&str, Vec<wasmparser::ValType>, wasmparser::ValType)> = match app.id {
        "photocraft" => vec![
            ("pc_abi_version", vec![], I32),
            ("pc_manifest", vec![], I64),
            ("pc_alloc", vec![I32], I32),
            ("pc_filter", vec![I32; 8], I32),
        ],
        "vectorcraft" => vec![
            ("vc_abi_version", vec![], I32),
            ("vc_manifest", vec![], I64),
            ("vc_alloc", vec![I32], I32),
            ("vc_run", vec![I32; 4], I64),
        ],
        "effectcraft" => vec![
            ("ec_api_version", vec![], I32),
            ("ec_manifest_ptr", vec![], I32),
            ("ec_manifest_len", vec![], I32),
            ("ec_alloc", vec![I32], I32),
            ("ec_render", vec![I32, I32, I32, I32, I32, F64, F64], I32),
        ],
        _ => return Err("No WebAssembly plugin loader is available for this app.".into()),
    };
    for (name, params, result) in required {
        let ty = exports
            .get(name)
            .filter(|(kind, _)| *kind == ExternalKind::Func)
            .and_then(|(_, i)| functions.get(*i as usize))
            .and_then(|i| types.get(*i as usize));
        if !ty.is_some_and(|ty| ty.params() == params && ty.results() == [result]) {
            return Err(format!(
                "The export {name} does not match {}'s plugin interface.",
                app.name
            ));
        }
    }
    Ok(())
}
fn config_path(app: &AppInfo) -> Result<PathBuf, String> {
    if app.id == "photocraft" {
        if let Some(path) = std::env::var_os("PHOTOCRAFT_CONFIG_DIR").filter(|p| !p.is_empty()) {
            return Ok(PathBuf::from(path).join("preferences.json"));
        }
        if let Some(exe) = installed_dir(app.id).and_then(|d| find_executable(&d, app.id)) {
            if let Some(dir) = exe.parent() {
                if ["portable.txt", "PhotoCraft.portable"]
                    .iter()
                    .any(|n| dir.join(n).exists())
                {
                    return Ok(dir.join("PhotoCraftData/preferences.json"));
                }
            }
        }
    }
    let root = platform::config_root().ok_or("Application configuration folder unavailable")?;
    #[cfg(target_os = "linux")]
    {
        return Ok(match app.id {
            "photocraft" => root.join("photocraft/preferences.json"),
            "vectorcraft" => root.join("vectorcraft/ui.json"),
            "effectcraft" => std::env::var_os("EFFECTCRAFT_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or(root.join("effectcraft"))
                .join("prefs.json"),
            _ => return Err("No plugin adapter is available".into()),
        });
    }
    #[cfg(not(target_os = "linux"))]
    Ok(match app.id {
        "photocraft" => root.join("Photocraft/preferences.json"),
        "vectorcraft" => root.join("VectorCraft/ui.json"),
        "effectcraft" => std::env::var_os("EFFECTCRAFT_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or(root.join("EffectCraft"))
            .join("prefs.json"),
        _ => return Err("No plugin adapter is available for this app.".into()),
    })
}
fn json_file(path: &Path) -> Result<Value, String> {
    if path.exists() {
        let v: Value = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("App settings are unreadable: {e}"))?;
        if !v.is_object() {
            return Err("App settings have an unexpected format.".into());
        }
        Ok(v)
    } else {
        Ok(json!({}))
    }
}
fn save_config(path: &Path, value: &Value) -> Result<(), String> {
    fs::create_dir_all(path.parent().ok_or("Invalid settings path")?).map_err(|e| e.to_string())?;
    let backup = path.with_extension("pre-master-suite.json");
    if path.exists() && !backup.exists() {
        fs::copy(path, &backup).map_err(|e| e.to_string())?;
    }
    atomic(
        path,
        &serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
}
fn destination(app: &AppInfo, base: &Path, configure: bool) -> Result<PathBuf, String> {
    let path = config_path(app)?;
    // VectorCraft imports preferences from its former name until it has saved
    // its own file. Preserve that import when configuring its plugin folder.
    let legacy = if app.id == "vectorcraft" && !path.exists() {
        path.parent().and_then(Path::parent).map(|root| {
            root.join(if cfg!(target_os = "linux") { "drawcraft" } else { "DrawCraft" })
                .join("ui.json")
        }).filter(|path| path.exists())
    } else {
        None
    };
    let mut value = json_file(legacy.as_deref().unwrap_or(&path))?;
    let key = if app.id == "photocraft" {
        "/plugIns/additionalPluginsFolder"
    } else {
        "/engine_prefs/pluginsFolder"
    };
    if app.id == "effectcraft" {
        return Ok(path.parent().unwrap().join("Plug-ins"));
    }
    let dest = value
        .pointer(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("Plugins"));
    if !dest.is_absolute() {
        return Err("The app's existing plugin folder is relative. Set an absolute folder in its preferences first.".into());
    }
    let section = if app.id == "photocraft" {
        "plugIns"
    } else {
        "engine_prefs"
    };
    if !value[section].is_null() && !value[section].is_object() {
        return Err("App plugin preferences have an unexpected format.".into());
    }
    if app.id == "photocraft" {
        value["plugIns"]["additionalPluginsFolder"] = json!(dest);
        value["plugIns"]["useAdditionalPluginsFolder"] = json!(true);
    } else {
        value["engine_prefs"]["pluginsFolder"] = json!(dest);
    }
    if configure {
        save_config(&path, &value)?;
    }
    Ok(dest)
}
pub fn prepare_launch(app: &AppInfo, root: &Path) -> Result<(), String> {
    let base = workspace_bridge::prepare(root, app)?;
    if running(app)? {
        return Ok(());
    }
    if app.id == "effectcraft" {
        let path = config_path(app)?;
        let mut value = json_file(&path)?;
        if !value["export"].is_null() && !value["export"].is_object() {
            return Err("Export settings have an unexpected format.".into());
        }
        value["export"]["defaultOutputFolder"] = json!(base.join("Exports"));
        save_config(&path, &value)?;
    }
    if supported(app.id) && app.id != "soundcraft" && !entries(app)?.is_empty() {
        let _ = destination(app, &base, true)?;
    }
    Ok(())
}

struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn lock(app: &AppInfo) -> Result<Lock, String> {
    let path = registry(app)?.with_extension("lock");
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|_| "Another plugin operation may be active. Close other suite instances; if none are running, remove the stale plugin lock from the suite settings folder.")?;
    Ok(Lock(path))
}
fn delete_owned(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}
pub fn active_path(entry: &Entry) -> PathBuf {
    if entry.enabled {
        entry.deployed.clone()
    } else {
        entry
            .disabled
            .clone()
            .unwrap_or_else(|| entry.deployed.with_extension("wasm.disabled"))
    }
}
fn install_packages(
    app: &AppInfo,
    root: &Path,
    source: &str,
    packages: Vec<packages::Package>,
) -> Result<String, String> {
    let _guard = lock(app)?;
    if running(app)? {
        return Err(format!("Close {} before installing plugins.", app.name));
    }
    let mut list = entries(app)?;
    let base = workspace_bridge::prepare(root, app)?;
    let config = if app.id == "soundcraft" {
        None
    } else {
        Some(config_path(app)?)
    };
    let before = config
        .as_ref()
        .filter(|p| p.exists())
        .map(fs::read)
        .transpose()
        .map_err(|e| e.to_string())?;
    let mut created = Vec::<PathBuf>::new();
    let mut config_written = false;
    let result = (|| {
        let mut added = 0;
        for package in packages {
            let native = app.id == "soundcraft";
            let folder = if native {
                audio_folder(&package.format)?
            } else {
                destination(app, &base, false)?
            };
            fs::create_dir_all(&folder)
                .map_err(|e| format!("Cannot write to {}: {e}", folder.display()))?;
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            let staging = folder.join(format!(".suite-plugin-{}-{nonce}", std::process::id()));
            if staging.exists() {
                return Err("A staging file already exists; try again.".into());
            }
            created.push(staging.clone());
            packages::write(&package, &staging)?;
            let digest = packages::digest(&staging)?;
            if list.iter().any(|entry| entry.hash == digest) {
                delete_owned(&staging).map_err(|e| e.to_string())?;
                created.pop();
                continue;
            }
            let filename = if native {
                package.name.clone()
            } else {
                format!("suite-{digest}.wasm")
            };
            let deployed = folder.join(&filename);
            let stored = base.join("Plugins").join(&filename);
            if deployed.exists() || (stored != deployed && stored.exists()) {
                return Err(format!(
                    "{} already exists. Remove or rename the existing plugin first; it has not been overwritten.",
                    filename
                ));
            }
            if stored != deployed {
                created.push(stored.clone());
                packages::write(&package, &stored)?;
                if packages::digest(&stored)? != digest {
                    return Err("Plugin copy verification failed.".into());
                }
            }
            if running(app)? {
                return Err(format!(
                    "{} opened during installation. Close it and try again.",
                    app.name
                ));
            }
            fs::rename(&staging, &deployed).map_err(|e| e.to_string())?;
            created.retain(|p| p != &staging);
            created.push(deployed.clone());
            let disabled = if native {
                Some(
                    folder
                        .parent()
                        .ok_or("Invalid audio plugin folder")?
                        .join(".ArtCraftDisabledPlugins")
                        .join(&package.format)
                        .join(&filename),
                )
            } else {
                None
            };
            list.push(Entry {
                name: package.name,
                source: source.into(),
                hash: digest,
                stored,
                deployed,
                enabled: true,
                format: package.format,
                disabled,
            });
            added += 1;
        }
        if added == 0 {
            return Ok("These plugins are already installed. No files were changed.".into());
        }
        if app.id != "soundcraft" {
            destination(app, &base, true)?;
            config_written = true;
        }
        save(app, &list)?;
        Ok(format!(
            "Installed {added} plugin(s) for {}. Restart {} to load them. The app checks each plugin's runtime compatibility on launch.",
            app.name, app.name
        ))
    })();
    if result.is_err() {
        for path in created.iter().rev() {
            let _ = delete_owned(path);
        }
        if config_written {
            if let Some(path) = config {
                if let Some(bytes) = before {
                    let _ = atomic(&path, &bytes);
                } else {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }
    result
}
pub fn toggle(app: &AppInfo, index: usize) -> Result<(), String> {
    let _guard = lock(app)?;
    if running(app)? {
        return Err(format!("Close {} before changing plugins.", app.name));
    }
    let mut list = entries(app)?;
    let entry = list.get_mut(index).ok_or("Plugin no longer exists")?;
    let from = active_path(entry);
    let to = if entry.enabled {
        entry
            .disabled
            .clone()
            .unwrap_or_else(|| entry.deployed.with_extension("wasm.disabled"))
    } else {
        entry.deployed.clone()
    };
    if packages::digest(&from)? != entry.hash {
        return Err(
            "Plugin files were changed outside Master Suite; they were left untouched.".into(),
        );
    }
    if to.exists() {
        return Err("The destination already exists; both files were left untouched.".into());
    }
    fs::create_dir_all(to.parent().ok_or("Invalid plugin destination")?)
        .map_err(|e| e.to_string())?;
    fs::rename(&from, &to).map_err(|e| e.to_string())?;
    entry.enabled = !entry.enabled;
    if let Err(error) = save(app, &list) {
        let _ = fs::rename(to, from);
        return Err(error);
    }
    Ok(())
}
pub fn uninstall(app: &AppInfo, index: usize) -> Result<(), String> {
    let _guard = lock(app)?;
    if running(app)? {
        return Err(format!("Close {} before removing plugins.", app.name));
    }
    let mut list = entries(app)?;
    let entry = list.get(index).ok_or("Plugin no longer exists")?.clone();
    let mut paths = vec![active_path(&entry)];
    paths.retain(|path| path.exists());
    if entry.stored != entry.deployed && entry.stored.exists() {
        paths.push(entry.stored);
    }
    for path in &paths {
        if packages::digest(path)? != entry.hash {
            return Err(
                "Plugin files were modified outside Master Suite; they were left untouched.".into(),
            );
        }
        if path.with_extension("suite-removed").exists() {
            return Err("An earlier removal backup exists; it was left untouched.".into());
        }
    }
    let mut staged = Vec::<(PathBuf, PathBuf)>::new();
    for path in paths {
        let backup = path.with_extension("suite-removed");
        if let Err(error) = fs::rename(&path, &backup) {
            for (original, backup) in staged.iter().rev() {
                let _ = fs::rename(backup, original);
            }
            return Err(error.to_string());
        }
        staged.push((path, backup));
    }
    list.remove(index);
    if let Err(error) = save(app, &list) {
        for (original, backup) in staged.iter().rev() {
            let _ = fs::rename(backup, original);
        }
        return Err(error);
    }
    for (_, backup) in staged {
        let _ = delete_owned(&backup);
    }
    Ok(())
}
pub fn running(app: &AppInfo) -> Result<bool, String> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
            System::Diagnostics::ToolHelp::*,
        };
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err("Could not check whether the app is running.".into());
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut next = Process32FirstW(snapshot, &mut entry);
        let mut found = false;
        while next != 0 {
            let length = entry
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..length]).to_lowercase();
            if name == format!("{}.exe", app.id)
                || name == format!("{}.exe", app.name.to_lowercase())
            {
                found = true;
                break;
            }
            next = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
        Ok(found)
    }
    #[cfg(not(windows))]
    {
        let mut command = if platform::flatpak() {
            let mut c = Command::new("flatpak-spawn");
            c.args(["--host", "pgrep"]);
            c
        } else {
            Command::new("pgrep")
        };
        let output = command
            .args(["-ix", release_slug(app.id)])
            .output()
            .map_err(|e| format!("Could not check running apps: {e}"))?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(
                "Could not check running apps. Install procps/pgrep before managing plugins."
                    .into(),
            ),
        }
    }
}

pub fn relocate_workspace(source: &Path, destination: &Path) -> Result<(), String> {
    let rebase = |path: &mut PathBuf| {
        if let Ok(relative) = path.strip_prefix(source) {
            *path = destination.join(relative);
        }
    };
    for app in APPS.iter().filter(|app| supported(app.id)) {
        let mut list = entries(app)?;
        let before = serde_json::to_vec(&list).map_err(|e| e.to_string())?;
        for entry in &mut list {
            rebase(&mut entry.stored);
            rebase(&mut entry.deployed);
            if let Some(path) = &mut entry.disabled {
                rebase(path);
            }
        }
        if before != serde_json::to_vec(&list).map_err(|e| e.to_string())? {
            save(app, &list)?;
        }
        if app.id == "soundcraft" {
            continue;
        }
        let config = config_path(app)?;
        if !config.exists() {
            continue;
        }
        let mut value = json_file(&config)?;
        let key = if app.id == "photocraft" {
            "/plugIns/additionalPluginsFolder"
        } else {
            "/engine_prefs/pluginsFolder"
        };
        if let Some(field) = value.pointer_mut(key) {
            if let Some(path) = field.as_str().map(PathBuf::from) {
                if let Ok(relative) = path.strip_prefix(source) {
                    *field = json!(destination.join(relative));
                    save_config(&config, &value)?;
                }
            }
        }
    }
    Ok(())
}
