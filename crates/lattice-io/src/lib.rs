//! File I/O for Lattice spreadsheets.
//!
//! Supports reading and writing `.xlsx`, `.csv`, and JSON export.

pub mod csv_io;
pub mod format_detect;
pub mod json_export;
pub mod recent_files;
pub mod tsv_io;
pub mod watcher;
pub mod xlsx_reader;
pub mod xlsx_writer;

use thiserror::Error;

/// Errors produced by lattice-io operations.
#[derive(Debug, Error)]
pub enum IoError {
    /// Standard I/O error (wraps `std::io::Error`).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Error reading an xlsx file via calamine.
    #[error("xlsx read error: {0}")]
    XlsxRead(String),

    /// Error writing an xlsx file via rust_xlsxwriter.
    #[error("xlsx write error: {0}")]
    XlsxWrite(String),

    /// CSV/TSV parsing or writing error.
    #[error("csv error: {0}")]
    Csv(String),

    /// JSON serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Unsupported or unrecognised file format.
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    /// The file does not exist.
    #[error("file not found: {0}")]
    FileNotFound(String),

    /// Permission denied when accessing a file.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// The file appears to be corrupt or invalid.
    #[error("corrupt file: {0}")]
    CorruptFile(String),

    /// The file was modified externally since it was last read or saved.
    #[error("conflict detected: file was modified externally")]
    ConflictDetected,

    /// Core engine error (e.g. sheet not found).
    #[error("core error: {0}")]
    Core(#[from] lattice_core::LatticeError),
}

/// Convenience result type for lattice-io.
pub type Result<T> = std::result::Result<T, IoError>;

// Re-exports for convenience.
pub use csv_io::{read_csv, write_csv};
pub use format_detect::{FileFormat, detect_format};
pub use json_export::{export_json, export_range_json};
pub use recent_files::{RecentFile, RecentFileStore};
pub use tsv_io::{read_tsv, write_tsv};
pub use watcher::FileWatcher;
pub use xlsx_reader::read_xlsx;
pub use xlsx_writer::write_xlsx;
