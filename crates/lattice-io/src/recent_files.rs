//! Recent files store for Lattice.
//!
//! Tracks recently opened files and persists the list to
//! `~/Library/Application Support/Lattice/recent_files.json`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{IoError, Result};

/// Metadata for a recently opened file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentFile {
    /// Full path to the file.
    pub path: String,
    /// Display name (typically the file name without directory).
    pub name: String,
    /// ISO 8601 timestamp of when the file was last opened.
    pub last_opened: String,
}

/// Manages a list of recently opened files.
///
/// The store is persisted as a JSON array in the application support
/// directory. By default, it holds up to 10 entries.
#[derive(Debug, Clone)]
pub struct RecentFileStore {
    files: Vec<RecentFile>,
    max_entries: usize,
}

impl RecentFileStore {
    /// Default maximum number of recent file entries.
    const DEFAULT_MAX: usize = 10;

    /// Create a new empty store with the default capacity.
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            max_entries: Self::DEFAULT_MAX,
        }
    }

    /// Create a new store with a custom maximum entry count.
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            files: Vec::new(),
            max_entries: max_entries.max(1),
        }
    }

    /// Add (or bump) a file in the recent list.
    ///
    /// If the file is already in the list, it is moved to the front with
    /// an updated timestamp. Otherwise, a new entry is prepended. If the
    /// list exceeds `max_entries`, the oldest entry is dropped.
    pub fn add(&mut self, path: &str, name: &str) {
        // Remove existing entry with the same path (case-sensitive).
        self.files.retain(|f| f.path != path);

        let entry = RecentFile {
            path: path.to_string(),
            name: name.to_string(),
            last_opened: current_iso_timestamp(),
        };

        self.files.insert(0, entry);

        // Trim to max entries.
        if self.files.len() > self.max_entries {
            self.files.truncate(self.max_entries);
        }
    }

    /// Return the list of recent files, most-recently-opened first.
    pub fn list(&self) -> &[RecentFile] {
        &self.files
    }

    /// Remove a specific file from the recent list by path.
    ///
    /// Returns `true` if the file was found and removed.
    pub fn remove(&mut self, path: &str) -> bool {
        let before = self.files.len();
        self.files.retain(|f| f.path != path);
        self.files.len() < before
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.files.clear();
    }

    /// Load the recent files list from the default persist path.
    ///
    /// If the file does not exist, returns an empty store.
    /// If the file is corrupt, logs a warning and returns an empty store.
    pub fn load() -> Self {
        let persist_path = Self::persist_path();
        Self::load_from(&persist_path)
    }

    /// Load from a specific path (useful for testing).
    pub fn load_from(path: &Path) -> Self {
        if !path.exists() {
            return Self::new();
        }

        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<Vec<RecentFile>>(&content) {
                Ok(files) => {
                    let mut store = Self::new();
                    store.files = files;
                    // Trim if saved list exceeds current max.
                    store.files.truncate(store.max_entries);
                    store
                }
                Err(e) => {
                    eprintln!(
                        "warning: could not parse recent files from '{}': {}",
                        path.display(),
                        e
                    );
                    Self::new()
                }
            },
            Err(e) => {
                eprintln!(
                    "warning: could not read recent files from '{}': {}",
                    path.display(),
                    e
                );
                Self::new()
            }
        }
    }

    /// Save the recent files list to the default persist path.
    pub fn save(&self) -> Result<()> {
        let persist_path = Self::persist_path();
        self.save_to(&persist_path)
    }

    /// Save to a specific path (useful for testing).
    pub fn save_to(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self.files)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Return the default persist path for the recent files list.
    ///
    /// On macOS: `~/Library/Application Support/Lattice/recent_files.json`
    fn persist_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Lattice")
            .join("recent_files.json")
    }
}

impl Default for RecentFileStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Return the current time as an ISO 8601 string (UTC, second precision).
///
/// Uses `std::time::SystemTime` to avoid pulling in chrono.
fn current_iso_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Convert Unix timestamp to ISO 8601 date-time.
    let days = secs / 86400;
    let day_secs = secs % 86400;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;

    // Compute year, month, day from days since epoch (1970-01-01).
    let (year, month, day) = days_to_ymd(days as i64);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Hinnant's civil_from_days algorithm.
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_store_is_empty() {
        let store = RecentFileStore::new();
        assert!(store.list().is_empty());
    }

    #[test]
    fn test_add_file() {
        let mut store = RecentFileStore::new();
        store.add("/path/to/file.xlsx", "file.xlsx");
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.list()[0].path, "/path/to/file.xlsx");
        assert_eq!(store.list()[0].name, "file.xlsx");
        assert!(!store.list()[0].last_opened.is_empty());
    }

    #[test]
    fn test_add_duplicate_moves_to_front() {
        let mut store = RecentFileStore::new();
        store.add("/path/a.xlsx", "a.xlsx");
        store.add("/path/b.xlsx", "b.xlsx");
        store.add("/path/a.xlsx", "a.xlsx");

        assert_eq!(store.list().len(), 2);
        assert_eq!(store.list()[0].path, "/path/a.xlsx");
        assert_eq!(store.list()[1].path, "/path/b.xlsx");
    }

    #[test]
    fn test_max_entries_enforced() {
        let mut store = RecentFileStore::with_max_entries(3);
        store.add("/a", "a");
        store.add("/b", "b");
        store.add("/c", "c");
        store.add("/d", "d");

        assert_eq!(store.list().len(), 3);
        assert_eq!(store.list()[0].path, "/d");
        assert_eq!(store.list()[2].path, "/b");
    }

    #[test]
    fn test_remove_file() {
        let mut store = RecentFileStore::new();
        store.add("/a", "a");
        store.add("/b", "b");

        assert!(store.remove("/a"));
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.list()[0].path, "/b");
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut store = RecentFileStore::new();
        store.add("/a", "a");
        assert!(!store.remove("/nonexistent"));
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut store = RecentFileStore::new();
        store.add("/a", "a");
        store.add("/b", "b");
        store.clear();
        assert!(store.list().is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recent.json");

        let mut store = RecentFileStore::new();
        store.add("/path/to/file.xlsx", "file.xlsx");
        store.add("/path/to/other.csv", "other.csv");
        store.save_to(&path).unwrap();

        let loaded = RecentFileStore::load_from(&path);
        assert_eq!(loaded.list().len(), 2);
        assert_eq!(loaded.list()[0].path, "/path/to/other.csv");
        assert_eq!(loaded.list()[1].path, "/path/to/file.xlsx");
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let store = RecentFileStore::load_from(Path::new("/nonexistent/recent.json"));
        assert!(store.list().is_empty());
    }

    #[test]
    fn test_load_corrupt_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recent.json");
        std::fs::write(&path, "not valid json!!!").unwrap();

        let store = RecentFileStore::load_from(&path);
        assert!(store.list().is_empty());
    }

    #[test]
    fn test_timestamp_format() {
        let ts = current_iso_timestamp();
        // Should look like "2024-01-15T10:30:00Z"
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
        assert_eq!(ts.len(), 20);
    }
}
