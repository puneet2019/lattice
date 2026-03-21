---
name: sync-check
description: Verify cloud sync compatibility — test file saving behavior with Google Drive, Dropbox, iCloud
---

# /sync-check

Verify that Lattice files are cloud sync compatible.

## Steps

1. Check for lock files or temp files in save directory:
   ```bash
   cd /Users/puneetmahajan/GolandProjects/lattice
   # After saving a test file, check the directory
   ls -la /tmp/lattice-sync-test/
   # Should contain ONLY the .xlsx file, no .tmp, .lock, .~lock, etc.
   ```

2. Verify atomic write behavior:
   ```bash
   # Run the atomic write test
   cargo test -p lattice-io test_atomic_write 2>&1
   ```

3. Verify file watcher detects external changes:
   ```bash
   cargo test -p lattice-io test_external_modification_detection 2>&1
   ```

4. Check file format is single-file (no folder dependencies):
   ```bash
   file /tmp/lattice-sync-test/test.xlsx
   # Should be: Microsoft Excel 2007+ (zip archive)
   ```

5. Test conflict detection:
   ```bash
   cargo test -p lattice-io test_conflict_detection 2>&1
   ```

## Cloud Sync Compatibility Checklist
- [ ] No lock files created in save directory
- [ ] No temp files left in save directory
- [ ] Writes are atomic (temp → rename)
- [ ] External file changes detected
- [ ] Conflict warning shown when file modified externally
- [ ] File is a single .xlsx (standard zip-based OOXML)
- [ ] File can be uploaded to Google Drive and opened in Google Sheets
- [ ] File can be downloaded from Google Drive and opened in Lattice
