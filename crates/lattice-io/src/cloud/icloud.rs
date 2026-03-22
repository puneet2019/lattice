//! iCloud Drive cloud provider.
//!
//! On macOS, iCloud Drive files are stored locally at
//! `~/Library/Mobile Documents/com~apple~CloudDocs/`. This provider simply
//! reads from and writes to that directory, making it fully functional without
//! any OAuth flow or API keys.
//!
//! The macOS iCloud daemon (`bird`) handles syncing transparently.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{CloudFile, CloudProvider};
use crate::{IoError, Result};

/// The well-known iCloud Drive path on macOS.
const ICLOUD_DRIVE_SUBPATH: &str = "Library/Mobile Documents/com~apple~CloudDocs";

/// Spreadsheet file extensions we list from iCloud Drive.
const SUPPORTED_EXTENSIONS: &[&str] = &["xlsx", "csv", "tsv", "xls", "ods"];

/// iCloud Drive cloud storage provider.
///
/// Works by scanning the local iCloud Drive folder for spreadsheet files.
/// No authentication is needed -- if iCloud Drive is enabled on the Mac,
/// files are already present on the local filesystem.
pub struct ICloudProvider {
    /// Root path of the iCloud Drive folder.
    root: PathBuf,
}

impl ICloudProvider {
    /// Create a new iCloud provider pointing at the default iCloud Drive path.
    ///
    /// Returns an unauthenticated provider if the iCloud Drive folder does not
    /// exist (iCloud is not signed in or not enabled).
    pub fn new() -> Self {
        let home = dirs_path();
        let root = home.join(ICLOUD_DRIVE_SUBPATH);
        Self { root }
    }

    /// Create a provider pointing at a custom root directory.
    ///
    /// Useful for testing without depending on the real iCloud Drive folder.
    #[cfg(test)]
    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// Recursively scan a directory for spreadsheet files.
    fn scan_dir(&self, dir: &Path, results: &mut Vec<CloudFile>) -> Result<()> {
        let entries = fs::read_dir(dir).map_err(|e| {
            IoError::Io(std::io::Error::new(
                e.kind(),
                format!("failed to read iCloud directory {}: {}", dir.display(), e),
            ))
        })?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Skip hidden files and directories (e.g. .Trash, .DS_Store).
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }

            if path.is_dir() {
                self.scan_dir(&path, results)?;
            } else if is_spreadsheet(&path) {
                if let Some(cf) = self.path_to_cloud_file(&path)? {
                    results.push(cf);
                }
            }
        }

        Ok(())
    }

    /// Convert a filesystem path inside iCloud Drive into a [`CloudFile`].
    fn path_to_cloud_file(&self, path: &Path) -> Result<Option<CloudFile>> {
        let metadata = fs::metadata(path)?;

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let modified = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| {
                // Format as ISO 8601. We use a simple seconds-since-epoch
                // conversion here; a full datetime library is not warranted
                // for this metadata field.
                let secs = d.as_secs();
                format!("{}Z", secs)
            })
            .unwrap_or_else(|_| "0Z".to_string());

        Ok(Some(CloudFile {
            id: path.to_string_lossy().to_string(),
            name,
            modified,
            size_bytes: metadata.len(),
            provider: "iCloud Drive".to_string(),
        }))
    }
}

impl Default for ICloudProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudProvider for ICloudProvider {
    fn name(&self) -> &str {
        "iCloud Drive"
    }

    fn key(&self) -> &str {
        "icloud"
    }

    fn is_authenticated(&self) -> bool {
        // iCloud is "authenticated" if the iCloud Drive folder exists.
        self.root.is_dir()
    }

    fn list_files(&self) -> Result<Vec<CloudFile>> {
        if !self.root.is_dir() {
            return Err(IoError::CloudNotConfigured(
                "iCloud Drive folder not found. \
                 Sign in to iCloud in System Settings and enable iCloud Drive."
                    .to_string(),
            ));
        }

        let mut files = Vec::new();
        self.scan_dir(&self.root, &mut files)?;

        // Sort by name for deterministic output.
        files.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(files)
    }

    fn download(&self, file_id: &str) -> Result<PathBuf> {
        // For iCloud, the file_id IS the local path.
        let path = PathBuf::from(file_id);
        if !path.exists() {
            return Err(IoError::FileNotFound(file_id.to_string()));
        }
        Ok(path)
    }

    fn upload(&self, local_path: &Path, name: &str) -> Result<CloudFile> {
        if !self.root.is_dir() {
            return Err(IoError::CloudNotConfigured(
                "iCloud Drive folder not found. \
                 Sign in to iCloud in System Settings and enable iCloud Drive."
                    .to_string(),
            ));
        }

        if !local_path.exists() {
            return Err(IoError::FileNotFound(local_path.display().to_string()));
        }

        let dest = self.root.join(name);

        // Use atomic save: write to temp then rename, so the iCloud daemon
        // never picks up a partially-written file.
        crate::atomic::save_atomic(&dest, &fs::read(local_path)?)?;

        self.path_to_cloud_file(&dest)?.ok_or_else(|| {
            IoError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "failed to read back uploaded file metadata",
            ))
        })
    }

    fn auth_url(&self) -> Option<String> {
        // iCloud does not use OAuth -- it is configured via macOS System Settings.
        None
    }
}

/// Return the user's home directory.
fn dirs_path() -> PathBuf {
    // On macOS, HOME is always set. Fall back to /tmp if somehow missing.
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Check if a path has a spreadsheet file extension.
fn is_spreadsheet(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            let lower = ext.to_lowercase();
            SUPPORTED_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icloud_provider_with_nonexistent_root() {
        let provider = ICloudProvider::with_root(PathBuf::from("/nonexistent_icloud_test_dir"));
        assert!(!provider.is_authenticated());
        assert_eq!(provider.name(), "iCloud Drive");
        assert_eq!(provider.key(), "icloud");
    }

    #[test]
    fn test_icloud_list_files_no_root() {
        let provider = ICloudProvider::with_root(PathBuf::from("/nonexistent_icloud_test_dir"));
        let result = provider.list_files();
        assert!(result.is_err());
        match result.unwrap_err() {
            IoError::CloudNotConfigured(msg) => {
                assert!(msg.contains("iCloud Drive"));
            }
            other => panic!("expected CloudNotConfigured, got {:?}", other),
        }
    }

    #[test]
    fn test_icloud_list_files_with_temp_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();

        // Create some spreadsheet files.
        fs::write(root.join("Budget.xlsx"), b"fake xlsx").unwrap();
        fs::write(root.join("Data.csv"), b"a,b,c").unwrap();
        fs::write(root.join("Notes.txt"), b"not a spreadsheet").unwrap();
        fs::write(root.join(".DS_Store"), b"hidden").unwrap();

        let provider = ICloudProvider::with_root(root);
        assert!(provider.is_authenticated());

        let files = provider.list_files().unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();

        assert!(names.contains(&"Budget.xlsx"));
        assert!(names.contains(&"Data.csv"));
        assert!(!names.contains(&"Notes.txt"));
        assert!(!names.contains(&".DS_Store"));
    }

    #[test]
    fn test_icloud_list_files_recursive() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let sub = root.join("Projects");
        fs::create_dir(&sub).unwrap();

        fs::write(root.join("Top.xlsx"), b"top").unwrap();
        fs::write(sub.join("Nested.xlsx"), b"nested").unwrap();

        let provider = ICloudProvider::with_root(root);
        let files = provider.list_files().unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();

        assert!(names.contains(&"Top.xlsx"));
        assert!(names.contains(&"Nested.xlsx"));
    }

    #[test]
    fn test_icloud_download_returns_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.xlsx");
        fs::write(&file_path, b"data").unwrap();

        let provider = ICloudProvider::with_root(dir.path().to_path_buf());
        let path_str = file_path.to_string_lossy().to_string();
        let result = provider.download(&path_str).unwrap();
        assert_eq!(result, file_path);
    }

    #[test]
    fn test_icloud_download_file_not_found() {
        let provider = ICloudProvider::with_root(PathBuf::from("/tmp"));
        let result = provider.download("/nonexistent/file.xlsx");
        assert!(result.is_err());
    }

    #[test]
    fn test_icloud_upload() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();

        // Create a source file to upload.
        let src_dir = tempfile::tempdir().unwrap();
        let src = src_dir.path().join("source.xlsx");
        fs::write(&src, b"spreadsheet data").unwrap();

        let provider = ICloudProvider::with_root(root.clone());
        let cloud_file = provider.upload(&src, "Uploaded.xlsx").unwrap();

        assert_eq!(cloud_file.name, "Uploaded.xlsx");
        assert_eq!(cloud_file.provider, "iCloud Drive");

        // Verify the file exists in the iCloud root.
        let dest = root.join("Uploaded.xlsx");
        assert!(dest.exists());
        assert_eq!(fs::read(&dest).unwrap(), b"spreadsheet data");
    }

    #[test]
    fn test_icloud_upload_no_root() {
        let provider = ICloudProvider::with_root(PathBuf::from("/nonexistent_icloud_test_dir"));
        let result = provider.upload(Path::new("/tmp/test.xlsx"), "test.xlsx");
        assert!(result.is_err());
    }

    #[test]
    fn test_icloud_auth_url_is_none() {
        let provider = ICloudProvider::new();
        assert!(provider.auth_url().is_none());
    }

    #[test]
    fn test_is_spreadsheet() {
        assert!(is_spreadsheet(Path::new("test.xlsx")));
        assert!(is_spreadsheet(Path::new("test.csv")));
        assert!(is_spreadsheet(Path::new("test.tsv")));
        assert!(is_spreadsheet(Path::new("test.xls")));
        assert!(is_spreadsheet(Path::new("test.ods")));
        assert!(is_spreadsheet(Path::new("test.XLSX")));
        assert!(!is_spreadsheet(Path::new("test.txt")));
        assert!(!is_spreadsheet(Path::new("test.pdf")));
        assert!(!is_spreadsheet(Path::new("noext")));
    }
}
