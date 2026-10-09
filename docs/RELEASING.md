# Publishing installable downloads

The **Release downloads** GitHub Actions workflow builds the suite on Windows, Linux and macOS and attaches the actual files to GitHub Releases. No WSL or local Linux installation is needed. It uses GitHub's built-in token; no personal access token needs to be saved in the repository.

## Build 3.0

1. Commit and push the updated source, `packaging/` files and both `.github/workflows/` files using GitHub Desktop.
2. On GitHub, create a release with the tag **v3.0.0**, targeting that updated commit. Use `RELEASE_NOTES_3.0.md` as the starting notes.
3. Publish the release (mark it as a prerelease while Linux is being validated). This starts **Release downloads** automatically.
4. Wait for the workflow to succeed. It uploads these individual assets, without wrapping them in ZIP files:

   - `ArtCraftMasterSuite-Setup.exe` — Windows x64 installer.
   - `ArtCraftMasterSuite-Linux-x86_64.AppImage` — Intel/AMD Linux.
   - `ArtCraftMasterSuite-Linux-x86_64.flatpak` — Intel/AMD Linux.
   - `ArtCraftMasterSuite-Linux-aarch64.AppImage` — ARM64 Linux.
   - `ArtCraftMasterSuite-Linux-aarch64.flatpak` — ARM64 Linux.
   - `ArtCraftMasterSuite-macOS-universal.dmg` — Intel and Apple Silicon Macs (Build 3.1 onward).
   - `SHA256SUMS.txt` — combined checksums for the installer packages.

For a release that already exists, use **Actions → Release downloads → Run workflow**, select the updated default branch and enter its release tag. This also works with an existing draft release, allowing package review before publishing. The tag must point to the intended source and match Cargo.toml (`3.0.0` or `v3.0.0`). Do not reuse an old release tag for new source.

If a job fails, no new packages are uploaded by that run. Open the failed job log, fix the cause and build the corrected source before publishing it. Re-running a successful tag replaces assets with the same names. Internal Actions artifacts are staging downloads; end users should use the individual files in the release's **Assets** section.

## Future versions

Update the package version in Cargo.toml and Cargo.lock, commit the changes, and create a matching release tag. The workflow compiles fresh binaries from that tag, including the launcher embedded in the Windows setup EXE. It never republishes the older EXE stored at the repository root.

Keep the asset filenames unchanged: the suite updater selects the exact platform and architecture filename and uses the release checksums. Flatpak bundles install directly, but automatic Flatpak updates additionally require a hosted Flatpak repository configured as the installation's origin; GitHub bundle uploads alone do not provide that service.

Build completion does not establish desktop compatibility. Linux installation, launching, tray integration and updating still require native validation before describing the Linux release as verified. Windows signing is not configured by this workflow.

## Build 3.1 macOS

Use tag `v3.1.0` and `RELEASE_NOTES_3.1.md` for the next release. The new release workflow also builds the universal DMG on a Mac runner. The `.app` is ad-hoc signed, not Developer ID signed or notarized. Keep this release a draft until the packages are available for review. The already published Build 3.0 files remain unchanged.
