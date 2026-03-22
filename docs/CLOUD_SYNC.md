# Cloud Sync Compatibility

Lattice is designed to work seamlessly with cloud sync services like Google Drive, iCloud Drive, and Dropbox. No special configuration is needed -- save your file to a synced folder and it works.

## How It Works

### Single-File Format

Lattice uses a single `.xlsx` file for each workbook. There are no sidecar files, no lock files, and no temp files placed alongside the workbook. Cloud sync services monitor folder contents for changes, and a single-file format means there is exactly one file to sync.

### Atomic Writes

When Lattice saves a file, it uses a three-step atomic write strategy:

1. **Write to temp file** -- The workbook is serialized to a temporary file in the OS temp directory (`/tmp` on macOS), not in the user's document folder.
2. **Flush and sync** -- The temp file is flushed and `fsync`'d to ensure data is written to disk.
3. **Rename** -- The temp file is renamed to the target path. On POSIX systems (macOS, Linux), `rename()` is atomic, meaning the file transitions from old content to new content in a single operation.

This prevents cloud sync clients from uploading a partially-written file. The sync client either sees the old file or the new file, never a half-written one.

### Conflict Detection

Lattice tracks a SHA-256 hash of the file content when it is opened or saved. Before saving, it re-hashes the file on disk and compares:

- **No change** -- Save proceeds normally.
- **File changed externally** -- Lattice warns the user that the file was modified (e.g., by another device syncing via cloud) and asks whether to overwrite or reload.
- **File deleted externally** -- Treated as a conflict and reported to the user.

### Auto-Save

When enabled (default: on, interval: 60 seconds), auto-save writes the workbook to its current file path using the same atomic write strategy. The auto-save interval is configurable (minimum 5 seconds).

## Supported Cloud Services

| Service | Status | Notes |
|---------|--------|-------|
| Google Drive | Supported | Desktop client syncs single files without issues |
| iCloud Drive | Supported | macOS-native; atomic renames work correctly |
| Dropbox | Supported | Desktop client handles single-file sync well |
| OneDrive | Supported | Standard sync behavior |

## Known Limitations

1. **No real-time collaboration.** Cloud sync provides file-level sync, not cell-level collaboration. If two people edit the same file simultaneously on different devices, the last save wins. Real-time collaboration via CRDT is planned for a future release.

2. **Large files may take time to sync.** Cloud services sync the entire file on each save. A 50MB workbook will upload 50MB on every save. This is inherent to the single-file approach.

3. **Sync conflicts are last-write-wins.** If the cloud service itself detects a conflict (e.g., two devices saved before syncing), it may create a "conflicted copy" file. Lattice does not automatically merge conflicted copies.

4. **Cross-device rename fallback.** If the OS temp directory is on a different filesystem than the save target (unusual on macOS but possible on Linux), the atomic rename falls back to a copy operation, which is briefly non-atomic.

## Troubleshooting

- **File not syncing after save:** Verify the file is saved inside the cloud service's sync folder. Lattice writes to wherever you choose in the Save dialog.
- **"File was modified externally" warning on every save:** Another application or sync client may be modifying the file. Check for other programs that auto-open `.xlsx` files.
- **Sync client showing temp files:** Lattice writes temp files to the OS temp directory, not your sync folder. If you see `.lattice-tmp-*` files in your sync folder, please file a bug report.
