# Workspace app information

Build 3.3 includes a short app summary in each workspace header. The copy is paraphrased from the upstream sources below, reviewed on 9 October 2026. It describes upstream capabilities, rather than promising feature parity in every installed version. The former feature-card section has been replaced by a project library using the same compact card styling.

Installed and latest versions come from the launcher's existing app state and release checks; they are not stored in the overview catalog. Release notes link to each app's own releases. The newer apps without an accessible product page link to their official repository.

| App | Source |
| --- | --- |
| PhotoCraft | [Product page](https://getartcraft.com/apps/photocraft) |
| VectorCraft | [Product page](https://getartcraft.com/apps/vectorcraft) |
| FilmCraft | [Product page](https://getartcraft.com/apps/filmcraft) |
| LightCraft | [Product page](https://getartcraft.com/apps/lightcraft) |
| PdfCraft | [Product page](https://getartcraft.com/apps/pdfcraft) |
| EffectCraft | [Product page](https://getartcraft.com/apps/effectcraft) |
| DesignCraft | [Product page](https://getartcraft.com/apps/designcraft) |
| SoundCraft | [Official repository](https://github.com/storytold/soundcraft) |
| CADCraft | [Official repository](https://github.com/storytold/cadcraft) |
| GridCraft | [Official repository](https://github.com/storytold/gridcraft) |
| WordCraft | [Official repository](https://github.com/storytold/wordcraft) |
| DeckCraft | [Official repository](https://github.com/storytold/deckcraft) |

Maintain the English catalog in `ArtCraftLauncher-Source/src/windows_ui/profile.rs` and its translations in `assets/translations-design.json`. The overview uses the installed app artwork already loaded by the launcher. The reference launcher's name, assets and marketing copy are not used.

The workspace design is shared across Windows, Linux and macOS. Projects, Assets and Plugins remain below the header. Workspace projects default to a compact grid, with a separately remembered list/grid/waterfall choice. The existing project actions, previews and backup indicators are reused. The sidebar's Workspaces section links directly to each app workspace. Both expanded and collapsed sidebars provide workspace shortcuts. Native file-manager actions remain platform-specific.
