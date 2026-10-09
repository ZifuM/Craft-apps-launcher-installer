# Cloud backups with synced folders

Master Suite saves versioned project backups into folders managed by Google Drive, Dropbox or OneDrive. The provider desktop app handles sign-in and uploading. No developer registration, client ID or API key is needed.

## Setup

1. Install the provider desktop app and sign in with your own account:
   - [Google Drive for desktop](https://www.google.com/drive/download/)
   - [Dropbox](https://www.dropbox.com/install)
   - [OneDrive](https://www.microsoft.com/microsoft-365/onedrive/download)
2. Finish its setup and locate a folder it syncs. For Google Drive, select a folder inside My Drive on its mounted drive or mirrored folder.
3. In Master Suite, open **Settings → Cloud → Sync folders → Choose sync folder** (or use the top-right Cloud shortcut) for that provider. Select the actual synced folder, not an ordinary local folder.
4. Leave **Use for project backups** enabled for your desired destinations.
5. In **Project files**, select projects and click **Back up now**. Optionally enable **Automatic backup**, which is off by default.

Optional onboarding also offers folder selection. Choosing a folder alone does not back up projects.

## Status and uploads

**Copied locally** confirms a saved project version was copied into the selected folder. It does not confirm a completed upload. Keep the provider desktop app running and check its upload status, connection and storage. Master Suite cannot establish whether an arbitrary selected folder is actually synced.

## Versions and restore

- Distinct file contents are kept separately under `ArtCraft Master Suite/Projects`, grouped by app and source path.
- Previous versions are retained; there is no automatic deletion. Storage usage grows as projects change.
- Refresh **Saved versions** to discover backups, including copies synced from another computer.
- **Restore copy** verifies size and checksum and saves a separate local file. Existing files are never overwritten.
- Online-only backups may need to download through the provider desktop app before restoring.
- Only selected project files are backed up. Linked files and assets are not included; asset backup is a future feature.
- Disconnecting a folder stops Master Suite from using it. Backups remain and the provider stays signed in.

This is versioned backup, not two-way project synchronization. Master Suite does not directly sign in to providers or use previous OAuth connections. Live provider uploads and restores have not been verified.

Provider controls include links for installing or opening the desktop app. Custom installations can also be opened through your operating system. Some clients appear in the system tray or menu bar instead of showing a window. Legacy keeps its separate Cloud page.
