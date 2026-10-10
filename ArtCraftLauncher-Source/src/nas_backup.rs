//! Backups to a share mounted by the operating system; no network passwords are stored.
use super::*;
use crate::google_drive::{Snapshot, Version, cancelled};
use std::{io::Write, sync::atomic::AtomicBool};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub root: Option<PathBuf>,
    pub identity: String,
    pub versions: Vec<Version>,
    pub confirmed: HashMap<PathBuf, (u64, String)>,
}

fn nonce() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "Could not prepare the NAS operation.")?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

struct Pending(PathBuf);
impl Drop for Pending {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn safe_path(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let mut path = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(name) = component else {
            return Err("Invalid NAS backup path.".into());
        };
        path.push(name);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err("NAS backup folders must not contain symbolic links.".into());
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(format!("Cannot access NAS path: {error}"));
            }
            _ => (),
        }
    }
    Ok(path)
}

pub fn connect(folder: &Path) -> Result<Settings, String> {
    if !folder.is_dir() {
        return Err(
            "The NAS folder is unavailable. Connect the share in your file manager first.".into(),
        );
    }
    let root = safe_path(folder, Path::new("ArtCraft Master Suite"))?;
    fs::create_dir_all(&root).map_err(|e| format!("Cannot create the NAS backup folder: {e}"))?;
    let marker = safe_path(&root, Path::new(".artcraft-nas.json"))?;
    if !marker.exists() {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
            .map_err(|e| e.to_string())?;
        file.write_all(
            serde_json::to_string(&serde_json::json!({"identity":nonce()?, "schema":1}))
                .unwrap()
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&marker).map_err(|e| e.to_string())?)
            .map_err(|_| "The NAS backup marker is invalid.")?;
    let identity = value["identity"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or("The NAS backup marker is missing its identity.")?
        .to_owned();
    let settings = Settings {
        root: Some(root),
        identity,
        ..Default::default()
    };
    check(&settings)?;
    Ok(settings)
}

pub fn check(settings: &Settings) -> Result<PathBuf, String> {
    let root = settings.root.as_ref().ok_or("Choose a NAS folder first.")?;
    // Never recreate a missing mount: otherwise an offline share could silently back up locally.
    let marker = safe_path(root, Path::new(".artcraft-nas.json"))?;
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(marker)
            .map_err(|_| "The NAS is unavailable. Reconnect the same network share and retry.")?,
    )
    .map_err(|_| "The NAS backup marker is invalid.")?;
    if value["identity"].as_str() != Some(settings.identity.as_str()) {
        return Err("This is a different NAS folder. Connect it again before backing up.".into());
    }
    let probe = Pending(root.join(format!(".artcraft-write-check-{}", nonce()?)));
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe.0)
        .map_err(|e| format!("The NAS is not writable: {e}"))?;
    output
        .write_all(b"ArtCraft NAS connection check")
        .and_then(|_| output.sync_all())
        .map_err(|e| format!("NAS write failed: {e}"))?;
    drop(output);
    if fs::read(&probe.0).map_err(|e| e.to_string())? != b"ArtCraft NAS connection check" {
        return Err("The NAS did not confirm the connection check.".into());
    }
    Ok(root.clone())
}

fn digest(path: &Path, cancel: &AtomicBool) -> Result<(u64, String), String> {
    let mut input = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut bytes = vec![0u8; 1024 * 1024];
    let mut size = 0;
    loop {
        cancelled(cancel)?;
        let count = input.read(&mut bytes).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
        size += count as u64;
    }
    Ok((size, format!("{:x}", hash.finalize())))
}

fn copy_verified(
    input: &Path,
    destination: &Path,
    bytes: u64,
    hash: &str,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64, &str),
) -> Result<(), String> {
    let pending = Pending(destination.with_file_name(format!(".artcraft-part-{}", nonce()?)));
    let mut source = fs::File::open(input).map_err(|e| e.to_string())?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending.0)
        .map_err(|e| e.to_string())?;
    let mut buffer = vec![0; 1024 * 1024];
    let mut sent = 0;
    loop {
        cancelled(cancel)?;
        let count = source.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        output
            .write_all(&buffer[..count])
            .map_err(|e| e.to_string())?;
        sent += count as u64;
        progress(sent, bytes, "Copying to NAS");
    }
    output.sync_all().map_err(|e| e.to_string())?;
    drop(output);
    progress(bytes, bytes, "Verifying NAS copy");
    if digest(&pending.0, cancel)? != (bytes, hash.to_owned()) {
        return Err("The NAS copy did not match the original file. Retry Sync.".into());
    }
    cancelled(cancel)?;
    replace(&pending.0, destination)?;
    Ok(())
}

fn replace(source: &Path, destination: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };
        let source = fs::canonicalize(source).map_err(|e| e.to_string())?;
        let destination = fs::canonicalize(
            destination
                .parent()
                .ok_or("The NAS destination has no parent folder.")?,
        )
        .map_err(|e| e.to_string())?
        .join(
            destination
                .file_name()
                .ok_or("The NAS destination has no filename.")?,
        );
        let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        if unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(format!(
                "Could not finish the NAS backup: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, destination).map_err(|e| format!("Could not finish the NAS backup: {e}"))
    }
}

fn save_version(
    root: &Path,
    input: &Path,
    mut version: Version,
    cancel: &AtomicBool,
    progress: impl FnMut(u64, u64, &str),
) -> Result<Version, String> {
    let key = format!(
        "{:x}",
        Sha256::digest(format!(
            "{}:{}:{}",
            version.source, version.destination, version.digest
        ))
    );
    let relative = PathBuf::from(".artcraft-versions").join(&key);
    let folder = safe_path(root, &relative)?;
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let data = safe_path(root, &relative.join("data"))?;
    if !data.is_file() || digest(&data, cancel)? != (version.bytes, version.digest.clone()) {
        copy_verified(
            input,
            &data,
            version.bytes,
            &version.digest,
            cancel,
            progress,
        )?;
    }
    version.id = format!("nas:{key}");
    // Forward slashes keep shared version records usable across Windows, macOS and Linux.
    version.local_file = Some(format!(".artcraft-versions/{key}/data"));
    let receipt = safe_path(root, &relative.join("version.json"))?;
    let pending = Pending(folder.join(format!(".receipt-{}", nonce()?)));
    fs::write(
        &pending.0,
        serde_json::to_vec_pretty(&version).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    replace(&pending.0, &receipt)?;
    Ok(version)
}

pub fn backup(
    settings: &Settings,
    project: &Project,
    destination: &cloud_layout::Destination,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64, &str),
) -> Result<Version, String> {
    let root = check(settings)?;
    cancelled(cancel)?;
    let lock_path = safe_path(&root, Path::new(".artcraft-sync.lock"))?;
    let lock_file = fs::OpenOptions::new().write(true).create_new(true).open(&lock_path)
        .map_err(|_| "The NAS is already being backed up. Wait for it to finish. If a previous backup was interrupted, remove .artcraft-sync.lock from the NAS backup folder once all Master Suite copies are closed.")?;
    let _lock = Pending(lock_path);
    drop(lock_file);
    let snapshot = Snapshot::create(project, cancel)?;
    let mut relative = PathBuf::new();
    for folder in &destination.folders {
        relative.push(folder);
    }
    relative.push(
        project
            .path
            .file_name()
            .ok_or("The source filename is missing.")?,
    );
    let output = safe_path(&root, &relative)?;
    if fs::canonicalize(&project.path).ok() == fs::canonicalize(&output).ok() {
        return Err("Choose a NAS backup folder separate from your original project files.".into());
    }
    fs::create_dir_all(output.parent().unwrap()).map_err(|e| e.to_string())?;
    let version = Version {
        id: String::new(),
        name: project.title.clone(),
        bytes: snapshot.size,
        modified: Local::now().to_rfc3339(),
        digest: snapshot.digest.clone(),
        source: google_drive::source_id(&project.path),
        revision: project.revision,
        app: project.app.id.into(),
        destination: destination.key(),
        local_file: None,
    };
    if output.is_file() {
        let (bytes, hash) = digest(&output, cancel)?;
        if hash != snapshot.digest {
            let previous = Version {
                bytes,
                digest: hash,
                revision: 0,
                ..version.clone()
            };
            save_version(&root, &output, previous, cancel, |_, _, _| {})?;
        }
    }
    let saved = save_version(&root, &snapshot.path, version, cancel, &mut progress)?;
    if !output.is_file() || digest(&output, cancel)? != (snapshot.size, snapshot.digest.clone()) {
        copy_verified(
            &snapshot.path,
            &output,
            snapshot.size,
            &snapshot.digest,
            cancel,
            &mut progress,
        )?;
    }
    if digest(&output, cancel)? != (snapshot.size, snapshot.digest.clone()) {
        return Err("The NAS did not confirm the final backup file.".into());
    }
    Ok(saved)
}

pub fn versions(settings: &Settings, cancel: &AtomicBool) -> Result<Vec<Version>, String> {
    let root = check(settings)?;
    let folder = safe_path(&root, Path::new(".artcraft-versions"))?;
    if !folder.is_dir() {
        return Ok(Vec::new());
    }
    let mut versions = Vec::new();
    for entry in WalkDir::new(folder)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !e.file_type().is_symlink())
    {
        cancelled(cancel)?;
        let entry = entry.map_err(|e| format!("Could not read NAS versions: {e}"))?;
        if !entry.file_type().is_file() || entry.file_name() != "version.json" {
            continue;
        }
        if entry.metadata().map_err(|e| e.to_string())?.len() > 64 * 1024 {
            continue;
        }
        let version: Version =
            serde_json::from_slice(&fs::read(entry.path()).map_err(|e| e.to_string())?)
                .map_err(|_| "A NAS version record could not be read.")?;
        if let Some(relative) = &version.local_file {
            if safe_path(&root, Path::new(relative))?.is_file() {
                versions.push(version);
            }
        }
        if versions.len() > 100000 {
            return Err("The NAS library exceeds 100,000 saved versions.".into());
        }
    }
    versions.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(versions)
}

pub fn restore(
    settings: &Settings,
    version: &Version,
    destination: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let root = check(settings)?;
    let source = safe_path(
        &root,
        Path::new(
            version
                .local_file
                .as_ref()
                .ok_or("This is not a NAS version.")?,
        ),
    )?;
    // Reserve the requested new filename; never replace an existing local file.
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        let mut input = fs::File::open(source).map_err(|e| e.to_string())?;
        let mut buffer = vec![0; 1024 * 1024];
        loop {
            cancelled(cancel)?;
            let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            output
                .write_all(&buffer[..count])
                .map_err(|e| e.to_string())?;
        }
        output.sync_all().map_err(|e| e.to_string())?;
        if digest(destination, cancel)? != (version.bytes, version.digest.clone()) {
            return Err("The restored NAS version did not match its saved checksum.".into());
        }
        Ok(())
    })();
    drop(output);
    if result.is_err() {
        let _ = fs::remove_file(destination);
    }
    result
}
