//! Dropbox cloud provider (stub).
//!
//! This module provides the [`DropboxProvider`] which implements
//! [`CloudProvider`] but currently returns errors for all operations because
//! the OAuth flow and API calls are not yet wired up.
//!
//! # Required Setup (for future implementation)
//!
//! 1. Create a Dropbox App at <https://www.dropbox.com/developers/apps>.
//! 2. Choose **Scoped access** with **Full Dropbox** or **App folder** access.
//! 3. Required permissions (scopes):
//!    - `files.metadata.read` -- list files
//!    - `files.content.read` -- download files
//!    - `files.content.write` -- upload files
//! 4. Implement the OAuth 2.0 Authorization Code flow with PKCE:
//!    - Open `https://www.dropbox.com/oauth2/authorize` in the user's browser.
//!    - Listen on a local loopback address for the redirect.
//!    - Exchange the authorization code for an access token.
//!    - Store the token securely in the macOS Keychain.
//! 5. Use the access token to call the Dropbox HTTP API:
//!    - `POST /2/files/list_folder` to list files.
//!    - `POST /2/files/download` (via content endpoint) to download.
//!    - `POST /2/files/upload` (via content endpoint) to upload.

use std::path::{Path, PathBuf};

use super::{CloudFile, CloudProvider};
use crate::{IoError, Result};

/// Dropbox cloud storage provider.
///
/// Currently a stub -- all operations return [`IoError::CloudNotConfigured`]
/// until the OAuth flow and REST API client are implemented.
pub struct DropboxProvider {
    /// Whether the provider has been authenticated with a valid OAuth token.
    authenticated: bool,
}

impl DropboxProvider {
    /// Create a new unauthenticated Dropbox provider.
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
        // TODO: validate the token against Dropbox's token endpoint,
        // store the access token in the macOS Keychain.
        self.authenticated = false; // remain unauthenticated until real impl
    }
}

impl Default for DropboxProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudProvider for DropboxProvider {
    fn name(&self) -> &str {
        "Dropbox"
    }

    fn key(&self) -> &str {
        "dropbox"
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn list_files(&self) -> Result<Vec<CloudFile>> {
        Err(IoError::CloudNotConfigured(
            "Dropbox authentication not configured. \
             See docs/CLOUD_SYNC.md for setup instructions."
                .to_string(),
        ))
    }

    fn download(&self, _file_id: &str) -> Result<PathBuf> {
        Err(IoError::CloudNotConfigured(
            "Dropbox authentication not configured. \
             See docs/CLOUD_SYNC.md for setup instructions."
                .to_string(),
        ))
    }

    fn upload(&self, _local_path: &Path, _name: &str) -> Result<CloudFile> {
        Err(IoError::CloudNotConfigured(
            "Dropbox authentication not configured. \
             See docs/CLOUD_SYNC.md for setup instructions."
                .to_string(),
        ))
    }

    fn auth_url(&self) -> Option<String> {
        // TODO: return Dropbox OAuth2 authorization URL with PKCE challenge.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dropbox_not_authenticated_by_default() {
        let provider = DropboxProvider::new();
        assert!(!provider.is_authenticated());
        assert_eq!(provider.name(), "Dropbox");
        assert_eq!(provider.key(), "dropbox");
    }

    #[test]
    fn test_dropbox_list_files_returns_error() {
        let provider = DropboxProvider::new();
        let result = provider.list_files();
        assert!(result.is_err());
        match result.unwrap_err() {
            IoError::CloudNotConfigured(msg) => {
                assert!(msg.contains("Dropbox"));
            }
            other => panic!("expected CloudNotConfigured, got {:?}", other),
        }
    }

    #[test]
    fn test_dropbox_download_returns_error() {
        let provider = DropboxProvider::new();
        let result = provider.download("some-file-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_dropbox_upload_returns_error() {
        let provider = DropboxProvider::new();
        let result = provider.upload(Path::new("/tmp/test.xlsx"), "test.xlsx");
        assert!(result.is_err());
    }

    #[test]
    fn test_dropbox_auth_url_is_none() {
        let provider = DropboxProvider::new();
        assert!(provider.auth_url().is_none());
    }
}
