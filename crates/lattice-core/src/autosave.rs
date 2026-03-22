//! Auto-save configuration for the workbook.
//!
//! This module provides the [`AutoSaveConfig`] struct that holds the auto-save
//! settings. The actual timer and I/O are handled by the Tauri layer — this
//! module only stores the configuration state so it can be persisted alongside
//! the workbook.

use serde::{Deserialize, Serialize};

/// Configuration for automatic saving.
///
/// The core engine does not perform any I/O itself. This struct is read by the
/// Tauri application layer which runs the actual timer and writes the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoSaveConfig {
    /// Whether auto-save is enabled.
    pub enabled: bool,
    /// Interval in seconds between auto-saves (default 60).
    pub interval_secs: u64,
    /// Optional file path to save to. When `None`, the Tauri layer will use
    /// the workbook's current file path.
    pub path: Option<String>,
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 60,
            path: None,
        }
    }
}

impl AutoSaveConfig {
    /// Create a new auto-save config with the default settings (enabled, 60s).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a disabled auto-save config.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Set the interval in seconds. The minimum is 5 seconds.
    ///
    /// Values below 5 are clamped to 5 to avoid excessive saves.
    pub fn with_interval(mut self, secs: u64) -> Self {
        self.interval_secs = secs.max(5);
        self
    }

    /// Set the file path for auto-save.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AutoSaveConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.interval_secs, 60);
        assert!(cfg.path.is_none());
    }

    #[test]
    fn test_disabled() {
        let cfg = AutoSaveConfig::disabled();
        assert!(!cfg.enabled);
        assert_eq!(cfg.interval_secs, 60);
    }

    #[test]
    fn test_with_interval_clamps_minimum() {
        let cfg = AutoSaveConfig::new().with_interval(2);
        assert_eq!(cfg.interval_secs, 5);
    }

    #[test]
    fn test_with_interval_normal() {
        let cfg = AutoSaveConfig::new().with_interval(120);
        assert_eq!(cfg.interval_secs, 120);
    }

    #[test]
    fn test_with_path() {
        let cfg = AutoSaveConfig::new().with_path("/tmp/auto.lattice");
        assert_eq!(cfg.path.as_deref(), Some("/tmp/auto.lattice"));
    }

    #[test]
    fn test_serde_round_trip() {
        let cfg = AutoSaveConfig::new().with_interval(30).with_path("/tmp/test");
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: AutoSaveConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, deserialized);
    }
}
