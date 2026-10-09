# Publishing installable downloads

## Build 3.3 prerelease

Package version: `3.3.0-beta.1`. Release tag: `v3.3.0-beta.1`.

1. Commit and push the intended source, assets, packaging and workflows after approval.
2. Create a **draft prerelease** for `v3.3.0-beta.1`, targeting that commit. Use `RELEASE_NOTES_3.3.md` for its notes.
3. Run **Actions → Release downloads → Run workflow**, choosing the updated default branch and that tag. Set **publish** to true only when publication is authorized.
4. Wait for all native jobs to succeed. The workflow builds Windows x64, Linux x86_64/ARM64 AppImage and Flatpak packages, and the universal macOS DMG from the tag.
5. With **publish** enabled, the workflow publishes the draft only after all six packages and checksums are uploaded. Otherwise, the release stays a draft.

The version job rejects tags that do not match Cargo.toml. Tags with a prerelease suffix are marked as prereleases and excluded from GitHub's latest stable release by the upload job.

## Asset names

- `Windows-x86_64.exe`
- `Linux-x86_64.AppImage`
- `Linux-x86_64.flatpak`
- `Linux-aarch64.AppImage`
- `Linux-aarch64.flatpak`
- `macOS-universal.dmg`
- `SHA256SUMS.txt`

Names contain no spaces and follow `OS-architecture.extension`. The `.flatpak` spelling is intentional. Internal Actions artifacts may be ZIP containers; the release uploads individual packages.

All six packages must be present and nonempty before upload. Checksums are generated from those fresh build outputs. A failed native job prevents that run's upload job. Re-running a successful tag replaces same-name assets; never reuse a version tag for different source.

The workflow uses GitHub's built-in token and never republishes the older setup executable stored at the repository root. Windows builds the launcher before setup because setup embeds that executable.

## Update compatibility

Build 3.2 recognizes both new and historical filenames. Older installed suite versions may require a one-time manual upgrade because their updater expects the old names. Upstream Craft app package names are unchanged.

AppImage updates preserve a previous copy. Flatpak updates use the exact checksum-verified GitHub bundle and preserve user/system installation scope. macOS uses the universal DMG and retains its previous app bundle. The prerelease preference applies to every package type; drafts are excluded.

## Native requirements

Linux and macOS packages are built on native GitHub runners; WSL is not required on the maintainer's Windows PC. Successful packaging does not establish desktop compatibility. Installation, launching, tray behavior and updates still require native desktop validation.

Windows Authenticode signing is not configured. The Mac app is ad-hoc signed, not Developer ID signed or notarized. Historical Build 3.0 and 3.1 assets keep their original names.
