# ArtCraft Master Suite

**Your creative and productivity apps, projects, and updates in one Windows workspace.**

ArtCraft Master Suite is a native desktop install manager for Storytold’s ArtCraft apps. Discover tools, launch your installed apps, and pick up your saved projects from one place.

**[Download the installer](ArtCraftMasterSuite-Setup.exe)** · **[Build 2.2 release notes](RELEASE_NOTES_2.2.md)** · **[Build from source](ArtCraftLauncher-Source/README.md)**

> An independent community project, not affiliated with Adobe Inc. Third-party app names, logos, and other assets remain subject to their respective rights.

## Get started

1. Download and run **ArtCraftMasterSuite-Setup.exe**. Setup installs for your Windows user and adds Start menu and desktop shortcuts.
2. Open **App Manager** to install the tools you want.
3. Add a folder in **Projects** to bring your saved work into the library.
4. Use **Your apps** to launch an app or enter its workspace.

Apps are installed separately. Finding releases and downloading apps requires an internet connection. Project files stay in their folders; removing a watched folder only stops indexing it.

## Explore the suite

### Home

Start with an overview of your installed apps, indexed projects, and available app updates. App-colored quick launch cards keep your tools close, with recent projects further down the page.

![ArtCraft Master Suite Home with workspace totals and app-colored quick launch cards](docs/screenshots/home.png)

### Your apps

Browse only the tools installed on your PC. Search your collection, filter by Creative or Productivity, and choose **Open app** to launch directly or **Workspace** to manage that app’s projects. The collection refreshes as apps are installed or removed.

![Your apps showing installed app cards, version information, and Open app and Workspace buttons](docs/screenshots/your-apps.png)

Each app workspace has three tabs:

- **Projects:** search saved work, browse previews, and switch between List, Grid, and Waterfall views.
- **Asset management:** a Coming soon layout; asset features are not implemented yet.
- **Plugin management:** a Coming soon layout; plugin features are not implemented yet.

### Projects

Keep work from your connected folders in one searchable library. Filter by app, favorites, or recent work; sort your files; and choose List, Grid, or Waterfall view. Supported files show previews of their saved contents.

Open projects in their associated app, mark favorites, reveal files in File Explorer, rename them, or delete them with confirmation. The **Folders** tab manages connected locations and the default project folder.

![Projects library in compact List view with filters, file previews, and project actions](docs/screenshots/projects.png)

### App Manager

Discover Creative and Productivity apps in separate categories. Search by name or purpose, filter installed apps or available updates, and manage installation, launch, update, and removal from app-colored cards. Up-arrow controls provide update actions.

![App Manager showing Creative and Productivity categories, search, filters, and app management cards](docs/screenshots/app-manager.png)

Screenshots show an example local setup. Installed apps, project counts, and app versions vary by device.

## Supported apps

| Creative apps | Productivity apps |
| --- | --- |
| PhotoCraft — image editing | PdfCraft — PDF workbench |
| VectorCraft — vector illustration | CADCraft — CAD and drafting |
| FilmCraft — video editing | GridCraft — spreadsheets |
| LightCraft — photo library | WordCraft — word processing |
| EffectCraft — motion and VFX | DeckCraft — presentations |
| DesignCraft — page layout | |
| SoundCraft — audio | |

Build 2.2 lists SoundCraft as Coming soon, with no installable Windows release configured. App availability depends on upstream releases.

See the [source README](ArtCraftLauncher-Source/README.md) for recognized file formats, local data locations, and build instructions.

## Make it yours

Settings are grouped into **Appearance**, **Updates**, **Projects**, **Windows**, and **About**.

- Choose **Dark** or **Light** mode, compact navigation, and reduced motion.
- Switch between **Modern** and **Classic** sidebar designs.
- Enable **Classic app screens** to restore the earlier App Manager, Your apps, and app workspace layouts. The [source backup](ArtCraftLauncher-Source/backups/app-screens-before-redesign/) is retained too.
- Set your preferred project view, connected folders, and default working folder.
- Configure app release checks from 1–24 hours and project refresh from 1–30 minutes; defaults are 4 hours and 3 minutes.
- Control persistent update notifications and automatic Master Suite updates.
- Optionally **minimize to the system tray** or **start with Windows in the tray**. Both are off by default.

Click the tray icon to reopen Master Suite, or right-click for Open and Quit. Closing the window exits; the minimize-to-tray preference applies to minimizing. Windows startup opens quietly without the splash animation.

Background checks run while Master Suite is open. Creative and productivity app updates are installed manually; Master Suite’s own automatic updates are controlled separately in **Settings → Updates**.

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

## Verify the installer

Compare the installer’s SHA-256 hash with the entry in [SHA256SUMS.txt](SHA256SUMS.txt):

```powershell
Get-FileHash .\ArtCraftMasterSuite-Setup.exe -Algorithm SHA256
```

## Source and license

Source code and assets are in [ArtCraftLauncher-Source](ArtCraftLauncher-Source/). The launcher source is licensed under the [MIT License](ArtCraftLauncher-Source/LICENSE). This license does not change the rights for third-party app names, logos, or other assets.
