# ArtCraft Launcher

A native Windows desktop launcher for the seven Storytold Crafting Apps. It uses egui/eframe, installs official 64-bit portable releases from GitHub, and scans only project folders selected by the user.

## Build

Install the stable Rust toolchain and Visual Studio Build Tools for the MSVC target. From this directory:

1. `cargo build --release --bin artcraft-launcher`
2. `cargo build --release --bin artcraft-setup`

Build the launcher first; the setup executable embeds it. The resulting installer is `target/release/artcraft-setup.exe`.

## Release and project formats

Release versions are resolved from GitHub's public release page, with its Atom feed as a fallback; release metadata does not depend on the rate-limited GitHub API. Windows packages and `SHA256SUMS.txt` are fetched from the release and verified before extraction. App logos are loaded from the matching ArtCraft website app pages. The supplied ArtCraft SVG mark is embedded in the launcher UI, window/taskbar icon, installer, Start menu shortcut, and Windows Installed apps entry. Network failures leave the launcher usable and show a clear release-check or download error.

The local project index recognizes `.pcraft`, `.psd`, `.psb`; `.vectorcraft`, `.svg`, `.eps`, `.ai`; `.fcproj`, `.otio`, `.edl`, `.aaf`; camera RAW files; `.pdf`; `.ecproj`, `.lottie`; and `.designcraft`, `.dcbook`, `.idml`. Projects in the launcher can be renamed (their extension stays intact) or deleted with confirmation; common project actions use compact icons with tooltips. The Projects page offers saved List, Grid, and Waterfall views. Cards show file type, size, modified date, and folder; image formats (including TIFF), PSD/PSB embedded composites, and compatible PhotoCraft bundle previews are shown when available. Other formats use the app logo when an actual canvas preview is unavailable. The repository for website-branded PdfCraft and its executable are named `printcraft`. When a project folder is added, the launcher creates an app-named subfolder for every app and automatically scans the chosen folder and all its subfolders. The selected default project root is the working directory for each app, using that app's subfolder. This is a best-effort default for Save dialogs; apps that remember their own location can override it.

## Notes

The launcher is installed per user to `%LOCALAPPDATA%\Programs\ArtCraft Launcher`. Setup installs the supplied ArtCraft `.ico` next to the executable, points the Start menu and desktop shortcuts directly at it, and embeds the mark in the executable for the running window and taskbar. App releases are installed separately to `%LOCALAPPDATA%\Programs\ArtCraft Apps`. Launcher preferences are stored in `%LOCALAPPDATA%\ArtCraftLauncher`; project files remain in their existing folders. This independent launcher is not affiliated with Adobe Inc. Product and company marks belong to their respective owners.

Each app in the sidebar opens its own detail page with install/update/open/remove controls, recent matching projects, supported file types, and links to the app source and website. Installed app cards in the All Apps page also include a red uninstall control with a confirmation step. App cards and detail pages use a subtle version of each app logo's background color. Launching an app shows a short branded splash before opening it. Navigation icons are drawn by the UI so they do not depend on symbol-font coverage.

While the launcher is open, it checks app releases at startup and every four hours. A toast summarizes installed apps with available updates. Watched project folders are rescanned every three minutes.
