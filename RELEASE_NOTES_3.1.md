# ArtCraft Master Suite — Build 3.1 Beta

## Update fixes

- Select the newest published suite release containing a binary for the current operating system and architecture.
- Add **Include beta suite updates**, enabled by default for this beta, so Linux and Mac builds are not overlooked in favor of the older Windows-only stable release.
- Correct the Mac architecture check used by both DMG packaging and Craft app installation.
- Audit all 12 Craft apps: all 78 checked desktop/download links were readable; no missing upstream release file was found.

## Build status

All six installer packages built successfully on GitHub. Downloaded package checksums and Windows/Linux/DMG headers were verified. The Mac runner also verified both CPU architectures, the app's ad-hoc signature and DMG integrity. Native desktop installation and behavior still need hands-on validation.

Existing Build 3.0 users may need to download this update manually once, because the older updater ignores beta releases.

## macOS support

- Universal `.dmg` installer for Intel and Apple Silicon Macs, targeting macOS 12 and later.
- The existing Master Suite interface, themes, language options and project tools.
- Craft app installation selects official Mac disk images instead of Windows or Linux packages.
- Finder actions, browser links, Mac cloud-client launching and Quick Look previews.
- Optional menu-bar controls, minimize-to-menu-bar and login startup.
- Mac-specific settings, application folders and supported plugin preference paths.
- Suite updates select the universal Mac DMG, verify the download, and retain the previous application during replacement.

## Downloads

- Windows x64: `ArtCraftMasterSuite-Setup.exe`
- Linux x86-64 and ARM64: matching `.AppImage` or `.flatpak` files
- macOS Intel and Apple Silicon: `ArtCraftMasterSuite-macOS-universal.dmg`
- Integrity checks: `SHA256SUMS.txt`

## Beta notes

The Mac app is ad-hoc signed, **not Apple Developer ID signed or notarized**. macOS may require explicit approval to open it. Move the app to Applications before using login startup or automatic updates.

Native Mac desktop behavior has not been manually validated. Cloud and plugin management remain experimental; asset management remains a placeholder. Craft app compatibility also depends on upstream packages. The new Include beta suite updates setting is enabled by default for this beta build; turn it off for stable releases only. Updates select the newest compatible release, avoiding older Windows-only releases on Linux and Mac.
