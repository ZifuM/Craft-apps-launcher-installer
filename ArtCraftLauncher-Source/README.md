# ArtCraft Master Suite

A native Windows, Linux and macOS home for Storytold’s open-source creative and office apps. Install and update the apps, keep local projects together, and open each project with the app that supports it.

> ArtCraft Master Suite is an independent community project. It is not affiliated with Adobe Inc. Adobe and other product names and marks belong to their respective owners.

## Build 3.4 prerelease

Version `3.4.0-beta.1` shares V2 light/dark mode, popup Settings and Cloud, remembered window placement, project-folder moves and themed startup animations across all desktop targets. Legacy remains selectable; UI theme changes require a restart. See [Linux packages](../docs/LINUX.md), [macOS packages](../docs/MACOS.md) and [release instructions](../docs/RELEASING.md).

The historical `windows_ui`, `windows_releases`, `windows_suite_update` and `windows_backup` filenames now contain shared code. Native operations remain behind target-specific guards. Unused duplicate views and saved source backups have been removed; Git history retains them.

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
| SoundCraft | Audio workstation | `.wav`, `.aiff`, `.aif`, `.flac` |
| CADCraft | CAD and drafting | `.dxf`, `.dwg` |
| GridCraft | Spreadsheets | `.xlsx`, `.csv`, `.tsv` |
| WordCraft | Word processing | `.docx`, `.odt`, `.rtf`, `.html`, `.md`, `.txt` |
| DeckCraft | Presentations | `.deckcraft`, `.pptx` |

V2 lists installed and available apps with category filters. Legacy separates **Creative Apps** and **Productivity Apps**. It selects the available official package for the current operating system and architecture.

The launcher matches files by extension. A preview is shown when it can be extracted from the file; formats without readable preview data show an unavailable-preview fallback. Recognizing an Adobe-compatible file extension does not guarantee that every feature of that format is supported by the app.

## Install

See the [main installation guide](../README.md#install) for Windows, Linux and macOS package names and instructions. Windows setup installs for the current user and adds Start menu and desktop shortcuts.

The launcher downloads official platform-specific app releases from the [Storytold GitHub organization](https://github.com/storytold). A network connection is needed to discover releases, install apps, and check for updates. If a release check or download fails, the launcher reports the problem; installed apps and local projects remain on the device.

## Build from source

### Windows requirements

- Windows
- The stable Rust toolchain, including the MSVC target
- Visual Studio Build Tools with the C++ build tools for MSVC

From `ArtCraftLauncher-Source`, build the launcher first, then the setup program:

```powershell
cargo build --locked --release --bin artcraft-launcher
cargo build --locked --release --bin artcraft-setup
```

The setup executable is written to `target/release/artcraft-setup.exe`. It embeds the launcher executable, so rebuild the launcher before rebuilding setup.

### Linux and macOS

Use a native machine/runner matching the target platform. From the repository root, run `bash packaging/linux/build-appimage.sh`, `bash packaging/linux/build-flatpak.sh`, or `bash packaging/macos/build-dmg.sh`. Platform requirements and limitations are in [LINUX.md](../docs/LINUX.md) and [MACOS.md](../docs/MACOS.md).

The release workflow builds all six installers from the matching version tag; see [RELEASING.md](../docs/RELEASING.md).

## Local data

**Windows**

- Launcher: `%LOCALAPPDATA%\Programs\ArtCraft Launcher`
- Installed apps: `%LOCALAPPDATA%\Programs\ArtCraft Apps`
- Launcher preferences: `%LOCALAPPDATA%\ArtCraftLauncher`

**macOS:** suite data is in `~/Library/Application Support/ArtCraft Master Suite`; managed apps are in `~/Applications/ArtCraft Apps`.

**Linux:** suite data uses `$XDG_DATA_HOME/artcraft-master-suite` (default `~/.local/share/artcraft-master-suite`); managed apps are under its `apps` folder. Flatpak uses its sandbox data directory and portal-selected project folders.

Projects stay in their existing folders. Adding a watched folder creates app-named subfolders inside it and scans the folder and its subfolders. The configured default app folder becomes each app’s working directory when launched. This can help apps choose a save location, but apps that manage their own save preferences may use a different location.

Deleting a project removes the file from disk after confirmation. Uninstalling an app removes its launcher-managed installation; it does not remove project files.

## License

The launcher source is available under the [MIT License](LICENSE). ArtCraft app names, logos, and other third-party assets remain the property of their respective owners.
