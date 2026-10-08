use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=assets/artcraft-icon.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let Some(rc) = find_resource_compiler() else {
        println!("cargo:warning=Windows Resource Compiler (rc.exe) not found; executable icon resource was skipped");
        return;
    };
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set"));
    let icon = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir is set"))
        .join("assets")
        .join("artcraft-icon.ico");
    let resource = out.join("artcraft-icon.rc");
    let compiled = out.join("artcraft-icon.res");
    let icon_path = icon.to_string_lossy().replace('\\', "\\\\");
    fs::write(&resource, format!("IDI_ICON1 ICON \"{icon_path}\"\r\n"))
        .expect("write icon resource file");
    let status = Command::new(rc)
        .arg("/nologo")
        .arg(format!("/fo{}", compiled.display()))
        .arg(&resource)
        .status()
        .expect("run Windows Resource Compiler");
    if !status.success() {
        panic!("Windows Resource Compiler failed while embedding the ArtCraft icon");
    }

    println!("cargo:rustc-link-arg-bin=artcraft-launcher={}", compiled.display());
    println!("cargo:rustc-link-arg-bin=artcraft-setup={}", compiled.display());
}

fn find_resource_compiler() -> Option<PathBuf> {
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            let candidate = directory.join("rc.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    let kits = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut versions: Vec<_> = fs::read_dir(kits)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    versions
        .into_iter()
        .map(|version| version.join("x64").join("rc.exe"))
        .find(|candidate| candidate.is_file())
}
