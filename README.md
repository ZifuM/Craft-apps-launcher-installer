# ArtCraft Master Suite

A native Windows launcher for Storytold’s open-source creative and productivity apps. Install and manage the apps, browse projects from one place, and open each project in the app that supports it.

> This is an independent community project and is not affiliated with Adobe Inc. Product names and marks belong to their respective owners.

## Download

Download and run **[ArtCraftMasterSuite-Setup.exe](ArtCraftMasterSuite-Setup.exe)** to install the launcher for your Windows user. Setup adds Start menu and desktop shortcuts.

The launcher installs creative apps separately and keeps project files in their existing folders. A network connection is needed to find, install, and check for app releases.

## What it does

- Installs, updates, opens, and uninstalls the supported apps.
- Brings projects from selected folders into one searchable library with List, Grid, and Waterfall views.
- Filters projects by app, favorites, or the last seven days, with sorting by date, name, size, or app.
- Manages watched folders in a separate Folders tab; removing a watched folder leaves its files untouched.
- Offers right-click actions to reveal a project in File Explorer or copy its path.
- Shows saved PSD/PSB canvas previews (8/16-bit RGB or grayscale with raw, RLE, or ZIP compression), plus embedded PhotoCraft previews. Changed previews refresh with the project scan.
- Falls back to embedded thumbnails or the app logo for unsupported formats. Native VectorCraft JSON rendering and live unsaved previews are not supported.
- Creates an app-named subfolder for every tool in added project directories.
- Supports renaming and deleting project files, with confirmation before deletion.
- Checks for updates automatically and lets you check an app manually.

## Sidebar designs

The Modern sidebar groups your workspace and management tools, with a compact icon rail available. Prefer the previous layout? Select **Settings → Appearance → Sidebar design → Classic** to restore it immediately. This preference is saved across restarts.

## Your apps

Open **Your apps** in the sidebar to browse installed tools, grouped into Creative and Productivity apps. Select an app tile to access its existing launch, update, uninstall, and project controls. The collection updates when apps are installed or removed.

## Workspace preferences

Settings is organized into **Appearance**, **Updates**, **Projects**, **Windows**, and **About** tabs, with saved controls for:

- Automatic release checks, with an interval from 1 to 24 hours (default: 4 hours).
- Persistent update notifications, which can be turned off separately.
- Automatic project refresh, from 1 to 30 minutes (default: 3 minutes).
- Compact sidebar, reduced motion, and List / Grid / Waterfall project views.
- Project folders and the default working folder for apps.

Background checks run while the launcher is open. Creative and productivity app updates remain manual. Master Suite itself can update automatically, controlled separately in Settings → Updates. Manual checks and refresh remain available when background checks are off.

## Supported apps

**Creative Apps:** PhotoCraft · VectorCraft · FilmCraft · LightCraft · EffectCraft · DesignCraft · SoundCraft

**Productivity Apps:** PdfCraft · CADCraft · GridCraft · WordCraft · DeckCraft

SoundCraft is listed in the launcher, but does not yet have an official Windows release to install.

The launcher recognizes common project and media extensions for each app, including PSD/PSB, SVG, video-project interchange files, camera RAW, PDF, Lottie, and page-layout documents. See the [source README](ArtCraftLauncher-Source/README.md) for the complete format list, local data locations, and build instructions.

## Source code

The current source code and assets are in [`ArtCraftLauncher-Source`](ArtCraftLauncher-Source/).

The launcher source is licensed under the [MIT License](ArtCraftLauncher-Source/LICENSE). The license for the launcher does not change the rights for third-party app names, logos, or other assets.

## Verify downloads

`SHA256SUMS.txt` contains SHA-256 a checksum for the installer. On Windows, verify a file with:

```powershell
Get-FileHash .\ArtCraftMasterSuite-Setup.exe -Algorithm SHA256
```

Compare the resulting hash with the matching entry in `SHA256SUMS.txt`.

## Light mode and Windows startup

- **Settings → Appearance → Theme:** choose Dark or Light. The choice is saved.
- **Settings → Windows → Minimize to system tray:** optional, off by default. Minimizing hides the window; closing still exits.
- **Settings → Windows → Start with Windows in the tray:** optional, off by default. Starts quietly at sign-in without the splash screen.

Click the tray icon to reopen Master Suite, or right-click for Open and Quit. Opening its shortcut brings the existing instance forward. If the tray cannot be created, the window remains accessible. Uninstalling removes the Windows startup entry.


## Project preview support

Project cards read the last saved file, not unsaved changes in an editor. Hover over a preview to see its source. A folder icon on every project row and card reveals the file in Windows File Explorer.

| Files | Preview |
| --- | --- |
| PSD / PSB | Supported saved RGB/grayscale canvas, with embedded-thumbnail fallback |
| SVG | Rendered artwork with embedded assets; external image references are not loaded |
| PDF / PDF-compatible AI | First page rendered by Windows |
| PhotoCraft / VectorCraft / native bundles | Embedded composite, artboard preview, or thumbnail when present; empty VectorCraft artboards are recognized |
| DOCX / ODT / PPTX / XLSX | Embedded thumbnail when present; otherwise a simplified text or cell-content preview (not exact page layout) |
| TXT / Markdown / CSV / TSV | Simplified saved-content preview |
| WAV / AIFF / FLAC | Audio waveform, with bounded decoding for large recordings |
| Camera RAW | Embedded camera JPEG when readable; may not include later edits |
| DXF | Basic 2D lines, circles and straight polylines; complex entities use the Windows fallback |
| Other formats | Windows thumbnail handler when installed and available |

Film/VFX timelines, complex CAD drawings, older native projects without embedded previews, and unsupported documents may still show **Preview unavailable**. This does not mean the document is blank. Native scene/composition rendering for every app is not included. Preview loading runs in the background, and unchanged successful previews are reused during scans.


## Master Suite automatic updates

Starting with **2.2.0**, Master Suite checks the latest stable release from [ZifuM/Craft-apps-launcher-installer](https://github.com/ZifuM/Craft-apps-launcher-installer/releases) after startup and every four hours. Automatic suite updates are enabled by default and can be disabled in **Settings → Updates**.

A newer release is downloaded in the background, checked against its SHA-256 digest, and installed after a notification and a 10-second idle delay. App installations, launches, project scans and project-edit dialogs defer the restart. The installer waits for Master Suite to exit, replaces the application, and starts it again. Project files and preferences are retained. Failed downloads or checksum checks do not start the installer.

### Publishing an update

1. Set the package version in `ArtCraftLauncher-Source/Cargo.toml` to the new version, greater than the previous release.
2. Build the launcher first, then the setup executable (setup embeds the launcher).
3. Publish a non-draft, non-prerelease GitHub Release with a matching numeric tag, such as `v2.3.0`.
4. Attach **ArtCraftMasterSuite-Setup.exe** and **SHA256SUMS.txt**. The updater prefers the direct installer. It also accepts `Windows-64bit.Installer.zip` or `ArtCraftMasterSuite-Windows-x64.zip` containing exactly one `ArtCraftMasterSuite-Setup.exe`.

The updater uses GitHub's asset SHA-256 digest when available, otherwise a matching entry in the release's SHA256SUMS.txt. For ZIP releases, the fallback checksum must cover the ZIP. Unsigned/unverified downloads are not installed when neither checksum source is available. A SHA-256 digest checks integrity; it is not an Authenticode signature.

Updating files on the repository's main branch alone does not publish an app update. Users on an older build without the self-updater must install 2.2.0 or later once to receive future automatic updates.


## App screen designs

Build 2.2 introduces redesigned **App Manager** and **Your apps** screens with app-colored headers, aligned action rows, category filters and search. Your apps includes direct launch and workspace buttons. App Manager retains install, launch, update, uninstall and app-detail actions.

To restore the previous screens, enable **Settings → Appearance → Classic app screens**. This setting is independent of the sidebar design. The pre-redesign source snapshot is also preserved in `ArtCraftLauncher-Source/backups/app-screens-before-redesign/`.

Each app workspace includes a searchable Projects tab with List, Grid and Waterfall views. Asset management and Plugin management are clearly marked Coming soon; their features are not implemented yet. Classic app screens also restores the previous workspace layout.
