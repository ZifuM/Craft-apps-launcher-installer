# Linux port (development)

The UI and core source are shared with Windows. Linux packaging is separate under `packaging/linux/`; macOS work is deferred. The release workflow builds standalone AppImage and Flatpak downloads for GitHub Releases. The optional source archive is for developers, not installation.

## Source bundle

Download **ArtCraftMasterSuite-3.0-Linux-Source.tar.gz**, extract it, then read **START_HERE.md** in the extracted folder. The bundle preserves the repository layout so both build scripts can run from its root. It excludes Git history, Windows installers, previous UI backups, build caches and existing release archives. It includes source licenses and checked-in dependencies; other dependencies still require an internet connection when building.

```sh
tar -xzf ArtCraftMasterSuite-3.0-Linux-Source.tar.gz
cd ArtCraftMasterSuite-3.0-Linux-Source
```

To recreate this archive from the repository, run `python3 packaging/linux/package-source.py`. The script also updates the archive entry in the repository's `SHA256SUMS.txt`.

## Current status

- Build 3.0 Windows x64, Linux x86-64 and Linux ARM64 compilation and packaging succeeded on GitHub Actions.
- Both AppImage and Flatpak files are attached to the Build 3.0 draft release, with verified SHA-256 checksums. Desktop behavior and installation still need hands-on Linux validation.
- The `Linux packages` workflow builds x86-64 and ARM64 packages on native Linux runners. The `Release downloads` workflow calls it, builds the Windows installer, and attaches all five installable files plus checksums to the selected GitHub release after all builds succeed. See [RELEASING.md](RELEASING.md).
- The workflow output is a build candidate, not a claim of completed desktop validation. Installation, updates, file portals, tray support, Wayland/X11 and app launching still need Linux validation.

## Package outputs

| File | Target |
|---|---|
| `ArtCraftMasterSuite-Linux-x86_64.AppImage` | Intel/AMD 64-bit Linux |
| `ArtCraftMasterSuite-Linux-aarch64.AppImage` | ARM64 Linux |
| `ArtCraftMasterSuite-Linux-x86_64.flatpak` | Flatpak, Intel/AMD 64-bit |
| `ArtCraftMasterSuite-Linux-aarch64.flatpak` | Flatpak, ARM64 |

Outputs go to `dist/linux/`, which is ignored by Git. Internal Actions artifacts may use ZIP containers, but the release workflow extracts the packages and uploads individual `.AppImage`, `.flatpak` and `.exe` files to release Assets. Users do not need to download a ZIP or source archive.

## Local builds

Use Linux matching the desired architecture. The AppImage workflow uses Ubuntu 24.04, so do not advertise older distributions as supported without compatibility validation. Install Rust stable, a C compiler, pkg-config, OpenSSL development headers, X11/Wayland/OpenGL development packages, Python 3, desktop-file-utils, patchelf, zsync, poppler-utils and the dependencies required by linuxdeploy.

```sh
bash packaging/linux/build-appimage.sh
```

The script obtains linuxdeploy from its official GitHub release and checks its published SHA-256 digest. `LINUXDEPLOY=/absolute/path/to/linuxdeploy.AppImage` allows a separately verified local copy.

For Flatpak, install flatpak, flatpak-builder and the Flathub remote. Install `org.freedesktop.Platform//24.08`, `org.freedesktop.Sdk//24.08` and `org.freedesktop.Sdk.Extension.rust-stable//24.08`, then run:

```sh
bash packaging/linux/build-flatpak.sh
```

Dependencies are vendored into a temporary source copy before the sandboxed, offline Cargo build. The existing Windows installer remains a separate build target.

## Craft app installation

All 12 upstream Craft repositories were checked on 2026-10-09 and publish Linux x86-64 and ARM64 AppImages. The manager reads actual GitHub release assets and selects an exact filename for its operating system and architecture. Windows uses the corresponding portable ZIP. Linux never falls back to a Windows package, CLI package, web ZIP or another CPU architecture.

Each download is checked against the release's `SHA256SUMS.txt`. Linux also checks the ELF architecture and AppImage header, makes the file executable and installs it in the user's app-data directory. Launch uses extraction-and-run to avoid requiring FUSE. Existing installations are preserved if download, checksum or staging fails.

## Suite updates

Manual and automatic updates share the same selection logic:

- **Windows x64:** Windows suite installer only.
- **AppImage:** exactly `ArtCraftMasterSuite-Linux-<architecture>.AppImage`. SHA-256 and architecture are checked, and the replacement is staged beside the current writable AppImage. A previous copy remains for manual recovery.
- **Flatpak:** identifies the running installation from /.flatpak-info and uses its exact ref, architecture, branch and configured origin. It downloads without deploying, then applies the staged update when the user chooses Install and restart, or when automatic restart becomes eligible. It does not install an AppImage or EXE over Flatpak.

Publish exact AppImage filenames and one combined `SHA256SUMS.txt` with future GitHub releases. Flatpak automatic updates require a configured, maintained Flatpak repository; a standalone local bundle without a reachable update origin cannot auto-update. The packaging script produces a repository under `dist/linux/flatpak-repo`; public signing, hosting and remote configuration are separate release steps. System-wide Flatpak updates may require desktop authorization.

## Flatpak permissions

The user explicitly approved host integration. The manifest requests network, graphics, display/IPC, the status-notifier tray service, `org.freedesktop.Flatpak` host integration and the specific `~/.config/autostart` directory. It does **not** grant the whole home directory. Choose project and backup directories through the desktop file portal.

Host integration permits commands outside the sandbox; it is used for launching managed Craft AppImages, provider clients, file-manager actions, process checks and Flatpak updates. Managed Craft apps inherit the suite's XDG configuration directories so plugin and export adapters agree with the apps. Such host-launched applications are not sandboxed by Master Suite. This is not a Flathub-reviewed package.

## Desktop integration and limitations

- Projects, preferences, previews and app installations use Linux paths. Windows data is not migrated automatically.
- Startup uses a per-user desktop entry. Tray support requires a StatusNotifierItem host; if unavailable, the suite remains visible. Some desktops need a tray extension.
- PDF first-page previews use `pdftoppm`; AppImage packaging bundles it. For Flatpak host previews, install the host's poppler-utils package. Existing embedded-image, PSD and content preview code is shared; Windows shell-only thumbnails still have no exact Linux equivalent.
- Dropbox has a Linux client. Google Drive and OneDrive do not have official Linux desktop clients; a separately configured sync tool can supply a folder for versioned backups. The suite reports copies to that folder, not verified cloud uploads.
- Plugin/save/export integration remains experimental. Upstream limitations described in PLUGIN_MANAGEMENT.md still apply.
- macOS builds and DMG packaging are not part of this phase.
