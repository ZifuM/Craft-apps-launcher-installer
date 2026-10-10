# Linux packages — Build 3.4 prerelease

The latest Windows interface, update discovery, settings and core source are shared with Linux and macOS. Linux packaging is under `packaging/linux/`. The release workflow builds standalone AppImage and Flatpak downloads for GitHub Releases. Source is available through GitHub; the obsolete separate source-bundle script has been removed.

## Current status

Build `v3.4.0-beta.3` uses the shared V2/Legacy themes, popup Settings, dedicated Cloud screen, window memory, project-folder moves and startup animations. Direct Google Drive sync and Local NAS backups preserve workspace subfolders. Native Ubuntu runners build x86_64 and ARM64 AppImage and Flatpak packages from the release tag; all four are included in the prerelease along with SHA-256 checksums. Native desktop behavior has not been manually validated for this build.

Desktop installation, updates, portals, tray support and Wayland/X11 behavior require hands-on Linux validation. A successful build alone does not establish compatibility.

## Package outputs

| File | Target |
|---|---|
| `Linux-x86_64.AppImage` | Intel/AMD 64-bit Linux |
| `Linux-aarch64.AppImage` | ARM64 Linux |
| `Linux-x86_64.flatpak` | Flatpak, Intel/AMD 64-bit |
| `Linux-aarch64.flatpak` | Flatpak, ARM64 |

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
- **AppImage:** exactly `Linux-<architecture>.AppImage`. SHA-256 and architecture are checked, and the replacement is staged beside the current writable AppImage. A previous copy remains for manual recovery.
- **Flatpak:** downloads the matching `Linux-<architecture>.flatpak` from the selected GitHub release, checks SHA-256, and installs that bundle when the update is accepted. The existing user/system scope is preserved. Custom branches and custom installations use their software manager.

**Settings → Apps → Include prerelease updates** (V2) applies to both package formats and to manual/automatic checks. Drafts are excluded. Fresh preferences default to stable-only; existing preferences are retained. The updater also accepts historical package names. Older versions may require manual installation once to recognize the new names.

Standalone GitHub bundles no longer require a hosted Flatpak update repository for the suite updater. The Flatpak runtime still comes from the configured runtime source. System-wide installation may require desktop authorization; a failure leaves an actionable error rather than silently changing scope.

## Flatpak permissions

The user explicitly approved host integration. The manifest requests network, graphics, display/IPC, the status-notifier tray service, `org.freedesktop.Flatpak` host integration and the specific `~/.config/autostart` directory. It does **not** grant the whole home directory. Choose project and backup directories through the desktop file portal.

Host integration permits commands outside the sandbox; it is used for launching managed Craft AppImages, provider clients, file-manager actions, process checks and Flatpak updates. Managed Craft apps inherit the suite's XDG configuration directories so plugin and export adapters agree with the apps. Such host-launched applications are not sandboxed by Master Suite. This is not a Flathub-reviewed package.

## Desktop integration and limitations

- Projects, preferences, previews and app installations use Linux paths. Windows data is not migrated automatically.
- Startup uses a per-user desktop entry. Tray support requires a StatusNotifierItem host; if unavailable, the suite remains visible. Some desktops need a tray extension.
- PDF first-page previews use `pdftoppm`; AppImage packaging bundles it. For Flatpak host previews, install the host's poppler-utils package. Existing embedded-image, PSD and content preview code is shared; Windows shell-only thumbnails still have no exact Linux equivalent.
- Google Drive uploads work without a provider desktop client, and Local NAS backups use a mounted network share selected through the folder picker. Both preserve the app's Projects/Assets/Exports/Plugins folders. Sign-in uses the browser and stores credentials in a running Secret Service, such as GNOME Keyring. The Flatpak manifest permits `org.freedesktop.secrets`; use the folder picker to grant access to a NAS mount. Release builds include the configured Google Desktop OAuth settings, including inside Flatpak. Dropbox and OneDrive are not available yet; see [Cloud setup](CLOUD_SETUP.md).
- Plugin/save/export integration remains experimental. Upstream limitations described in PLUGIN_MANAGEMENT.md still apply.
- The shared UI uses native Linux font candidates, with bundled egui fallbacks; CJK languages use installed Noto fonts where available.
