//! Cloud storage provider abstraction.
//!
//! Defines the [`CloudProvider`] trait that all cloud storage backends implement,
//! plus the shared [`CloudFile`] type used to represent files listed from any
//! provider.
//!
//! # Providers
//!
//! - [`google_drive::GoogleDriveProvider`] -- API-based (requires OAuth setup).
//! - [`icloud::ICloudProvider`] -- Filesystem-based (works out of the box on macOS).
//! - [`dropbox::DropboxProvider`] -- API-based (requires OAuth setup).

pub mod dropbox;
pub mod google_drive;
pub mod icloud;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Result;

/// Metadata for a file stored in a cloud provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFile {
    /// Provider-specific unique identifier for the file.
    /// For iCloud this is the absolute path; for API providers it is the
    /// remote file ID.
    pub id: String,
    /// Human-readable file name (e.g. `"Budget 2026.xlsx"`).
    pub name: String,
    /// Last-modified timestamp in ISO 8601 format.
    pub modified: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Name of the provider this file belongs to (e.g. `"Google Drive"`).
    pub provider: String,
}

/// Summary information about a cloud provider's status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Machine-readable provider key (e.g. `"google_drive"`).
    pub key: String,
    /// Display name (e.g. `"Google Drive"`).
    pub name: String,
    /// Whether the provider is authenticated and ready.
    pub authenticated: bool,
}

/// Trait for cloud storage providers.
///
/// Each provider must be `Send + Sync` so it can be held in shared Tauri state.
/// Methods are synchronous because actual network calls in the API-based
/// providers are not yet implemented (they are stubs). When a provider is
/// implemented for real it may make sense to introduce an async variant.
pub trait CloudProvider: Send + Sync {
    /// Display name (e.g. `"Google Drive"`).
    fn name(&self) -> &str;

    /// Machine-readable key (e.g. `"google_drive"`).
    fn key(&self) -> &str;

    /// Whether the provider is configured and authenticated.
    fn is_authenticated(&self) -> bool;

    /// List spreadsheet files (`.xlsx`, `.csv`) in the user's cloud storage.
    fn list_files(&self) -> Result<Vec<CloudFile>>;

    /// Download a file to a local path and return that path.
    ///
    /// For filesystem-based providers (iCloud) this may simply return the
    /// existing path without copying.
    fn download(&self, file_id: &str) -> Result<PathBuf>;

    /// Upload a local file to the cloud provider.
    ///
    /// `local_path` is the file on disk; `name` is the desired remote
    /// filename (e.g. `"Budget.xlsx"`).
    fn upload(&self, local_path: &Path, name: &str) -> Result<CloudFile>;

    /// Return a URL the user can open in a browser to start OAuth
    /// authentication. Returns `None` for providers that do not need OAuth
    /// (e.g. iCloud, which is filesystem-based).
    fn auth_url(&self) -> Option<String>;
}
