use super::*;
use std::collections::BTreeMap;

pub(super) const PACKAGE_LIMIT: u64 = 128 * 1024 * 1024;
const EXPANDED_LIMIT: u64 = 256 * 1024 * 1024;

pub(super) struct File {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    #[cfg_attr(not(unix), allow(dead_code))]
    pub executable: bool,
}
pub(super) struct Package {
    pub name: String,
    pub format: String,
    pub directory: bool,
    pub files: Vec<File>,
}

fn extension(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase()
}
fn native_format(path: &Path) -> bool {
    matches!(extension(path).as_str(), "clap" | "vst3" | "component")
}
fn safe_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path.components().all(|part| {
            let std::path::Component::Normal(part) = part else {
                return false;
            };
            let name = part.to_string_lossy();
            let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
            !name.contains([':', '\\', '\0'])
                && !name.ends_with(['.', ' '])
                && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                && !(stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && stem.as_bytes()[3].is_ascii_digit())
        })
}

pub(super) fn from_local(path: &Path, app: &AppInfo) -> Result<Vec<Package>, String> {
    if fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Choose a plugin file or bundle, not a symbolic link.".into());
    }
    if !path.is_dir() {
        return from_bytes(
            &path.file_name().unwrap_or_default().to_string_lossy(),
            read_limit(path, PACKAGE_LIMIT)?,
            app,
        );
    }
    if app.id != "soundcraft" || !native_format(path) {
        return Err("Choose a .clap, .vst3 or macOS .component bundle folder. Source folders cannot be installed.".into());
    }
    let mut files = Vec::new();
    let mut total = 0_u64;
    let mut visited = 0;
    for item in WalkDir::new(path).follow_links(false) {
        let item = item.map_err(|e| e.to_string())?;
        visited += 1;
        if visited > 8192 || item.depth() > 24 {
            return Err("Plugin bundle exceeds the file or folder depth limit.".into());
        }
        if item.file_type().is_symlink() {
            return Err(
                "Plugin bundles containing symbolic links require the publisher's installer."
                    .into(),
            );
        }
        if !item.file_type().is_file() {
            continue;
        }
        let bytes = read_limit(item.path(), PACKAGE_LIMIT)?;
        total += bytes.len() as u64;
        if total > EXPANDED_LIMIT || files.len() >= 4096 {
            return Err("Plugin bundle exceeds the size or file limit.".into());
        }
        let relative = item
            .path()
            .strip_prefix(path)
            .map_err(|e| e.to_string())?
            .to_owned();
        if !safe_path(&relative) {
            return Err("The bundle contains an unsupported file name.".into());
        }
        files.push(File {
            path: relative,
            bytes,
            executable: executable(item.path()),
        });
    }
    let package = Package {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        format: extension(path),
        directory: true,
        files,
    };
    validate_native(&package)?;
    Ok(vec![package])
}

pub(super) fn from_bytes(
    name: &str,
    bytes: Vec<u8>,
    app: &AppInfo,
) -> Result<Vec<Package>, String> {
    if bytes.starts_with(b"\0asm") {
        validate(&bytes, app)?;
        return Ok(vec![Package {
            name: name.into(),
            format: "wasm".into(),
            directory: false,
            files: vec![File {
                path: PathBuf::new(),
                bytes,
                executable: false,
            }],
        }]);
    }
    if native_format(Path::new(name)) {
        if app.id != "soundcraft" {
            return Err(
                "This is an audio plugin. Install it from SoundCraft's Plugins tab.".into(),
            );
        }
        let package = Package {
            name: name.into(),
            format: extension(Path::new(name)),
            directory: false,
            files: vec![File {
                path: PathBuf::new(),
                bytes,
                executable: true,
            }],
        };
        validate_native(&package)?;
        return Ok(vec![package]);
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| "Choose a compiled plugin or a ZIP package containing plugins. Source code and application installers cannot be installed here.")?;
    if zip.len() > 4096 {
        return Err("The archive contains too many files.".into());
    }
    let mut packages = BTreeMap::<PathBuf, Package>::new();
    let mut total = 0_u64;
    let mut seen = std::collections::HashSet::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
        let path = entry
            .enclosed_name()
            .ok_or("The archive contains an unsafe path.")?
            .to_owned();
        if !safe_path(&path) {
            return Err("The archive contains an unsafe or unsupported file name.".into());
        }
        if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("Archives with symbolic links require the publisher's installer.".into());
        }
        if entry.is_dir() {
            continue;
        }
        if !seen.insert(path.to_string_lossy().to_lowercase()) {
            return Err("The archive contains duplicate file names.".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Archive size overflow.")?;
        if total > EXPANDED_LIMIT || entry.size() > PACKAGE_LIMIT {
            return Err("The expanded archive is too large.".into());
        }
        let root = if app.id == "soundcraft" {
            let mut prefix = PathBuf::new();
            let mut found = None;
            for part in path.components() {
                prefix.push(part);
                if native_format(&prefix) {
                    found = Some(prefix.clone());
                    break;
                }
            }
            let Some(root) = found else {
                continue;
            };
            root
        } else if extension(&path) == "wasm" {
            path.clone()
        } else {
            continue;
        };
        let mut data = Vec::new();
        (&mut entry)
            .take(PACKAGE_LIMIT + 1)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        if data.len() as u64 > PACKAGE_LIMIT {
            return Err("A plugin exceeds the size limit.".into());
        }
        let relative = path
            .strip_prefix(&root)
            .map_err(|e| e.to_string())?
            .to_owned();
        let package = packages.entry(root.clone()).or_insert_with(|| Package {
            name: root
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            format: extension(&root),
            directory: root != path,
            files: Vec::new(),
        });
        if relative.components().count() > 24 {
            return Err("The bundle contains folders nested too deeply.".into());
        }
        package.files.push(File {
            path: relative,
            bytes: data,
            executable: entry.unix_mode().is_some_and(|m| m & 0o111 != 0),
        });
    }
    if packages.is_empty() {
        return Err(format!(
            "No supported {} plugins were found. Download a compiled plugin release, not the source-code ZIP.",
            app.name
        ));
    }
    if packages.len() > 32 {
        return Err("Install at most 32 plugins at a time.".into());
    }
    for package in packages.values() {
        if app.id == "soundcraft" {
            validate_native(package)?;
        } else {
            validate(&package.files[0].bytes, app)?;
        }
    }
    Ok(packages.into_values().collect())
}

fn validate_native(package: &Package) -> Result<(), String> {
    if !safe_path(Path::new(&package.name)) || Path::new(&package.name).components().count() != 1 {
        return Err("Invalid plugin name.".into());
    }
    if package.format == "component" && !cfg!(target_os = "macos") {
        return Err("Audio Units (.component) require macOS. Choose a CLAP or VST3 download for this computer.".into());
    }
    if package.files.is_empty() {
        return Err("The plugin bundle is empty.".into());
    }
    let folder = if cfg!(target_os = "macos") {
        "Contents/MacOS"
    } else if cfg!(windows) {
        if cfg!(target_arch = "aarch64") {
            "Contents/arm64-win"
        } else {
            "Contents/x86_64-win"
        }
    } else if cfg!(target_arch = "aarch64") {
        "Contents/aarch64-linux"
    } else {
        "Contents/x86_64-linux"
    };
    if package.directory && package.format == "clap" && !cfg!(target_os = "macos") {
        return Err(
            "CLAP bundles are for macOS. Choose a single .clap file for Windows or Linux.".into(),
        );
    }
    let candidates: Vec<_> = package
        .files
        .iter()
        .filter(|file| !package.directory || file.path.parent() == Some(Path::new(folder)))
        .collect();
    if candidates.is_empty() || !candidates.iter().any(|file| native_binary(&file.bytes)) {
        return Err(format!(
            "{} does not contain a compatible {} / {} plugin binary. Choose the download for this computer.",
            package.name,
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }
    if cfg!(target_os = "macos")
        && (!package.directory
            || !package
                .files
                .iter()
                .any(|f| f.path == Path::new("Contents/Info.plist")))
    {
        return Err(
            "macOS audio plugins must be complete bundles, including Contents/Info.plist.".into(),
        );
    }
    Ok(())
}

fn native_binary(b: &[u8]) -> bool {
    let u16le = |offset: usize| {
        b.get(offset..offset + 2)
            .map(|x| u16::from_le_bytes([x[0], x[1]]))
    };
    let u32le = |offset: usize| {
        b.get(offset..offset + 4)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()))
    };
    if cfg!(windows) {
        if !b.starts_with(b"MZ") {
            return false;
        }
        let Some(offset) = u32le(0x3c).map(|o| o as usize) else {
            return false;
        };
        return b.get(offset..offset + 4) == Some(b"PE\0\0")
            && u16le(offset + 4)
                == Some(if cfg!(target_arch = "aarch64") {
                    0xaa64
                } else {
                    0x8664
                })
            && u16le(offset + 22).is_some_and(|v| v & 0x2000 != 0);
    }
    if cfg!(target_os = "linux") {
        return b.starts_with(b"\x7fELF")
            && b.get(4..6) == Some(&[2, 1])
            && u16le(16) == Some(3)
            && u16le(18)
                == Some(if cfg!(target_arch = "aarch64") {
                    183
                } else {
                    62
                });
    }
    let cpu = if cfg!(target_arch = "aarch64") {
        0x0100000c
    } else {
        0x01000007
    };
    let thin = |slice: &[u8]| {
        let field = |o: usize| {
            slice
                .get(o..o + 4)
                .map(|x| u32::from_le_bytes(x.try_into().unwrap()))
        };
        slice.len() >= 32
            && slice.starts_with(&[0xcf, 0xfa, 0xed, 0xfe])
            && field(4) == Some(cpu)
            && matches!(field(12), Some(6 | 8))
            && field(20).is_some_and(|size| size as usize <= slice.len() - 32)
    };
    if thin(b) {
        return true;
    }
    let fat64 = b.starts_with(&[0xca, 0xfe, 0xba, 0xbf]);
    if fat64 || b.starts_with(&[0xca, 0xfe, 0xba, 0xbe]) {
        let be = |o: usize| {
            b.get(o..o + 4)
                .map(|x| u32::from_be_bytes(x.try_into().unwrap()))
        };
        return be(4).is_some_and(|count| {
            count <= 32
                && (0..count as usize).any(|i| {
                    let record = 8 + i * if fat64 { 32 } else { 20 };
                    if be(record) != Some(cpu) {
                        return false;
                    }
                    let number = |o: usize| {
                        if fat64 {
                            b.get(o..o + 8)
                                .map(|x| u64::from_be_bytes(x.try_into().unwrap()))
                        } else {
                            be(o).map(u64::from)
                        }
                    };
                    let Some((offset, size)) =
                        number(record + 8).zip(number(record + if fat64 { 16 } else { 12 }))
                    else {
                        return false;
                    };
                    let Some(end) = offset
                        .checked_add(size)
                        .filter(|end| *end <= b.len() as u64)
                    else {
                        return false;
                    };
                    b.get(offset as usize..end as usize).is_some_and(thin)
                })
        });
    }
    false
}

fn executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}
pub(super) fn write(package: &Package, dest: &Path) -> Result<(), String> {
    if package.directory {
        fs::create_dir(dest).map_err(|e| e.to_string())?;
    }
    for file in &package.files {
        let path = if package.directory {
            dest.join(&file.path)
        } else {
            dest.to_owned()
        };
        fs::create_dir_all(path.parent().ok_or("Invalid plugin location.")?)
            .map_err(|e| e.to_string())?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        output
            .write_all(&file.bytes)
            .and_then(|_| output.sync_all())
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if file.executable
                || file.path.starts_with("Contents/MacOS")
                || (!package.directory && package.format != "wasm")
            {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}
pub(super) fn digest(path: &Path) -> Result<String, String> {
    if fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err(
            "The managed plugin was replaced with a symbolic link; it was left untouched.".into(),
        );
    }
    if path.is_file() {
        return Ok(hash(&read_limit(path, PACKAGE_LIMIT)?));
    }
    let mut hasher = Sha256::new();
    let mut entries = Vec::new();
    let mut total = 0;
    let mut visited = 0;
    for item in WalkDir::new(path).follow_links(false) {
        let item = item.map_err(|e| e.to_string())?;
        visited += 1;
        if visited > 8192 || item.depth() > 24 {
            return Err(
                "The managed bundle exceeds the folder limit; it was left untouched.".into(),
            );
        }
        if item.file_type().is_symlink() {
            return Err(
                "A managed plugin now contains a symbolic link; it was left untouched.".into(),
            );
        }
        if item.file_type().is_file() {
            entries.push(item.into_path());
        }
        if entries.len() > 4096 {
            return Err("The managed bundle has too many files.".into());
        }
    }
    entries.sort();
    if entries.is_empty() {
        return Err("The plugin is missing or empty.".into());
    }
    for file in entries {
        let bytes = read_limit(&file, PACKAGE_LIMIT)?;
        total += bytes.len() as u64;
        if total > EXPANDED_LIMIT {
            return Err("The managed bundle exceeds the size limit.".into());
        }
        let name = file
            .strip_prefix(path)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
