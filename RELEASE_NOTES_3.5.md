# Build 3.5 — Prerelease

Version: `3.5.0-beta.1` · Tag: `v3.5.0-beta.1`

## New in Build 3.5

- **Assets library:** open Assets from the sidebar or an app workspace. Import images, video, audio/MIDI, fonts, models, documents, presets and other files. File/folder imports and drag and drop preserve originals, retain companion folders and avoid overwriting existing names.
- **Presets and reusable files:** each app has an `Assets` folder, with imported presets in `Assets/Presets`. Save/export instructions help keep PhotoCraft and other app presets together. Search, app/type filters, favorites, previews, Grid/List/Waterfall views, rename, saved copies and recoverable library removal are included.
- **Use assets in apps:** choose an installed app to open a supported file through its native file-open/import route, or copy the path for Import, Place, Insert or preset commands. The suite does not modify an already open document or apply presets through undocumented automation.
- **Redesigned Plugins screen:** install compiled plugins from public GitHub repository/release/file links or local files. Select a compatible package when several are available. Manage installed copies with enable, disable, reveal and remove actions.
- **App-specific plugin locations:** PhotoCraft, VectorCraft and EffectCraft WebAssembly plugins use their documented settings and loader folders. SoundCraft supports CLAP and VST3, plus Audio Units on macOS. The installer checks platform/CPU compatibility, preserves bundle layout and Unix executable permissions, and respects existing custom plugin folders.
- **Centered search:** the top search field stays centered in the window across pages and searches the active Assets, Plugins, Projects, Cloud or Apps screen.
- **Cloud and NAS:** separate Cloud and Local NAS connection pages, a database icon for NAS, direct Google Drive uploads and matching app/Projects/Assets folder structure are included from the latest 3.4 work. Dropbox and OneDrive remain unavailable.

## Windows, Linux and macOS

The same release source includes the new Assets and Plugins screens on all platforms. Linux uses native XDG paths, file portals and per-user audio plugin folders; macOS uses Finder, Application Support and Audio Plug-Ins folders. The universal Mac app includes Intel and Apple Silicon binaries. Flatpak includes the specific audio-plugin folder permissions needed by the new installer.

| Platform | Downloads |
| --- | --- |
| Windows x64 | `Windows-x86_64.exe` |
| Linux x86_64 | `Linux-x86_64.AppImage`, `Linux-x86_64.flatpak` |
| Linux ARM64 | `Linux-aarch64.AppImage`, `Linux-aarch64.flatpak` |
| macOS Intel + Apple Silicon | `macOS-universal.dmg` |

All six installable packages are built from this tag on native GitHub Actions runners. `SHA256SUMS.txt` records their SHA-256 hashes. Publication waits until every build and package upload succeeds. This is a prerelease and does not replace the latest stable release.

[Installation](https://github.com/ZifuM/Craft-apps-launcher-installer/blob/v3.5.0-beta.1/README.md#install) · [Assets guide](https://github.com/ZifuM/Craft-apps-launcher-installer/blob/v3.5.0-beta.1/docs/ASSETS.md) · [Plugin guide](https://github.com/ZifuM/Craft-apps-launcher-installer/blob/v3.5.0-beta.1/docs/PLUGINS.md) · [Cloud/NAS setup](https://github.com/ZifuM/Craft-apps-launcher-installer/blob/v3.5.0-beta.1/docs/CLOUD_SETUP.md)

## Known limitations

- Compilation and packaging do not establish desktop behavior on every OS. Native installation, real plugin loading and end-to-end asset handoff have not been manually exercised for this build.
- Windows binaries are unsigned. The Mac app is ad-hoc signed, not Developer ID signed or notarized.
- Plugin and preset support depends on the installed Craft app version. ZIP presets remain stored files; they are not automatically applied. PhotoCraft internal brush groups may require companion tip files.
- Google sign-in remains subject to the configured Google project's testing/publishing status. NAS shares must already be mounted and writable. Keep Master Suite running until backups complete.
- Moving or renaming a linked asset can require relinking it in the editor. Master Suite keeps removed library files in the library's `.Trash` folder for manual recovery.
