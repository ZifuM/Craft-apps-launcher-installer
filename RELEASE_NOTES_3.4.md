# Build 3.4 — Prerelease

Version: `3.4.0-beta.1` · Tag: `v3.4.0-beta.1`

## New in this build

- **V2 is the default interface:** a redesigned app catalog, sidebar and workspaces with light and dark modes. Legacy remains available in Settings → Themes; changing UI themes requires a restart.
- **Settings opens as a popup:** a compact navigation rail and consistent controls, with Cloud backups integrated into their own tab. The top-right Cloud shortcut opens that tab directly.
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

`SHA256SUMS.txt` accompanies all six packages. See the [installation guide](https://github.com/ZifuM/Craft-apps-launcher-installer/blob/v3.4.0-beta.1/README.md#install).

## Build and release status

This release is a **draft prerelease** for review. The release workflow builds Windows x64, native Linux x86_64/ARM64 packages and the universal macOS app from the same tag, then attaches the packages and checksums without publishing the draft.

## Known limitations

- Desktop installation and behavior have not been manually validated for this build. Successful compilation and packaging do not establish compatibility on every desktop.
- Windows binaries are unsigned. The macOS app is ad-hoc signed, not Developer ID signed or notarized.
- Some window managers, including Wayland compositors, control window placement and may ignore a saved position.
- Cloud backups and plugin integration remain experimental. Provider software performs cloud uploads; the suite reports local copies. Assets remains a placeholder.
