# Installing plugins

Open an installed app's workspace and choose **Plugins**. This screen uses the suite's current light or dark appearance and is shared with Legacy.

1. Paste a public GitHub repository, release page, release download, or compiled file link and click **Install from GitHub**. If the release contains several packages, choose the one for your app and computer.
2. Alternatively, click **Install from file** for a compiled plugin or ZIP. SoundCraft also has **Install bundle folder** for native plugin directories.
3. Close the target app before installation. The suite checks compatibility and displays an error if a package belongs to another app or platform.
4. Restart the app to load the plugin. Its own loader performs the final manifest, API version, entry-point and runtime checks.

**Installation folders** shows the actual destination. The suite keeps a managed copy under `<project folder>/<App>/Plugins`. Existing custom plugin locations are respected. Search, enable, disable, reveal and remove controls apply to plugins installed through Master Suite. Plugins installed independently remain untouched and are not listed as suite-managed plugins.

## Supported loaders

| App | Accepted packages | Where the app loads them |
| --- | --- | --- |
| PhotoCraft | ABI v1 `.wasm`, or ZIP containing those modules | Its existing Additional Plug-ins Folder, or the workspace `PhotoCraft/Plugins` folder. The suite enables `plugIns.useAdditionalPluginsFolder` and sets `plugIns.additionalPluginsFolder` in `preferences.json`. Files are placed directly in the folder because the app scans non-recursively. |
| VectorCraft | ABI v1 `.wasm`, or ZIP containing those modules | Its existing Additional Plug-ins Folder, or workspace `VectorCraft/Plugins`. The suite sets `engine_prefs.pluginsFolder` in `ui.json`. Modules are placed directly in that folder. |
| EffectCraft | API v1 `.wasm`, or ZIP containing those modules | `Plug-ins` beside the app's `prefs.json`. The workspace copy remains separate. Text `.wat`, scripts and ScriptUI panels are not installed by this screen. |
| SoundCraft | `.clap`, `.vst3`, complete bundles, or ZIP; `.component` bundles on macOS | The per-user CLAP/VST3/Audio Unit folders listed below. These are shared audio plugin locations. |
| Other suite apps | No installer exposed | No compatible third-party file loader was confirmed in the reviewed official documentation and source. |

PhotoCraft uses `%APPDATA%/Photocraft`, `~/Library/Application Support/Photocraft`, or `$XDG_CONFIG_HOME/photocraft` (falling back to `~/.config/photocraft`). `PHOTOCRAFT_CONFIG_DIR` takes precedence. A portable marker beside the suite-managed PhotoCraft executable selects `PhotoCraftData/preferences.json`.

VectorCraft uses `%APPDATA%/VectorCraft/ui.json`, `~/Library/Application Support/VectorCraft/ui.json`, or `$XDG_CONFIG_HOME/vectorcraft/ui.json` (falling back to `~/.config/vectorcraft/ui.json`). If this file does not exist, preferences from the former `DrawCraft` / `drawcraft` folder are preserved when creating it, matching the app's migration behavior.

EffectCraft uses `%APPDATA%/EffectCraft/Plug-ins`, `~/Library/Application Support/EffectCraft/Plug-ins`, or `$XDG_CONFIG_HOME/effectcraft/Plug-ins` (falling back to `~/.config/effectcraft/Plug-ins`). `EFFECTCRAFT_CONFIG_DIR` takes precedence.

| SoundCraft format | Windows | macOS | Linux |
| --- | --- | --- | --- |
| CLAP | `%LOCALAPPDATA%/Programs/Common/CLAP` | `~/Library/Audio/Plug-Ins/CLAP` | `~/.clap` |
| VST3 | `%LOCALAPPDATA%/Programs/Common/VST3` | `~/Library/Audio/Plug-Ins/VST3` | `~/.vst3` |
| Audio Units | Not supported | `~/Library/Audio/Plug-Ins/Components` | Not supported |

Audio bundles retain their directory layout and executable permissions. Packages with a different OS/CPU binary are rejected. Disabling an audio plugin moves the active copy outside scanned plugin folders; removing or disabling it also affects other audio apps using the shared copy. The Linux Flatpak grants access only to the specific per-user audio plugin and disabled-plugin folders in addition to its existing permissions.

## Package handling

- WebAssembly is validated without running it: core-module format, no imports, supported features, bounded initial memory, correct memory export and app-specific function signatures. The suite does not claim the plugin's manifest or runtime behavior is valid before the target app loads it.
- ZIP packages are bounded in size and file count. Unsafe paths, duplicate paths and symbolic links are rejected. Install bundles that require symlinks or a vendor installer using the publisher's installer instead.
- The suite downloads compiled release assets over HTTPS, follows only GitHub download hosts and checks GitHub's SHA-256 digest when one is supplied. Repository source archives, application installers and unsupported native formats are not executed or built.
- All packages are checked before installation. File changes are tracked for rollback if installation fails; existing files are never overwritten. App preferences are backed up before configuration. Plugin operations are locked per app to avoid simultaneous installs from different suite instances.
- Enable/remove operations compare saved checksums and leave externally modified plugin files untouched. Unknown or damaged app preferences produce an error rather than being replaced with defaults.
- A ZIP may be at most 128 MiB, its declared expanded contents 256 MiB, with at most 4,096 entries and 32 plugins. Each WebAssembly module is limited to 32 MiB.

## Official references reviewed

Reviewed against the official [ArtCraft organization](https://github.com/storytold) on 10 October 2026:

- [PhotoCraft plugin API and loading](https://github.com/storytold/photocraft/blob/0c72d95425dece90ef9a1cceb49e3315c96e22d5/docs/plugins.md), [desktop paths](https://github.com/storytold/photocraft/blob/0c72d95425dece90ef9a1cceb49e3315c96e22d5/apps/photocraft/src/app_dirs.rs).
- [VectorCraft plugin API and loading](https://github.com/storytold/vectorcraft/blob/1090385507f2215a4d7558f59e19c34cc47d5a4c/docs/plugins.md), [desktop preferences path](https://github.com/storytold/vectorcraft/blob/1090385507f2215a4d7558f59e19c34cc47d5a4c/apps/vectorcraft/src/main.rs).
- [EffectCraft plugins](https://github.com/storytold/effectcraft/blob/813c7c4650f0d4c805da620c8d5837c9600ad2e3/docs/plugins.md), [preferences locations](https://github.com/storytold/effectcraft/blob/813c7c4650f0d4c805da620c8d5837c9600ad2e3/docs/preferences.md).
- SoundCraft [CLAP scan paths](https://github.com/storytold/soundcraft/blob/522e59f8f51734f59cebedcbf6641a8e77591c5f/crates/clap-host/src/scan.rs), [VST3 scan paths and bundle layout](https://github.com/storytold/soundcraft/blob/522e59f8f51734f59cebedcbf6641a8e77591c5f/crates/vst3-host/src/scan.rs), and [Audio Unit host](https://github.com/storytold/soundcraft/blob/522e59f8f51734f59cebedcbf6641a8e77591c5f/crates/au-host/src/lib.rs).

Use an up-to-date app release with the documented loader. Older releases may not yet include the same plugin support. Native macOS/Linux installation and real plugin loading have not been manually exercised for this change.
