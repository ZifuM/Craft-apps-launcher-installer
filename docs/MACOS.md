# macOS beta

Build 3.1 adds a universal macOS application and a drag-to-Applications disk image:

`ArtCraftMasterSuite-macOS-universal.dmg`

The build targets macOS 12 or later on Intel and Apple Silicon. Compilation and packaging are performed on a GitHub macOS runner; a successful build does not establish desktop compatibility on every supported OS version.

## Install

Open the DMG, drag **ArtCraft Master Suite.app** to Applications (or your user's `~/Applications` folder), eject the image, and launch the installed app. Complete onboarding to choose a project folder and language.

This beta uses an ad-hoc code signature. It is **not Developer ID signed or notarized**. Apple may require approval in Privacy & Security before opening it. Do not disable Gatekeeper. Public distribution without the unsigned-app prompt will require an Apple Developer membership, Developer ID Application certificate and notarization credentials.

## Mac integration

- Settings and caches: `~/Library/Application Support/ArtCraft Master Suite`.
- Managed Craft applications: `~/Applications/ArtCraft Apps/<app-id>/`.
- App Manager selects the upstream `macos-universal.dmg` asset and verifies its release checksum. The image is mounted read-only, its application architecture is checked, and the app bundle is copied before replacing an existing install.
- Project-folder actions use Finder. Browser links and provider clients use macOS `open`.
- Native previews use Quick Look where a shared format preview is unavailable. Availability depends on the installed Quick Look providers.
- The menu-bar icon supports opening and quitting Master Suite. Minimize-to-menu-bar and login startup remain off by default.
- Login startup uses a per-user LaunchAgent. Enable it only after moving the suite out of its disk image and into its installed location.
- Existing PhotoCraft, VectorCraft and EffectCraft plugin adapters use their upstream Mac Application Support folders. Plugin integration remains experimental.
- Cloud backups continue to use folders managed by installed Google Drive, Dropbox or OneDrive clients. Local backup completion does not confirm provider upload completion.

## Updates

Both Intel and Apple Silicon select only `ArtCraftMasterSuite-macOS-universal.dmg` for suite updates. The downloader checks its SHA-256, then the Mac installer checks the bundle identifier, native architecture and code-signature integrity. The replacement waits for the suite to exit and retains the previous `.app` beside the installation in an `.ArtCraft-update-*` folder.

The installed location must be writable by the current user. Running from a mounted disk image or App Translocation is not eligible for automatic replacement; install in Applications or `~/Applications` first. No administrator prompt, quarantine removal or Gatekeeper bypass is performed. The beta update channel is enabled by default for this beta build. Disable Include beta suite updates in Settings for stable releases only. Draft releases are never offered.

## Build

Run **Actions → macOS package → Run workflow** for a Mac-only build candidate. The release workflow also builds the universal DMG and includes it with the Windows and Linux downloads.

On a Mac with Xcode command-line tools, Rust stable and Python 3.11 or newer:

```sh
bash packaging/macos/build-dmg.sh
```

This compiles both Apple targets, combines them with `lipo`, creates the `.app` metadata and icon, applies an ad-hoc signature, and creates a compressed DMG. It also checks both architectures, the signature and disk-image integrity. Output is under `dist/macos/`.

Desktop installation, tray behavior, updates and launching upstream Craft apps still require hands-on Mac validation.
