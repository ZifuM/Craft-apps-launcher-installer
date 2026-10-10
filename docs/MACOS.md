# macOS beta

Build 3.5 brings the current shared desktop interface and features to the universal macOS application:

`macOS-universal.dmg`

The build targets macOS 12 or later on Intel and Apple Silicon. Compilation and packaging are performed on a GitHub macOS runner; a successful build does not establish desktop compatibility on every supported OS version.

## Build status

Build `v3.5.0-beta.1` includes the Assets library, presets and app handoff, redesigned GitHub/local plugin installation, centered top search, direct Google Drive sync and Local NAS backups. The release workflow compiles both Intel and Apple Silicon targets, combines them into one application, applies an ad-hoc signature and creates the universal DMG. Architecture, signature integrity and disk-image checks are part of packaging. Native desktop behavior has not been manually validated for this build.

## Install

Open the DMG, drag **ArtCraft Master Suite.app** to Applications (or your user's `~/Applications` folder), eject the image, and launch the installed app. Complete onboarding to choose a project folder and language.

This beta uses an ad-hoc code signature. It is **not Developer ID signed or notarized**. Apple may require approval in Privacy & Security before opening it. Do not disable Gatekeeper. Public distribution without the unsigned-app prompt will require an Apple Developer membership, Developer ID Application certificate and notarization credentials.

## Mac integration

The compact screens, menus, text-size settings and translation catalog are shared with Windows/Linux. Search uses Command-F and the interface uses installed Mac fonts where available.

- Settings and caches: `~/Library/Application Support/ArtCraft Master Suite`.
- Managed Craft applications: `~/Applications/ArtCraft Apps/<app-id>/`.
- App Manager selects the upstream `macos-universal.dmg` asset and verifies its release checksum. The image is mounted read-only, its application architecture is checked, and the app bundle is copied before replacing an existing install.
- Project-folder actions use Finder. Browser links and provider clients use macOS `open`.
- Native previews use Quick Look where a shared format preview is unavailable. Availability depends on the installed Quick Look providers.
- The menu-bar icon supports opening and quitting Master Suite. Minimize-to-menu-bar and login startup remain off by default.
- Login startup uses a per-user LaunchAgent. Enable it only after moving the suite out of its disk image and into its installed location.
- Assets use `<project folder>/<App>/Assets`, with imported presets in `Assets/Presets`. Native file/folder dialogs, Finder reveal and file handoff are shared with the existing Mac app launcher. See [Assets](ASSETS.md).
- PhotoCraft, VectorCraft and EffectCraft plugin adapters use their upstream Mac Application Support folders. SoundCraft supports per-user CLAP, VST3 and Audio Unit bundles in `~/Library/Audio/Plug-Ins`; package checks require a compatible Intel/Apple Silicon binary and preserve bundle layouts and executable permissions. See [Plugins](PLUGINS.md). Plugin loading and asset handoff remain experimental.
- Google Drive uploads use browser sign-in and Keychain credential storage, and Local NAS backups use a share mounted in Finder. Both preserve the app's Projects/Assets/Exports/Plugins folders. The universal release build includes the configured Google Desktop OAuth settings for both CPU architectures. Dropbox and OneDrive are not available yet; see [Cloud setup](CLOUD_SETUP.md).

## Updates

Both Intel and Apple Silicon select `macOS-universal.dmg` for suite updates; the updater also accepts the historical universal DMG name. Older installed versions may need a one-time manual upgrade to recognize the new names. The downloader checks its SHA-256, then the Mac installer checks the bundle identifier, native architecture and code-signature integrity. The replacement waits for the suite to exit and retains the previous `.app` beside the installation in an `.ArtCraft-update-*` folder.

The installed location must be writable by the current user. Running from a mounted disk image or App Translocation is not eligible for automatic replacement; install in Applications or `~/Applications` first. No administrator prompt, quarantine removal or Gatekeeper bypass is performed. New preferences default to stable releases only. Enable **Allow prerelease updates** in Settings to receive published prereleases; existing preferences are retained. Draft releases are never offered.

## Build

Run **Actions → macOS package → Run workflow** for a Mac-only build candidate. The release workflow also builds the universal DMG and includes it with the Windows and Linux downloads.

On a Mac with Xcode command-line tools, Rust stable and Python 3.11 or newer:

```sh
bash packaging/macos/build-dmg.sh
```

This compiles both Apple targets, combines them with `lipo`, creates the `.app` metadata and icon, applies an ad-hoc signature, and creates a compressed DMG. It also checks both architectures, the signature and disk-image integrity. Output is under `dist/macos/`.

Desktop installation, tray behavior, updates and launching upstream Craft apps still require hands-on Mac validation.
