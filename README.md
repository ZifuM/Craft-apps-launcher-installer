# ArtCraft Master Suite

A native Windows launcher for Storytold’s open-source creative and productivity apps. Install and manage the apps, browse projects from one place, and open each project in the app that supports it.

> This is an independent community project and is not affiliated with Adobe Inc. Product names and marks belong to their respective owners.

## Download

Download and run **[ArtCraftMasterSuite-Setup.exe](ArtCraftMasterSuite-Setup.exe)** to install the launcher for your Windows user. Setup adds Start menu and desktop shortcuts.

The launcher installs creative apps separately and keeps project files in their existing folders. A network connection is needed to find, install, and check for app releases.

## What it does

- Installs, updates, opens, and uninstalls the supported apps.
- Brings projects from selected folders into one searchable library with List, Grid, and Waterfall views.
- Shows project details and previews when a file contains preview data the launcher can read.
- Creates an app-named subfolder for every tool in added project directories.
- Supports renaming and deleting project files, with confirmation before deletion.
- Checks for updates automatically and lets you check an app manually.

## Sidebar designs

The Modern sidebar groups your workspace and management tools, with a compact icon rail available. Prefer the previous layout? Select **Settings → Appearance → Sidebar design → Classic** to restore it immediately. This preference is saved across restarts.

## Your apps

Open **Your apps** in the sidebar to browse installed tools, grouped into Creative and Productivity apps. Select an app tile to access its existing launch, update, uninstall, and project controls. The collection updates when apps are installed or removed.

## Workspace preferences

Settings is organized into **Appearance**, **Updates**, **Projects**, and **About** tabs, with saved controls for:

- Automatic release checks, with an interval from 1 to 24 hours (default: 4 hours).
- Persistent update notifications, which can be turned off separately.
- Automatic project refresh, from 1 to 30 minutes (default: 3 minutes).
- Compact sidebar, reduced motion, and List / Grid / Waterfall project views.
- Project folders and the default working folder for apps.

Background checks run while the launcher is open. Updates are never installed automatically. Manual checks and refresh remain available when background checks are off.

## Supported apps

**Creative Apps:** PhotoCraft · VectorCraft · FilmCraft · LightCraft · EffectCraft · DesignCraft · SoundCraft

**Productivity Apps:** PdfCraft · CADCraft · GridCraft · WordCraft · DeckCraft

SoundCraft is listed in the launcher, but does not yet have an official Windows release to install.

The launcher recognizes common project and media extensions for each app, including PSD/PSB, SVG, video-project interchange files, camera RAW, PDF, Lottie, and page-layout documents. See the [source README](ArtCraftLauncher-Source/README.md) for the complete format list, local data locations, and build instructions.

## Source code

The current source code and assets are in [`ArtCraftLauncher-Source`](ArtCraftLauncher-Source/).

The launcher source is licensed under the [MIT License](ArtCraftLauncher-Source/LICENSE). The license for the launcher does not change the rights for third-party app names, logos, or other assets.

## Verify downloads

`SHA256SUMS.txt` contains SHA-256 a checksum for the installer. On Windows, verify a file with:

```powershell
Get-FileHash .\ArtCraftMasterSuite-Setup.exe -Algorithm SHA256
```

Compare the resulting hash with the matching entry in `SHA256SUMS.txt`.
