//! File I/O for Lattice spreadsheets.
//!
//! Supports reading and writing `.xlsx`, `.csv`, `.tsv`, `.xls`, `.ods`,
//! and JSON export.

pub mod csv_io;
pub mod format_detect;
pub mod json_export;
pub mod pdf_export;
pub mod tsv_io;
pub mod xlsx_chart_parser;
pub mod xlsx_chart_reader;
pub mod xlsx_reader;
pub mod xlsx_writer;

// Filesystem-bound modules. These do direct `std::fs` I/O, atomic
// temp-then-rename writes, file watching, or pull in native-only crates
// (`notify`, `libc`). They are unavailable in a `--no-default-features`
// (WASM) build, which works purely with in-memory buffers and strings.
#[cfg(feature = "native")]
pub mod atomic;
#[cfg(feature = "native")]
pub mod cloud;
#[cfg(feature = "native")]
pub mod file_info;
#[cfg(feature = "native")]
pub mod recent_files;
#[cfg(feature = "native")]
pub mod watcher;

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

    /// Cloud provider is not authenticated or not configured.
    #[error("cloud provider not configured: {0}")]
    CloudNotConfigured(String),

    /// Core engine error (e.g. sheet not found).
    #[error("core error: {0}")]
    Core(#[from] lattice_core::LatticeError),
}

/// Convenience result type for lattice-io.
pub type Result<T> = std::result::Result<T, IoError>;

// Re-exports for convenience.
//
// WASM-available (buffer/string-based, no filesystem):
pub use csv_io::{read_csv_str, write_csv_string};
pub use format_detect::{FileFormat, detect_format_from_bytes};
pub use json_export::{export_json, export_range_json};
pub use pdf_export::{PrintSettings, export_print_html};
pub use tsv_io::{read_tsv_str, write_tsv_string};
pub use xlsx_chart_reader::{ImportedChart, read_xlsx_charts_from_bytes};
pub use xlsx_reader::{read_xlsx_from_bytes, read_xls_from_bytes, read_ods_from_bytes};
pub use xlsx_writer::write_xlsx_to_buffer;

// Native-only (filesystem path-based, atomic saves, watcher, cloud).
#[cfg(feature = "native")]
pub use atomic::{save_atomic, write_atomic};
#[cfg(feature = "native")]
pub use csv_io::{read_csv, write_csv};
#[cfg(feature = "native")]
pub use file_info::{FileInfo, get_file_info};
#[cfg(feature = "native")]
pub use format_detect::detect_format;
#[cfg(feature = "native")]
pub use recent_files::{RecentFile, RecentFileStore};
#[cfg(feature = "native")]
pub use tsv_io::{read_tsv, write_tsv};
#[cfg(feature = "native")]
pub use watcher::FileWatcher;
#[cfg(feature = "native")]
pub use xlsx_chart_reader::read_xlsx_charts;
#[cfg(feature = "native")]
pub use xlsx_reader::{read_ods, read_spreadsheet, read_xls, read_xlsx};
#[cfg(feature = "native")]
pub use xlsx_writer::write_xlsx;
