# Direct cloud sync

Build **3.4 beta 3** backs up to **Google Drive** or a **Local NAS** share connected through your operating system. Google Drive for desktop is not required. **Dropbox and OneDrive are not available yet.** These features are included in the Windows, Linux and macOS packages for `v3.4.0-beta.3`.

## Connect and sync

1. Click the **cloud icon at the top right** of Master Suite. This is the only entry to the Cloud screen.
2. Under **Storage**, open **Cloud**, then **Google Drive → Connect**. This page also lists Dropbox and OneDrive as not available yet.
3. Complete Google sign-in and consent in your browser, then return to Master Suite.
4. Open **Your files** and select the projects, assets, exports or plugins to upload. Use the app and file-kind filters to narrow the list; **Refresh files** discovers local changes.
5. Click **Sync** in the bottom bar. Keep Master Suite running until the bottom-right notification says **Sync complete**. The notification counts down five seconds and closes.

The Google Drive button shows **Connecting…** during browser sign-in and **Connected** only after the saved credential successfully refreshes and Google confirms access to the expected Drive account. Saved connections are checked again at startup, when returning to Cloud after 30 seconds, and every five minutes while Cloud is open. **Check connection** runs the check immediately. A failed check or sign-in displays a persistent error inside the Google Drive card and disables syncing until the connection is verified again.

The current desktop client belongs to the `cloud-storage-511200` project. Connections created using an earlier client must reconnect; old tokens and cached account labels are not accepted as proof of a current connection.

The progress bar advances as Google acknowledges each uploaded chunk. Completion requires Google's file size and SHA-256 checksum to match the prepared file. Errors and cancellations have their own outcome; a local copy is never reported as a completed cloud upload.

**Automatically sync changes to selected files** is optional and off by default. It checks for changes while Master Suite is running, including when minimized to the tray. No uploads run after Master Suite has fully quit.

## Files and saved versions

- Backups mirror the local workspace inside **ArtCraft Master Suite**, using the same folder names and preserving nested subfolders:

  ```text
  ArtCraft Master Suite/
    PhotoCraft/
      Projects/Client job/document.psd
      Assets/Textures/paper.png
      Exports/finished.png
      Plugins/plugin-file
    VectorCraft/
      Projects/illustration.vectorcraft
      Assets/reference.png
  ```

- The containing workspace determines the app and kind. A PNG in `FilmCraft/Assets` stays in `FilmCraft/Assets`. Older files directly in an app folder go into that app's `Projects` folder. Projects in other connected roots use their detected app and preserve subfolders relative to the most specific connected root.
- The destination folder is shown beneath every file in Cloud. Hidden/internal folders, symlinks and backup folders are excluded from additional workspace discovery. The picker shows up to 10,000 workspace files with a notice if the limit is reached.
- Google Drive keeps each changed file as a separate version with its original filename. Identical content for the same source reuses a verified version and moves it into the correct subfolder on the next sync. Older versions are kept; versions that are not synced again are not relocated automatically.
- **Saved versions → Refresh saved versions** reads the selected destination's library. Google Drive includes versions uploaded from another computer using this Google client.
- **Restore copy** downloads a version and checks its size and checksum. Choose a new destination filename; existing local files are never overwritten.
- Only selected files are backed up. Existing files in your local app's `Assets`, `Exports`, and `Plugins` folders are selectable, but linked assets elsewhere are not included automatically. The separate app-workspace asset manager remains a placeholder.
- Disconnecting removes the local credential and disables automatic sync. Remote files remain. You can also revoke access in your Google Account's connected-app settings.
- Existing backups created by the older sync-folder feature remain on disk and are not migrated, deleted, or uploaded automatically. Its automatic folder-copy process is no longer started by the launcher.

## Local NAS backups

1. Connect the NAS share in your operating system's file manager first (a Windows network folder/mapped drive, or a mounted share on macOS/Linux). Use the operating system to sign in to the share.
2. Open the top-right **cloud icon → Storage → Local NAS → Connect NAS** and choose a writable folder on that share.
3. Master Suite checks that the folder can be written and read, then creates its **ArtCraft Master Suite** backup folder. A successful connection selects **Local NAS** as the destination. No Google account is required.
4. Select files in **Your files** and click **Sync**. The bottom bar identifies the destination; the bottom-right notification reports copying, verification, completion and its five-second dismissal countdown.
5. Use **Saved versions** to restore a separate copy. Use **Check connection**, **Change folder**, or **Disconnect** in the NAS card to manage the share.

NAS backups keep the latest files at their normal app paths and retain older copies in `.artcraft-versions`. Each saved copy is read back and checked against its SHA-256 checksum before completion. Interrupted writes use temporary files, and a previous backup is retained before replacement. Keep the hidden version records if you want to restore older copies. Two selected files with the same NAS destination must be renamed or placed in separate subfolders.

An identity marker prevents a missing network mount from silently being replaced by a new local backup directory. If the share is offline, read-only, full, or unavailable, the app reports an error. NAS credentials remain with the operating system. Disconnecting keeps the backup files. Only one Master Suite instance can write to a NAS backup folder at a time; after a crash, remove a leftover `.artcraft-sync.lock` only after closing all instances using that folder.

Google Drive and NAS have separate backup receipts and histories. Choose **Use Google Drive for backups** or **Use Local NAS for backups** on the corresponding **Storage → Cloud** or **Storage → Local NAS** page. Opening a storage page only changes the view. Changing destinations turns automatic syncing off; enable it again for the chosen destination if wanted. Automatic backups refresh local files every two minutes while Master Suite is running, even when automatic project scanning is disabled. NAS shares must stay mounted and writable.

## Google project configuration

The desktop client ID configured in the source is:

`500269938170-gn17tokb63uivgok2dtcu10n7tg335ei.apps.googleusercontent.com`

The project owner must enable **Google Drive API**, configure Google Auth Platform for **External** users, and permit the `https://www.googleapis.com/auth/drive.file` scope. During Google's **Testing** stage, add the accounts that will connect under **Audience → Test users**. Google test connections can expire after seven days and require reconnection. Public distribution requires completing Google's applicable publishing requirements.

The client uses browser sign-in, PKCE, a random loopback callback port, and a checked state token. It only requests `drive.file`, allowing access to files created or explicitly shared with this application.

Some Google desktop client configurations require the optional client secret from the downloaded **Desktop app** JSON. If Google reports that it is missing, place the downloaded JSON at `<Master Suite app-data>/cloud/google-desktop-client.json`. The client ID in that file must match the one above. Release builders can instead provide `ARTCRAFT_GOOGLE_DESKTOP_CLIENT_SECRET` when compiling. Do not use a Web application or service-account credential, or commit downloaded credentials to the repository.

Refresh tokens are stored in **Windows Credential Manager**, **macOS Keychain**, or the **Linux Secret Service**. If the credential store is unavailable or locked, sign-in fails with an actionable message rather than saving tokens to plaintext preferences. Linux needs a running Secret Service such as GNOME Keyring or a compatible wallet; the Flatpak manifest permits access to `org.freedesktop.secrets`.

## Implementation references

- [Google desktop OAuth](https://developers.google.com/identity/protocols/oauth2/native-app)
- [Drive file permission](https://developers.google.com/workspace/drive/api/guides/api-specific-auth)
- [Resumable uploads](https://developers.google.com/workspace/drive/api/guides/manage-uploads)

The integration still needs a signed-in upload/restore check against the configured Google project and a backup/restore check against a real NAS share. No account sign-in, upload, or NAS connection is performed while editing the source.
