# ArtCraft Master Suite — Build 3.2 prerelease

Version: **3.2.0-beta.1** · Tag: **v3.2.0-beta.1**

## Shared desktop experience

The latest Windows interface and features now share the same implementation on Linux and macOS:

- Redesigned Home, App Manager, Your apps, Projects, app workspaces, Cloud, plugin management and Settings.
- Compact cards, short descriptions, three-dot context menus, shortcut icons and Properties dialogs.
- Centered page headers and sidebar items, consistent buttons, shorter search fields and fewer dividers.
- Small, Medium and Large text sizes; updated translations across the shared screens.
- Cloud List, Grid and Waterfall views, service logos, backup dates and status indicators.
- Project renaming and file-location actions, refreshed saved-file previews and first-run setup.
- More resilient release discovery when GitHub's API quota is exhausted, plus a prerelease update switch.

Native paths, app packages, tray/startup integration and file-manager actions are retained. Mac search uses Command-F. Linux file reveal requests selection of the file, with a containing-folder fallback when the desktop does not support it.

## Downloads

| Platform | File |
| --- | --- |
| Windows x64 | `Windows-x86_64.exe` |
| Linux Intel/AMD | `Linux-x86_64.AppImage` or `Linux-x86_64.flatpak` |
| Linux ARM64 | `Linux-aarch64.AppImage` or `Linux-aarch64.flatpak` |
| macOS Intel and Apple Silicon | `macOS-universal.dmg` |

Release assets include `SHA256SUMS.txt`. No source archive or ZIP wrapper is required for installation.

## Updates and existing installations

Enable **Allow prerelease updates** in **Settings → Updates** to receive published prereleases. Fresh preferences default to stable releases; existing choices are preserved. Draft releases are never offered.

The 3.2 updater recognizes the new filenames and older release names. Older installed suite versions that only recognize the previous filenames may require a one-time manual installation of 3.2. Existing preferences and projects are retained.

Flatpak updates now download the checksum-verified GitHub bundle for the selected release channel and architecture. Installation preserves the current user/system scope; custom branches or custom installation locations must use their software manager.

## Platform notes

- Cloud backup dates and local-copy indicators work on all platforms. A completed copy does not prove an upload. Windows can report confirmed upload state when its provider exposes Cloud Files metadata; Linux and macOS report local-copy status and direct users to their sync client for upload confirmation.
- The Mac application targets macOS 12+ and remains ad-hoc signed, without Developer ID signing or notarization.
- The Linux packages retain the existing AppImage and Flatpak integration. File previews, tray support and provider clients depend on the desktop and installed software.
- Assets remain a placeholder; plugin integration remains experimental.

## Build status

Source and packaging are prepared locally. Native Linux/macOS packages must be built by the release workflow before this prerelease is published. Desktop installation and behavior still require hands-on validation on those systems.
