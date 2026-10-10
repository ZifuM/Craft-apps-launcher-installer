# Build 3.4 beta 3 — Prerelease

Version: `3.4.0-beta.3` · Tag: `v3.4.0-beta.3`

## Updated in beta 3

- **Separate storage pages:** the Storage sidebar now contains **Cloud** for Google Drive, Dropbox and OneDrive, and **Local NAS** for NAS connections only. Local NAS uses a database icon. Opening either page does not change the active backup destination; choose its backup option explicitly.

## Cloud and NAS update

- **Direct Google Drive sync:** connect through your browser and upload without Google Drive for desktop. Connected status is shown only after live Drive access is confirmed; failures display an error and disable syncing until the connection is restored.
- **Local NAS backups:** connect a network share already mounted through the operating system, choose it instead of Google Drive, and back up selected files with copy verification and saved versions. Google Drive and NAS keep separate histories.
- **Matching workspace folders:** files retain their app and Projects/Assets/Exports/Plugins folders, including nested directories. For example, `PhotoCraft/Projects/Client job/document.psd` and `PhotoCraft/Assets/Textures/paper.png`. The Cloud picker shows destinations and includes app and file-kind filters.
- **A dedicated Cloud screen:** opened from the top-right cloud icon, with its own sidebar, Your files, Selected files, Saved versions, Cloud and Local NAS. It replaces the previous Cloud tab in Settings.
- **A revised sync bar:** a flat theme-colored footer, thin divider, standard buttons and the selected backup destination. Sync notifications show progress, completion and a five-second dismissal countdown.
- **Window and navigation polish:** a taller Settings popup, removal of the sidebar Settings entry, vertically centered overflow buttons, and a Windows title bar that matches the selected light/dark toolbar color.
- **Native packages updated together:** Windows x64 installer, universal macOS DMG and Linux x86_64/ARM64 AppImage and Flatpak packages share the new cloud and NAS features. Google Desktop OAuth build configuration is included in each package; refresh tokens stay in the OS credential store.

Dropbox and OneDrive are **not available yet**. Google access remains subject to the configured Google project's testing/publishing status. Master Suite must remain running while syncing; NAS shares must remain connected and writable.

## New in this build

- **V2 is the default interface:** a redesigned app catalog, sidebar and workspaces with light and dark modes. Legacy remains available in Settings → Themes; changing UI themes requires a restart.
- **Settings opens as a popup:** a compact navigation rail and consistent controls, opened by the top-right settings cog. Cloud now has its own screen through the adjacent cloud icon.
- **Startup animations return:** suite and app launch animations follow light/dark mode in taller containers. The suite splash has no title bar, and the normal window title bar follows the selected color mode.
- **Window placement is remembered:** size, position and maximized state are restored where supported by the desktop environment.
- **Project folders can be moved:** available from Projects and Settings, with verified copies, recovery tracking and updated project references.
- **Project layout fixes:** grid previews keep a 16:9 shape and consistent size as the window grows; list names and column headings align correctly.
- **More consistent presentation:** matching corner radii, corrected banner icon corners and clipping, smaller scroll gutters, independent sidebar/content scrollbar visibility and subtle page transitions. Reduce motion disables the transitions.
- **Installed app shortcuts:** Workspaces shows only installed apps, including in the collapsed sidebar. Redundant navigation and the local-workspace footer are removed.
- **Repository cleanup:** removed unused duplicate views, source backups, obsolete root installers and old screenshots. The README now has one current screenshot and short installation steps for each OS.

## Downloads

| Platform | Packages |
| --- | --- |
| Windows x64 | `Windows-x86_64.exe` |
| macOS Intel + Apple Silicon | `macOS-universal.dmg` |
| Linux x86_64 | `Linux-x86_64.AppImage`, `Linux-x86_64.flatpak` |
| Linux ARM64 | `Linux-aarch64.AppImage`, `Linux-aarch64.flatpak` |

`SHA256SUMS.txt` accompanies all six packages. See the [installation guide](https://github.com/ZifuM/Craft-apps-launcher-installer/blob/v3.4.0-beta.3/README.md#install) and [Cloud/NAS setup](https://github.com/ZifuM/Craft-apps-launcher-installer/blob/v3.4.0-beta.3/docs/CLOUD_SETUP.md).

## Build and release status

All six packages are built from the release tag on native GitHub runners. The release workflow publishes the prerelease after every platform build succeeds and the packages and SHA-256 checksums are attached. It remains separate from the latest stable release. [Release build results](https://github.com/ZifuM/Craft-apps-launcher-installer/actions/workflows/release-packages.yml).

## Known limitations

- Desktop installation and behavior have not been manually validated for this build. Successful compilation and packaging do not establish compatibility on every desktop.
- Windows binaries are unsigned. The macOS app is ad-hoc signed, not Developer ID signed or notarized.
- Some window managers, including Wayland compositors, control window placement and may ignore a saved position.
- Cloud backups and plugin integration remain experimental. Signed-in Google Drive transfers and backup/restore on a real NAS share have not been exercised for this build. The app verifies file size and content checksums before reporting a completed backup.
- Existing files in workspace Assets folders can be selected for backup. The separate workspace asset manager remains a placeholder.
- Previous sync-folder backups are retained locally and are not automatically migrated or uploaded.
