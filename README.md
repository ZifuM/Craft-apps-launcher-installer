# ArtCraft Master Suite

Your Craft apps, projects and backups in one desktop workspace.

**Build 3.4 beta 2 · `3.4.0-beta.2` · Windows, Linux and macOS**

[Releases](https://github.com/ZifuM/Craft-apps-launcher-installer/releases) · [Build 3.4 notes](RELEASE_NOTES_3.4.md) · [Build from source](ArtCraftLauncher-Source/README.md)

Build 3.4 beta 2 adds direct Google Drive sync, Local NAS backups, matching workspace folders and an updated Cloud interface. Get the six platform packages from the [prerelease downloads](https://github.com/ZifuM/Craft-apps-launcher-installer/releases/tag/v3.4.0-beta.2). Prereleases are optional and do not replace the latest stable release.

## Install

Download the matching file from a release's **Assets**:

| OS | Download | Install |
| --- | --- | --- |
| Windows x64 | `Windows-x86_64.exe` | Run the installer, then open **ArtCraft Master Suite** from Start. |
| macOS 12+ — Intel or Apple Silicon | `macOS-universal.dmg` | Open the DMG, drag the app into **Applications**, then launch it. |
| Linux Intel/AMD | `Linux-x86_64.AppImage` or `Linux-x86_64.flatpak` | Follow either option below. |
| Linux ARM64 | `Linux-aarch64.AppImage` or `Linux-aarch64.flatpak` | Follow either option below using the ARM64 filename. |

**AppImage:** allow execution in file Properties and open it, or run:

```sh
chmod +x Linux-x86_64.AppImage
./Linux-x86_64.AppImage
```

**Flatpak:** with Flatpak and Flathub set up, run:

```sh
flatpak install --user ./Linux-x86_64.flatpak
flatpak run io.github.ZifuM.ArtCraftMasterSuite
```

**macOS:** this prerelease is not notarized. If macOS blocks opening it, use **System Settings → Privacy & Security → Open Anyway** for the downloaded app.

On first launch, choose your language and project folder. Use **All Apps** to install Craft apps and **Projects → Add folder** to connect existing work. Apps download separately. **Settings → Apps → Include prerelease updates** enables future beta updates; drafts are excluded.

## Main features

- **App catalog:** install, update and launch 12 Craft apps.
- **Projects:** search, filters, previews and List/Grid/Waterfall views; move folders from Projects or Settings.
- **Workspaces:** shortcuts for installed apps and their project libraries.
- **Appearance:** V2 light and dark modes, with Legacy available after a restart.
- **Desktop comfort:** remembered window placement, themed startup animations and optional subtle transitions.
- **Cloud backups:** direct Google Drive uploads or backups to a connected Local NAS share, with matching app/project/asset folders, saved versions, progress and completion notifications. Access Cloud from the top-right icon. Dropbox and OneDrive are not available yet.

## Screenshot

![ArtCraft Master Suite — V2 light mode, Home and recent projects](docs/screenshots/home.png)

*Windows shown. The interface is shared across Windows, Linux and macOS.*

Cloud and plugin integration are experimental. Existing local assets can be selected for backup; the separate workspace asset manager remains a placeholder. Google sign-in is subject to the configured Google project's testing/publishing status, and NAS shares must already be connected through the operating system. See [Cloud setup](docs/CLOUD_SETUP.md) for connection instructions. Keep Master Suite running until syncing finishes.

[Linux details](docs/LINUX.md) · [macOS details](docs/MACOS.md) · [Cloud setup](docs/CLOUD_SETUP.md) · [License](ArtCraftLauncher-Source/LICENSE)

Independent community launcher for Storytold's ArtCraft apps. App names and artwork belong to their respective owners.
