# ArtCraft Master Suite — Build 3.1 Beta

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
