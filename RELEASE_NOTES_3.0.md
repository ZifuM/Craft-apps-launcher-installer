# ArtCraft Master Suite — Build 3.0 Beta

This development preview begins the Linux port while retaining the suite's existing design and shared codebase.

## Downloads

The following installable files are attached directly to this release:

- **Windows x64:** `ArtCraftMasterSuite-Setup.exe`
- **Linux x86-64:** `ArtCraftMasterSuite-Linux-x86_64.AppImage` or `ArtCraftMasterSuite-Linux-x86_64.flatpak`
- **Linux ARM64:** `ArtCraftMasterSuite-Linux-aarch64.AppImage` or `ArtCraftMasterSuite-Linux-aarch64.flatpak`
- **Checksums:** `SHA256SUMS.txt`

Choose one package for your operating system and architecture. Linux users do not need to compile the source. Flatpak requires Flatpak support and may download a runtime on first installation.

**Build status:** Windows x64, Linux x86-64 and Linux ARM64 packages built successfully on GitHub Actions. All five downloads were checked against the release SHA-256 checksums. Linux desktop installation and runtime behavior still need hands-on validation.

## Added in this preview

- Linux build targets for **x86-64** and **ARM64**, with separate AppImage and Flatpak packaging.
- Craft app installation selects an actual published package for the current operating system and CPU architecture, with SHA-256 checks. Unsupported targets do not fall back to Windows downloads.
- Linux application-data paths, executable permissions, app launching and file-manager actions.
- Linux tray and login-startup integration, with a visible-window fallback when the desktop has no tray support.
- Linux PDF preview support through Poppler, alongside shared PSD, embedded-thumbnail, document and audio previews.
- Platform-aware suite updates: matching AppImages for Linux, installed references for Flatpak, and Windows installers for Windows.
- A Linux build workflow and instructions for producing both package formats.

## Included suite features

- The existing Home, Your apps, Projects and App Manager design, themes and language options.
- Versioned project backups to provider-managed sync folders, with local copy status and restore-copy actions.
- Provider download/open controls, plus experimental notices for Cloud, Plugin management and Asset management.
- Experimental plugin management for compatible apps and organized workspace folders.

## Important limitations

- Native Windows and Linux compilation and package creation succeeded. Linux installation, updates and desktop integration still need hands-on validation.
- Craft apps are downloaded separately through App Manager; they are not bundled with the suite installer.
- Flatpak host integration allows launching applications and update commands outside the sandbox. The manifest avoids blanket home-directory access; project and backup folders are selected through the desktop portal.
- Flatpak automatic updates require a configured, maintained update repository. A standalone source archive or local Flatpak bundle cannot provide that service by itself.
- Google Drive and OneDrive have no official Linux desktop clients. Their backup destinations need a separately configured Linux sync tool. Dropbox has a Linux client.
- Windows shell-specific thumbnails have no exact Linux equivalent. Some previews depend on embedded content or additional tools.
- Cloud and plugin features remain experimental; asset management remains a placeholder. Universal save/export routing still requires upstream Craft app integration.
- macOS is deferred. The existing Windows download has not been repackaged as part of this Linux source release.

## Building and publishing

See `docs/RELEASING.md` for the GitHub release process and `docs/LINUX.md` for Linux requirements. Publish a release tagged `v3.0.0` from the updated source; the workflow builds and attaches the Windows installer, both Linux formats and their checksums. A manual run can also attach builds to an existing draft release.

The source application version is **3.0.0**. Previously built Windows 2.2 files in the repository are not relabeled; the workflow compiles a fresh Windows installer from the release tag.
