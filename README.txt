# ArtCraft Launcher

A native Windows home for Storytold’s open-source creative apps. Install and update the apps, keep local projects together, and open each project with the app that supports it.

> ArtCraft Launcher is an independent community project. It is not affiliated with Adobe Inc. Adobe and other product names and marks belong to their respective owners.

## Features

- **Install and manage apps:** Find releases, install or update apps, open them, or uninstall them from one place.
- **Browse projects together:** Scan folders you choose and filter projects by app. Switch between List, Grid, and Waterfall views.
- **See project details:** View file type, size, modified date, location, and a canvas preview when the file contains preview data the launcher can read.
- **Keep app folders organized:** Add a project folder and the launcher creates a subfolder for each app. The selected default folder is also used as the app’s working directory when launching, where supported.
- **Find work quickly:** Rename projects while keeping their file extension, open them in their associated app, or delete them with confirmation.
- **Get update notices:** The launcher checks for app updates when it starts and every four hours while open. You can also check an app manually. Project folders are rescanned every three minutes.
- **Use a local desktop app:** The launcher and project index run on your PC. It only scans folders you choose.

## Apps

| App | Purpose | Recognized project and media formats |
| --- | --- | --- |
| PhotoCraft | Image editing | `.pcraft`, `.psd`, `.psb`, `.png`, `.jpg`, `.jpeg`, `.tif`, `.tiff`, `.webp` |
| VectorCraft | Vector illustration | `.vectorcraft`, `.svg`, `.eps`, `.ai` |
| FilmCraft | Video editing | `.fcproj`, `.otio`, `.edl`, `.aaf` |
| LightCraft | Photo library and RAW development | `.dng`, `.cr2`, `.cr3`, `.nef`, `.arw`, `.raf`, `.orf`, `.rw2`, `.pef` |
| PdfCraft | PDF workbench | `.pdf` |
| EffectCraft | Motion and visual effects | `.ecproj`, `.lottie` |
| DesignCraft | Page layout | `.designcraft`, `.dcbook`, `.idml` |

The launcher matches files by extension. A preview is shown when it can be extracted from the file; formats without readable preview data use the app’s logo instead. Recognizing an Adobe-compatible file extension does not guarantee that every feature of that format is supported by the app.

## Install

Download the Windows setup executable from the project’s **Releases** page and run it. Setup installs the launcher for the current Windows user and adds Start menu and desktop shortcuts.

The launcher downloads official Windows app releases from the [Storytold GitHub organization](https://github.com/storytold). A network connection is needed to discover releases, install apps, and check for updates. If a release check or download fails, the launcher reports the problem; installed apps and local projects remain on the device.

## Build from source

### Requirements

- Windows
- The stable Rust toolchain, including the MSVC target
- Visual Studio Build Tools with the C++ build tools for MSVC

From the repository directory, build the launcher first, then the setup program:

```powershell
cargo build --release --bin artcraft-launcher
cargo build --release --bin artcraft-setup
```

The setup executable is written to `target/release/artcraft-setup.exe`. It embeds the launcher executable, so rebuild the launcher before rebuilding setup.

## Local data

- Launcher: `%LOCALAPPDATA%\Programs\ArtCraft Launcher`
- Installed apps: `%LOCALAPPDATA%\Programs\ArtCraft Apps`
- Launcher preferences: `%LOCALAPPDATA%\ArtCraftLauncher`

Projects stay in their existing folders. Adding a watched folder creates app-named subfolders inside it and scans the folder and its subfolders. The configured default app folder becomes each app’s working directory when launched. This can help apps choose a save location, but apps that manage their own save preferences may use a different location.

Deleting a project removes the file from disk after confirmation. Uninstalling an app removes its launcher-managed installation; it does not remove project files.

## License

The launcher source is available under the [MIT License](LICENSE). ArtCraft app names, logos, and other third-party assets remain the property of their respective owners.
