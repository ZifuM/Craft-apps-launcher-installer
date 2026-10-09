# ArtCraft Master Suite — Build 3.3 prerelease

**Version:** `3.3.0-beta.1` · **Tag:** `v3.3.0-beta.1`

## What's new

- A shared redesign for Windows, Linux and macOS: rounded banners, app-colored cards, compact controls and more consistent spacing.
- Reworked app workspaces with summaries, live version details, official links and larger artwork with a gradual fade and lighter cropping.
- Project cards with existing previews, favorites, rename, Properties, backup indicators and file-manager actions. Workspace layout is remembered separately.
- Single-row project toolbars, with the workspace's **Project library** heading above its controls.
- **Workspaces** in both expanded and collapsed sidebars, linking directly to each app. **Your apps** is hidden for now; its implementation is retained.
- Text actions in app context menus and a refreshed Properties popup with version, location and metadata details.
- Smaller orbiting Home icons, balanced App Manager spacing and a two-column app credits grid.
- Updated README with simple installation instructions and five new screenshots.

## Downloads

| Platform | Package |
| --- | --- |
| Windows x64 | `Windows-x86_64.exe` |
| Linux Intel/AMD | `Linux-x86_64.AppImage` or `Linux-x86_64.flatpak` |
| Linux ARM64 | `Linux-aarch64.AppImage` or `Linux-aarch64.flatpak` |
| macOS Intel / Apple Silicon | `macOS-universal.dmg` |

See the [installation guide](https://github.com/ZifuM/Craft-apps-launcher-installer/blob/v3.3.0-beta.1/README.md#install). `SHA256SUMS.txt` accompanies the six packages.

## Upgrade notes

Enable **Settings → Updates → Allow prerelease updates** to receive this build. Older installations that do not recognize the current package names may need a manual upgrade. Existing projects and preferences are retained.

The refreshed screens, translations and appearance settings use the same source on all platforms. Native package selection, file-manager actions, tray/startup integration and update handling remain platform-specific. macOS search uses Command-F.

## Known limits

- Cloud backup and plugin integration remain experimental; Assets is not implemented yet. Local backup completion does not prove a cloud upload. Linux/macOS rely on the provider's sync client for upload confirmation.
- Windows is unsigned. The macOS 12+ app is ad-hoc signed and not notarized.
- Small windows can scroll the single-row toolbars horizontally. “Your apps” remains hidden in this build.
- Packages are built on native GitHub runners. Hands-on Linux/macOS installation and desktop behavior have not been validated for this build.

