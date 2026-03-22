//! File watcher for conflict detection.
//!
//! Tracks the SHA-256 hash of the currently open file and detects
//! external modifications before saving, so the user can be warned
//! about overwriting changes made on another device (e.g. via cloud sync).

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{IoError, Result};

/// Tracks the state of the currently open file for conflict detection.
///
/// Usage:
/// 1. Call [`set_file`](FileWatcher::set_file) when a file is opened.
/// 2. Call [`check_conflict`](FileWatcher::check_conflict) before saving.
/// 3. Call [`update_hash`](FileWatcher::update_hash) after a successful save.
pub struct FileWatcher {
    /// Path to the currently watched file.
    path: Option<PathBuf>,
    /// SHA-256 hash of the file content when last read or saved.
    last_hash: Option<String>,
}

impl FileWatcher {
    /// Create a new watcher with no file tracked.
    pub fn new() -> Self {
        Self {
            path: None,
            last_hash: None,
        }
    }

    /// Start watching a file. Computes and stores its current hash.
    ///
    /// If the file does not exist yet (new unsaved file), the path is stored
    /// but no hash is recorded.
    pub fn set_file(&mut self, path: &Path) -> Result<()> {
        self.path = Some(path.to_path_buf());
        if path.exists() {
            self.last_hash = Some(Self::compute_hash(path)?);
        } else {
            self.last_hash = None;
        }
        Ok(())
    }

    /// Compute the SHA-256 hash of a file at the given path.
    ///
    /// Reads the file in 8 KiB chunks to support large files without
    /// loading the entire file into memory.
    pub fn compute_hash(path: &Path) -> Result<String> {
        if !path.exists() {
            return Err(IoError::FileNotFound(path.display().to_string()));
        }

        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buf)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buf[..bytes_read]);
        }

        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Check whether the currently watched file has been modified externally.
    ///
    /// Returns `true` if the file's current hash differs from the stored hash,
    /// indicating the file was changed since we last read or saved it.
    ///
    /// Returns `false` if:
    /// - The hashes match (no external modification).
    /// - No file is being watched.
    /// - No hash was stored (new file that doesn't exist on disk yet).
    pub fn check_conflict(&self) -> Result<bool> {
        let path = match &self.path {
            Some(p) => p,
            None => return Ok(false),
        };

        let stored_hash = match &self.last_hash {
            Some(h) => h,
            None => return Ok(false),
        };

        if !path.exists() {
            // File was deleted externally -- that is a conflict.
            return Ok(true);
        }

        let current_hash = Self::compute_hash(path)?;
        Ok(&current_hash != stored_hash)
    }

    /// Update the stored hash to match the file's current state.
    ///
    /// Call this after a successful save so the next `check_conflict()`
    /// compares against the newly saved content.
    pub fn update_hash(&mut self) -> Result<()> {
        let path = match &self.path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        if path.exists() {
            self.last_hash = Some(Self::compute_hash(&path)?);
        }
        Ok(())
    }

    /// Return the path of the currently watched file, if any.
    pub fn watched_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Return the last stored hash, if any.
    pub fn last_hash(&self) -> Option<&str> {
        self.last_hash.as_deref()
    }

    /// Clear the watcher state (stop watching any file).
    pub fn clear(&mut self) {
        self.path = None;
        self.last_hash = None;
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_watcher_no_conflict() {
        let watcher = FileWatcher::new();
        assert!(!watcher.check_conflict().unwrap());
        assert!(watcher.watched_path().is_none());
        assert!(watcher.last_hash().is_none());
    }

    #[test]
    fn test_set_file_and_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();

        let mut watcher = FileWatcher::new();
        watcher.set_file(&path).unwrap();

        assert_eq!(watcher.watched_path(), Some(path.as_path()));
        assert!(watcher.last_hash().is_some());
        assert!(!watcher.check_conflict().unwrap());
    }

    #[test]
    fn test_detect_external_modification() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.txt");
        std::fs::write(&path, "original content").unwrap();

        let mut watcher = FileWatcher::new();
        watcher.set_file(&path).unwrap();

        // Simulate external modification.
        std::fs::write(&path, "modified content").unwrap();

        assert!(watcher.check_conflict().unwrap());
    }

    #[test]
    fn test_no_conflict_after_update_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.txt");
        std::fs::write(&path, "original").unwrap();

        let mut watcher = FileWatcher::new();
        watcher.set_file(&path).unwrap();

        // Simulate external modification.
        std::fs::write(&path, "changed").unwrap();
        assert!(watcher.check_conflict().unwrap());

        // Update hash (as if we saved or acknowledged the change).
        watcher.update_hash().unwrap();
        assert!(!watcher.check_conflict().unwrap());
    }

    #[test]
    fn test_conflict_when_file_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.txt");
        std::fs::write(&path, "content").unwrap();

        let mut watcher = FileWatcher::new();
        watcher.set_file(&path).unwrap();

        std::fs::remove_file(&path).unwrap();
        assert!(watcher.check_conflict().unwrap());
    }

    #[test]
    fn test_set_file_nonexistent() {
        let mut watcher = FileWatcher::new();
        watcher
            .set_file(Path::new("/tmp/lattice_nonexistent_file.txt"))
            .unwrap();
        assert!(watcher.last_hash().is_none());
        assert!(!watcher.check_conflict().unwrap());
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hash_test.txt");
        std::fs::write(&path, "deterministic content").unwrap();

        let hash1 = FileWatcher::compute_hash(&path).unwrap();
        let hash2 = FileWatcher::compute_hash(&path).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_file_not_found() {
        let result = FileWatcher::compute_hash(Path::new("/nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_watcher() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.txt");
        std::fs::write(&path, "content").unwrap();

        let mut watcher = FileWatcher::new();
        watcher.set_file(&path).unwrap();
        assert!(watcher.watched_path().is_some());

        watcher.clear();
        assert!(watcher.watched_path().is_none());
        assert!(watcher.last_hash().is_none());
        assert!(!watcher.check_conflict().unwrap());
    }
}
