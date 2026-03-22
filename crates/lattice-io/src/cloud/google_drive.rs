//! Google Drive cloud provider (stub).
//!
//! This module provides the [`GoogleDriveProvider`] which implements
//! [`CloudProvider`] but currently returns errors for all operations because
//! the OAuth flow and API calls are not yet wired up.
//!
//! # Required Setup (for future implementation)
//!
//! 1. Create a Google Cloud project at <https://console.cloud.google.com/>.
//! 2. Enable the **Google Drive API**.
//! 3. Create an **OAuth 2.0 Client ID** (type: Desktop application).
//! 4. Required scopes:
//!    - `https://www.googleapis.com/auth/drive.file` -- per-file access
//!      (preferred, least-privilege)
//!    - `https://www.googleapis.com/auth/drive.readonly` -- list all files
//! 5. Implement the OAuth Authorization Code flow with PKCE:
//!    - Open the auth URL in the user's browser.
//!    - Listen on a local loopback address for the redirect.
//!    - Exchange the authorization code for access + refresh tokens.
//!    - Store tokens securely in the macOS Keychain.
//! 6. Use the access token to call the Drive v3 REST API:
//!    - `GET /drive/v3/files` with `q` filter for spreadsheet MIME types.
//!    - `GET /drive/v3/files/{id}?alt=media` to download.
//!    - `POST /upload/drive/v3/files` to upload.

use std::path::{Path, PathBuf};

use super::{CloudFile, CloudProvider};
use crate::{IoError, Result};

/// Google Drive cloud storage provider.
///
/// Currently a stub -- all operations return
/// [`IoError::CloudNotConfigured`] until the OAuth flow and REST API
/// client are implemented.
pub struct GoogleDriveProvider {
    /// Whether the provider has been authenticated with a valid OAuth token.
    authenticated: bool,
}

impl GoogleDriveProvider {
    /// Create a new unauthenticated Google Drive provider.
    pub fn new() -> Self {
        Self {
            authenticated: false,
        }
    }

    /// Placeholder for the OAuth authentication flow.
    ///
    /// In a full implementation this would exchange a code/token obtained via
    /// the browser-based OAuth flow and store the resulting credentials.
    pub fn authenticate(&mut self, _token: &str) {
        // TODO: validate the token against Google's token-info endpoint,
        // store access + refresh tokens in the macOS Keychain.
        self.authenticated = false; // remain unauthenticated until real impl
    }
}

impl Default for GoogleDriveProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudProvider for GoogleDriveProvider {
    fn name(&self) -> &str {
        "Google Drive"
    }

    fn key(&self) -> &str {
        "google_drive"
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn list_files(&self) -> Result<Vec<CloudFile>> {
        Err(IoError::CloudNotConfigured(
            "Google Drive authentication not configured. \
             See docs/CLOUD_SYNC.md for setup instructions."
                .to_string(),
        ))
    }

    fn download(&self, _file_id: &str) -> Result<PathBuf> {
        Err(IoError::CloudNotConfigured(
            "Google Drive authentication not configured. \
             See docs/CLOUD_SYNC.md for setup instructions."
                .to_string(),
        ))
    }

    fn upload(&self, _local_path: &Path, _name: &str) -> Result<CloudFile> {
        Err(IoError::CloudNotConfigured(
            "Google Drive authentication not configured. \
             See docs/CLOUD_SYNC.md for setup instructions."
                .to_string(),
        ))
    }

    fn auth_url(&self) -> Option<String> {
        // TODO: return Google OAuth2 authorization URL with PKCE challenge.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_drive_not_authenticated_by_default() {
        let provider = GoogleDriveProvider::new();
        assert!(!provider.is_authenticated());
        assert_eq!(provider.name(), "Google Drive");
        assert_eq!(provider.key(), "google_drive");
    }

    #[test]
    fn test_google_drive_list_files_returns_error() {
        let provider = GoogleDriveProvider::new();
        let result = provider.list_files();
        assert!(result.is_err());
        match result.unwrap_err() {
            IoError::CloudNotConfigured(msg) => {
                assert!(msg.contains("Google Drive"));
            }
            other => panic!("expected CloudNotConfigured, got {:?}", other),
        }
    }

    #[test]
    fn test_google_drive_download_returns_error() {
        let provider = GoogleDriveProvider::new();
        let result = provider.download("some-file-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_google_drive_upload_returns_error() {
        let provider = GoogleDriveProvider::new();
        let result = provider.upload(Path::new("/tmp/test.xlsx"), "test.xlsx");
        assert!(result.is_err());
    }

    #[test]
    fn test_google_drive_auth_url_is_none() {
        let provider = GoogleDriveProvider::new();
        assert!(provider.auth_url().is_none());
    }
}
