# Windows interface redesign

> Build 3.2 update: this implementation is now shared with Linux and macOS. The Windows-only boundaries described below record the original redesign phase; see `RELEASE_NOTES_3.2.md` for the current port.

This is a local Windows preview of the interface redesign, based on Build 3.1. It has not been published as a release.

## Design

- Compact navigation with one label per destination, aligned counts, and descriptive tooltips.
- Consistent page headings, 36–40 px controls, restrained borders, and app-specific color accents.
- Content width is capped at 1360 px and centered on large displays. Cards adapt to the available width.
- Home combines a brand banner, overview cards, horizontal quick-launch tiles, and recent projects.
- App Manager and Your apps use matching cards, stable action positions, search, categories, and installation filters.
- Projects retain list, grid, and waterfall views, previews, favorites, rename/delete, cloud status, and Explorer actions.
- Workspaces use a compact identity panel and Projects, Assets, and Plugins tabs.
- Cloud groups file selection into one panel and backup controls into a separate overview. At narrow widths the overview becomes a compact block above the file list.
- Settings, onboarding, plugin panels, placeholders, and project dialogs use the same spacing and surface treatments. Experimental features remain visibly labeled.

## Platform boundary

The new views are in `ArtCraftLauncher-Source/src/windows_ui.rs` and `src/windows_ui/`. Conditional module paths select them only for Windows. The existing Linux and macOS layouts are retained until a port is explicitly requested. Installation, update selection, project scanning, thumbnail generation, cloud backup, and plugin backends remain shared and unchanged by this pass.

## Backup and rollback

The pre-edit project snapshot is saved outside the repository:

`C:\Users\oshot\Documents\Codex\ArtCraft-backups\before-ui-redesign-20261009.zip`

An unpacked copy is available beside it. It contains the source, vendored dependencies, assets, workflows, documentation, and root release files. Generated `target`/`dist` directories and `.git` history are excluded. To revert the interface, restore `ArtCraftLauncher-Source/src/main.rs` from that snapshot; the Windows-specific view files will no longer be selected. Restore other project files only if a complete snapshot rollback is intended.

## Review

The Windows release executable compiles successfully. The running Home, App Manager, and Cloud views were inspected during development. This is not a full regression test of installation, updates, backup, or plugin operations.

Review executables are placed in `dist/windows-ui-preview/`. The preview retains the 3.1.0 package version and does not change the GitHub draft release.

## Library toolbar revision

The current Windows preview is in `dist/windows-toolbar-redesign/`; the repository-root `ArtCraftMasterSuite-Setup.exe` contains the same build.

- App Manager, Your apps, and Projects have new icon-led headings, native Segoe UI typography, neutral segmented navigation with counts, and aligned 40 px controls.
- Searches have a search icon, focus border, clear action, and Ctrl+F shortcut. Escape clears a focused search.
- App Manager has a labelled Check updates action with a busy state, and additional Not installed and Needs attention filters.
- Projects groups app/scope/sort controls separately from its view picker. Add folder is the primary header action.
- The app-library command bars reflow below category navigation in narrower windows. Existing app colors and card actions are retained.
- Fonts are loaded from Windows when available, with the existing multilingual fonts as fallbacks; no Windows font files are redistributed.

The pre-toolbar source files are backed up in `C:\Users\oshot\Documents\Codex\ArtCraft-backups\before-toolbar-redesign-20261009`. Both Windows release binaries built successfully. The Your apps layout was visually inspected in the running preview; no automated tests or full functional regression checks were run. Nothing was pushed or published.

### Alignment and workspace follow-up

- Header icons and actions are vertically centered against the measured title and subtitle block. Wrapped subtitles reserve their own height.
- Primary header actions use the suite purple. Button icons and labels are centered together.
- Workspace Projects/Assets/Plugins navigation, project search, and view selection use the shared toolbar components. Project actions and plugin/asset functionality are retained.
- Windows refresh controls share an open circular arrow with a clear arrowhead.
- Project reveal actions use `SHOpenFolderAndSelectItems` to open the actual parent folder and select the file, avoiding Explorer command-line parsing of paths with spaces. If selection fails, the containing folder is opened instead.
- Pre-edit source copies are in `C:\Users\oshot\Documents\Codex\ArtCraft-backups\before-workspace-toolbar-20261009`.

### Home, Cloud, Settings and Plugins

- Page titles, icons and header actions now occupy one shared row; subtitles sit below it. This supersedes the earlier centering against the full title/subtitle block.
- Home uses the shared header, section actions, typography and overview cards. The colored launch tiles and orbital artwork remain; the banner reserves enough space around the outer icons.
- Cloud uses segmented section navigation, the shared searchable file toolbar, consistent provider buttons, and aligned backup actions. Selection, automatic backup, saved versions, cancellation, restore and sync-folder management retain their existing behavior.
- Settings uses the same section navigation and consistent label/control columns for language, theme, sidebar design and project layout. All existing preferences remain available.
- Plugin management uses the shared button styling and a dedicated release-URL field in its installation panel. Integration folders, local-file/GitHub installs, enable/disable and removal remain available.
- Backups for this pass are in `C:\Users\oshot\Documents\Codex\ArtCraft-backups\before-home-cloud-refresh-20261009`.
- This pass is Windows-only. No automated tests were run and no changes were pushed or published.

### App Manager source links

A Links & community section below the app cards opens the official ArtCraft website, ArtCraft Discord, Storytold GitHub, and the Master Suite repository in the default browser. The buttons share the toolbar styling, show their destination on hover, and wrap to two columns in narrower windows. They remain available when a search has no matching apps.

### Compact app cards

App Manager and Your apps now share 160 px cards with no top stripe or internal divider. Each card keeps its app icon, name, category, a one-line description, and the primary Open/Install action. A three-dot menu contains the full description, status, versions, project count for installed collections, progress/error details, workspace access, update checks/installation, and uninstall (with the existing confirmation flow). Small menu indicators keep errors and available updates discoverable. Card background clicks still open the workspace, and the original app colors remain.

The previous card source is backed up in `C:\Users\oshot\Documents\Codex\ArtCraft-backups\before-compact-app-cards-20261009`.


### Context menus, Cloud layouts and readability

- App cards now have an action-only menu: Check for updates, Properties, and Uninstall. Properties contains the description, installed/latest versions, installation folder, executable path, available file metadata, supported formats, workspace access and update installation. No executable renaming is performed.
- Project rows and tiles use a three-dot menu for Open, Rename, Show in File Explorer, Copy path, Favorites, Cloud backup, Properties and Delete. Right-click offers the same actions. Existing rename/delete confirmations and precise Explorer selection remain in place.
- Cloud project files have independent, persisted List, Grid and Waterfall layouts. Search, filters, select-all, individual backup selection, backup status and project menus work across layouts.
- Settings > Appearance adds Small, Medium and Large text (100%, 112.5%, 125%). Small is the existing default. The preference applies immediately and persists across launches; it scales application text rather than Windows desktop scaling.
- A Windows-specific translation catalog fills missing redesigned screen labels, descriptions, menus, dialogs and application status/error messages for Spanish, French, German, Portuguese, Russian, Simplified Chinese, Japanese, Korean and Italian. Plugin messages now use the translator. Names, paths, URLs and external system diagnostics retain their original content. Dates on redesigned Windows screens use numeric formatting.
- Source backup: `C:\Users\oshot\Documents\Codex\ArtCraft-backups\before-context-menus-20261009`.
- Windows release compilation only; no automated tests or interactive UI validation were run. No push or publishing.

### App menu shortcuts

The four app shortcuts (Show in File Explorer, Copy file path, Open install folder, Open workspace) now occupy a fixed horizontal icon row above the context-menu actions. Each uses a translated tooltip and accessibility label; file actions are disabled when their target is unavailable. These shortcuts were removed from app Properties. Version details and update installation remain in Properties. Backup: before-menu-shortcuts-20261009.


### Compact navigation and backup indicators — 2026-10-09

- Center segmented-control labels and count badges together; translate and truncate long labels inside their own segment.
- Match Home hero button widths. Scale orbit icons progressively from 22px to the existing 36px size.
- Replace decorative toolbar dividers with compact spacing in Projects, workspaces, Settings, Cloud and Plugins. Retain separation for destructive context-menu actions.
- Combine workspace library heading, a search capped at 360px and view controls into one row, with a narrow-window fallback.
- Condense sync providers to 72px rows in wide windows. Keep folder selection and backup inclusion visible; move folder opening, desktop launch, downloads and disconnect into the provider menu. Preserve disconnect confirmation and busy-state guards.
- Bundle original Google Drive, Dropbox and OneDrive logos. Asset provenance is in `assets/providers/sources.json`; no runtime image downloads.
- Windows backup status polls metadata in a worker every five seconds. `CfGetPlaceholderStateFromAttributeTag` must report both backup data and its manifest in sync before displaying a green checked cloud. Unsupported provider filesystems remain “Copied to sync folder,” with upload explicitly unconfirmed.
- Status icons are shared by Cloud list/grid/waterfall and project rows/tiles (including Home and app workspaces). Deselecting a project does not hide its existing backup. New source revisions show changes awaiting backup; missing files and errors show attention states. Last-backup timestamps are local version creation times, not invented upload times. Status observations expire after 20 seconds.
- New backup labels have translations for all nine alternate UI languages.
- Builds compile on Windows; no interactive provider upload or UI tests were run for this change.

Cloud state API reference: https://learn.microsoft.com/en-us/windows/win32/api/cfapi/ne-cfapi-cf_placeholder_state

### Compact app menus and complete header alignment

- App context menus size to their translated action labels, with a 184px minimum instead of stretching to the popup's available width. Four fixed 36px square shortcut buttons sit in one centered row.
- Shortcuts use folder-search, overlapping-copy, open-folder and window-layout symbols, with the existing translated tooltips and accessibility labels.
- The shared Windows page header now vertically centers its icon and action against the full title and subtitle block, including wrapped translations and text scaling. Footer spacing is excluded from the alignment calculation.


### Sidebar alignment

- Navigation icons, labels and count badges use the same row center. Labels use measured font height instead of a fixed top offset. Count badges adapt to text scaling, with single-line truncation and translated tooltips retained for navigation labels.

