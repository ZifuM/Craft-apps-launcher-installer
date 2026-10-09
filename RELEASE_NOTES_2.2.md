# ArtCraft Master Suite — Build 2.2

A refreshed workspace for managing your ArtCraft apps and picking up your projects.

## Highlights

- **Beta label:** the window branding and About version now identify this build as beta.
- **Ten interface languages:** English (default), Spanish, French, German, Portuguese, Russian, Simplified Chinese, Japanese, Korean, and Italian. Change the saved preference in Settings → Appearance → Language.
- **Redesigned project dialogs:** centered rename, delete, and uninstall dialogs with file details, clearer warnings, and aligned actions.
- **Preview refresh after saving:** background checks detect changes to known projects and invalidate old thumbnails across supported file formats.

- **Redesigned Home:** a new banner, workspace overview, app-colored quick launch cards, and recent projects.
- **New Your apps and App Manager layouts:** clearer app cards, search, Creative and Productivity filters, and consistent app-colored actions.
- **Redesigned app workspaces:** a searchable Projects tab with List, Grid, and Waterfall views, plus placeholder tabs for Asset management and Plugin management.
- **Improved project library:** compact full-width list rows, neutral Grid and Waterfall cards, favorites, sorting, folder management, and File Explorer shortcuts.
- **Saved-file previews:** supported PSD/PSB canvases, SVG artwork, PDFs, embedded project thumbnails, and additional document, audio, and CAD preview support where available.
- **Master Suite automatic updates:** checks GitHub Releases for newer suite versions, verifies download checksums, and installs updates automatically when enabled. A manual suite update check is available in Settings.

## Appearance and preferences

- Placed the Beta badge beside the window title, removed workspace tab underlines, and added GitHub credits for every app under Settings → About.

- Refined launcher and app splash screens with softer orbit artwork, staggered logo entrances, wrapped descriptions, and a smooth 3-second progress animation.
- Best-effort background warming of app executable and runtime files during the splash; apps still start only after the animation finishes.

- Light and Dark themes, categorized settings, and optional compact navigation.
- Optional minimize-to-tray and start-with-Windows-in-tray settings, both off by default.
- Retained Classic sidebar and Classic app screen options for restoring earlier layouts.
- Refined scrollbars, tighter right-edge spacing, aligned sidebar badges and banner content, and improved card clipping while scrolling.

## Fixes

- Corrected app-card text drawing over page headers during scrolling.
- Improved installer file handling and shutdown coordination to address the file-in-use installation error.

## Availability and limitations

- Asset management remains a **UI placeholder**. Plugin management supports compatible apps as described below.
- Previews show the **last saved file**, not unsaved changes. Some formats depend on embedded thumbnails or Windows thumbnail support; unsupported content may show “Preview unavailable.”
- Cloud uploads require a provider desktop app; Master Suite reports local backup copies only.
- Technical system errors and untranslated messages fall back to English. East Asian text uses installed Windows fonts. Translations have not yet had native-speaker review.
- Creative and productivity app updates remain manual; automatic updating applies to Master Suite itself.

## Install

Download **ArtCraftMasterSuite-Setup.exe** and run it to install or update. Existing preferences and project files are retained. **SHA256SUMS.txt** is provided for checking the installer download.

For an older installation without the suite updater, install Build 2.2 manually once to receive future automatic suite updates.

### First-run setup and cloud groundwork

- Added a first-launch setup window for language, project folder, appearance, Windows startup preferences and optional cloud connection. Existing users who have not completed onboarding also see it once.
- Added the Cloud page, project backup selection, saved-version history, restore-copy actions and project status icons.
- Added Google Drive, Dropbox and OneDrive backup destinations through desktop sync folders, without developer registrations.

### Cloud interface refinements

- Condensed provider connections into compact cards, arranged side by side on wider windows.
- Restyled cloud tabs, search, backup controls, project previews and sync status badges to match the suite.
- Scrollbars fade out after scrolling stops while keeping content spacing stable.
- Older preferences without an onboarding completion flag now show setup once.

### Plugins and organized workspaces

- Added GitHub/local-file installation and enable/disable/remove controls for compatible PhotoCraft, VectorCraft and EffectCraft WebAssembly plugins.
- Added per-app Projects, Exports, Assets and Plugins folders without relocating existing projects.
- Added native plugin-folder adapters, settings backups and an EffectCraft export-folder preference adapter.
- Added a workspace configuration contract and app-side integration helper. Full save/export routing across all apps still requires changes to those applications; it is not available through their sandboxed filter plugins.

### Cloud workspace redesign

- Rebuilt Cloud around Files, Saved versions, Sync folders and Assets views.
- Added a compact file table, visible-file selection, search/status filters and backup summary counts.
- Moved provider connections into a dedicated sync-folder management view with disconnect confirmation.
- Improved saved-version dates, restore actions, empty states and connection error reporting.
- Replaced direct sign-in with desktop sync folders. Includes manual and optional automatic versioned backups, checksum-verified restore copies and local backup status. Provider desktop apps handle sign-in and uploads; live uploads have not been verified.

- Added an Experimental banner to Cloud, Plugin management and Asset management to indicate that these features are still in development and may not function as intended.

### Linux port groundwork (not yet validated on Linux)

- Added OS/architecture-specific Craft release selection with checksum validation.
- Added Linux paths, app launching, tray/login startup adapters and preview support.
- Added separate AppImage and Flatpak packaging workflows for x86-64 and ARM64.
- Suite updates select the matching AppImage or installed Flatpak ref; Linux never downloads the Windows installer.
- macOS remains deferred. See docs/LINUX.md for validation and distribution requirements.
