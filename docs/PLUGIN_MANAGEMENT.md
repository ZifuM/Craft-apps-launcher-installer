# Plugin management and workspace integration

## Available now

Each app workspace has a Plugins page. See [Installing plugins](PLUGINS.md) for the current GitHub/local installer, supported formats and platform paths. PhotoCraft, VectorCraft and EffectCraft can install compiled WebAssembly plugins from a local `.wasm` file, a ZIP containing built modules, a GitHub file/release asset URL, or a repository URL with one suitable latest-release asset. Ambiguous repositories require the exact asset link. GitHub source archives are not compiled automatically.

Master Suite checks module validity, rejects host imports, and checks the selected app's required export names. The app performs the final ABI/manifest/runtime checks when loading. Use an app release that supports the documented interface; successful copying is not proof that an older app build can load a newer plugin.

Installed extensions can be enabled, disabled and removed. Close the app first. Removal checks the stored checksum and touches only the copies managed by Master Suite. Original downloads and projects remain unchanged. Plugin records include their source, checksum and deployment location.

### App adapters

| App | Interface | Where modules load |
| --- | --- | --- |
| PhotoCraft | `pc_*` WebAssembly filters | Additional Plug-ins Folder, enabled in `preferences.json`; portable settings are detected |
| VectorCraft | `vc_*` WebAssembly object plugins | `engine_prefs.pluginsFolder` in `%APPDATA%/VectorCraft/ui.json` |
| EffectCraft | `ec_*` WebAssembly effects | `Plug-ins` beside the app settings |
| SoundCraft | Native CLAP, VST3; Audio Units on macOS | Per-user OS audio plugin folders; see the [platform path table](PLUGINS.md#supported-loaders) |
| Other apps | Not enabled in this manager yet | A documented compatible adapter is required; native plugins are not treated as WebAssembly modules |

The project workspace keeps a plugin copy in its `Plugins` folder. If PhotoCraft/VectorCraft already use another plugin folder, it is preserved and the module is also deployed there. EffectCraft needs a copy in its native folder. Existing settings are backed up to `*.pre-master-suite.json` before changes. Restart the app to load newly installed modules.

References: [PhotoCraft API](https://github.com/storytold/photocraft/blob/main/docs/plugins.md), [VectorCraft API](https://github.com/storytold/vectorcraft/blob/main/docs/plugins.md), [EffectCraft API](https://github.com/storytold/effectcraft/blob/main/docs/plugins.md).

## Workspace folders

```text
Selected project root/
  PhotoCraft/
    Projects/
    Exports/
    Assets/
    Plugins/
    .artcraft-suite.json
  VectorCraft/
    Projects/
    Exports/
    Assets/
    Plugins/
    .artcraft-suite.json
  ...each app...
```

Existing files are not moved. The project library still indexes legacy files, but skips managed Assets, Exports and Plugins folders. The default launch working directory is the app's Projects folder. Windows/app file dialogs can remember a different folder, so the working directory alone is not a guaranteed save destination.

EffectCraft's supported `export.defaultOutputFolder` preference is set to Exports before launch while the app is closed. Other apps still need native integration to distinguish project saves from exports.

## Built-in bridge: current limits and app-side work

**A universal save-routing plugin cannot run inside the current sandboxed plugin APIs.** These plugins process pixels or objects and have no filesystem/network imports or file-dialog hooks. Master Suite does not inject code, intercept saves, or move files after saving.

The built-in workspace bridge creates a versioned configuration file and passes `ARTCRAFT_SUITE_CONFIG`, `ARTCRAFT_APP_ID`, `ARTCRAFT_PROJECTS_DIR`, `ARTCRAFT_EXPORTS_DIR`, `ARTCRAFT_ASSETS_DIR`, and `ARTCRAFT_PLUGINS_DIR` when launching an app. This is a one-way configuration contract, not a live IPC connection, and current upstream apps do not automatically consume it.

The app-side helper is [`master_suite_bridge.rs`](../ArtCraftLauncher-Source/integrations/master_suite_bridge.rs). To complete save routing, each app must include it in its native host, read `Workspace::from_launcher(app_id)`, and pass `save_directory(current_document)` or `export_directory()` to the appropriate file dialog. Existing-document Save must retain its existing path. Standalone launches retain normal app defaults. Asset/plugin discovery can use the other two paths when the app supports those features.

The app-side helper is provided for integration; it has not been installed into or compiled with the upstream Craft app executables. No UI reports a live bridge connection. Changes to those applications and new app builds are required for full cross-app save/export behavior.

## Dependency

WebAssembly structural validation uses wasmparser 0.240.0, vendored with its upstream licenses. The downloaded crate was checked against its Cargo registry checksum before unpacking. This avoids dependency-cache extraction limitations in the build environment.
