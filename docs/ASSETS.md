# Assets

Open **Assets** in the sidebar for every app's library, or open an app workspace and select **Assets**. Both use the same light/dark layout and controls as Projects. The shared top search field searches the active library and stays centered on the window.

## Import and save

- **Import files** accepts images, video, audio/MIDI, fonts, models, documents, presets and other files. Choose the app that owns the library. Files are copied into `<project folder>/<App>/Assets`; originals remain untouched. Unsupported formats can be stored without being executed.
- **Import folder** preserves the folder structure and companion files. Hidden files and symbolic links are excluded. Import at most 10,000 files at a time, with no more than 32 folder levels.
- **Import presets** stores exported presets in `<project folder>/<App>/Assets/Presets`. Presets keep their original format. ZIP files are stored as files; they are not automatically extracted or installed.
- Drop local files or folders onto the Assets screen to open the same import dialog.
- Save or export directly from an app into its Assets folder, then choose **Refresh**. The suite's normal automatic library scan also refreshes assets when enabled. **Save presets from app** shows instructions and provides the destination folder.
- Existing names receive a numbered copy. Imports never overwrite another asset. Progress and errors are displayed while copying, and a file that changes during copying is rejected.

## Browse and organize

Search by file name, path, app or type; filter by app, type, favorites or recent changes. Sort by name, date, size or app. Grid uses fixed-width tiles with 16:9 preview frames; Waterfall follows image proportions. List aligns names, types, app, date and size in columns.

Common raster images, SVG, saved document thumbnails and supported audio/document formats use the suite's preview readers. Unsupported or oversized files use a format label. Preview decoding is bounded and does not execute file contents. Large libraries limit the number of decoded previews retained in memory.

Use the selection checkboxes for **Save copies** or **Remove**. The three-dot/context menu provides favorites, rename, file location and path copying. Renames preserve the file extension. Removal moves library copies into that Assets folder's `.Trash` directory; open it from **Folders → Open library trash** to recover a file. Renaming or moving an asset already linked to a document can require relinking in that app.

Assets stay separate from the Projects list. Their app/Assets folder hierarchy is retained by the existing Google Drive and NAS backup features. Asset favorites follow a library-folder move.

## Use in the apps

Double-click a tile/name or select **Use in app**. Choose an installed app and read the available action:

- **Open in app** passes supported file types to the official app's desktop file-open/import route. PhotoCraft supports document/image files and imports `.abr`, `.grd`, `.aco`, `.ase`, `.kys` and supported `.psp` shortcut sets this way. FilmCraft and EffectCraft receive supported media imports; SoundCraft receives audio/MIDI files; LightCraft receives supported photos.
- **Open app and copy path** launches the app and copies the asset's full path for its Import, Place, Insert or preset picker. Instructions are shown for PhotoCraft LUTs/patterns, LightCraft presets, palettes and fonts.
- To place a file in an **existing document**, use that app's own Import/Place/Insert command with **Copy file path**. Master Suite does not remotely edit a currently open document or apply a preset to it.
- Use **Save As** in an editor to preserve the library original. The app remains responsible for reporting unsupported codecs, invalid presets and version compatibility. Native font installation remains in the operating system's font manager.

PhotoCraft's internal `.pcbrushes` groups can reference separate `tips/*.pctip` files; a group file alone is not a portable brush pack. Keep a complete folder together or export a portable preset using PhotoCraft. LightCraft offers **Export User Presets** and **Export Group** in its Presets panel and **Import Presets** to load `.lcpreset`/supported preset formats.

## Official references

Reviewed on 10 October 2026:

- PhotoCraft [preset file handling](https://github.com/storytold/photocraft/blob/0c72d95425dece90ef9a1cceb49e3315c96e22d5/crates/ui-egui/src/preset_files_ui.rs), [persistent brush store](https://github.com/storytold/photocraft/blob/0c72d95425dece90ef9a1cceb49e3315c96e22d5/crates/engine/src/preset_store.rs), and [raster formats](https://github.com/storytold/photocraft/blob/0c72d95425dece90ef9a1cceb49e3315c96e22d5/book/src/formats/raster-formats.md).
- LightCraft [preset panel import/export](https://github.com/storytold/lightcraft/blob/1cc0dde747917f96a0a368a6d6cf8b3b8b36b62c/crates/ui-egui/src/panels/presets.rs).
- [FilmCraft desktop imports](https://github.com/storytold/filmcraft/blob/7bd76212629a067a9d1c5426d751de9faf4664b7/apps/filmcraft/src/main.rs), [EffectCraft desktop imports](https://github.com/storytold/effectcraft/blob/813c7c4650f0d4c805da620c8d5837c9600ad2e3/apps/effectcraft/src/main.rs), and [SoundCraft audio/MIDI opening](https://github.com/storytold/soundcraft/blob/522e59f8f51734f59cebedcbf6641a8e77591c5f/apps/soundcraft/src/main.rs).

Current official app source is the compatibility reference; older installed releases may support fewer formats. Windows compilation is checked during development; end-to-end asset handoff and native macOS/Linux behavior require runtime verification.
