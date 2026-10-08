# Craft Launcher

A native Windows launcher (Rust + egui, no web view) for PhotoCraft, FilmCraft, DesignCraft,
LightCraft, EffectCraft, PrintCraft and VectorCraft.

## Get the installer (.exe)
The installer is a normal Windows setup wizard (`CraftLauncher-Setup.exe`, made with Inno Setup).
It is built automatically on GitHub - nothing to install on your PC:

1. Create a new repository at https://github.com/new (private is fine).
2. Click **uploading an existing file** and drag in everything from this folder
   (including the `.github` folder). Commit.
3. Open the **Actions** tab, pick **Build Windows installer**, press **Run workflow**
   (it also runs on every push). It takes ~5-10 minutes.
4. Open the repo's **Releases** page and download **CraftLauncher-Setup.exe**
   (or double-click **Get-Installer.bat**, which downloads and runs it for you - the repo must be public).

## How it works
* **Apps tab** - looks for `<AppName>.lnk` in
  `C:\ProgramData\Microsoft\Windows\Start Menu\Programs` (also in sub-folders and the per-user
  Start Menu). Launching opens the shortcut, same as double-clicking it.
* **Logos** - pulled automatically from each app's shortcut / exe (up to 256x256).
  To override one, put `<AppName>.png` or `<AppName>.webp` (e.g. https://getartcraft.com/images/apps/photocraft/icon.webp saved as `PhotoCraft.webp`) in `%APPDATA%\CraftLauncher\icons\`.
  If nothing can be loaded, a clean coloured badge is shown instead.
* **Projects tab** - scans each app's project folder (Settings tab) and lists everything in one
  place. "Open" starts the app's real .exe with the project path as an argument.
* Settings are saved in `%APPDATA%\CraftLauncher\config.json`.

## App Manager (install & update)
The **App Manager** tab installs and updates the seven apps from their official GitHub releases
(`github.com/storytold/<repo>` - PrintCraft's repo is `pdfcraft`):
* Checks the latest build of each app (set *Include pre-release builds* - they're all early alpha).
* **Install / Update / Update all** downloads the Windows x64 `.msi` (falling back to the portable
  `.zip`), verifies GitHub's SHA-256 for the file, then runs the installer. Windows will show its
  normal install/UAC prompt.
* Installed versions are read from each app's .exe; apps installed through the launcher also have
  their exact release tag remembered, so `-rc` builds update correctly.
* **More > Uninstall** opens Windows' Installed apps list. **More > Release notes** opens GitHub.
* GitHub allows 60 anonymous API requests/hour; the launcher makes 7 per check.

## Things you will probably need to tweak (Settings tab)
* **Project folder** per app - defaults to `Documents\<AppName>`.
* **File types** per app - blank means "show every file".
