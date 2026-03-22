//! Sparkline support for Lattice sheets.
//!
//! Sparklines are tiny inline charts embedded in a single cell. They
//! visualise a data range as a line, bar, or win/loss chart and are
//! stored per-cell in [`SparklineStore`] on a [`Sheet`](crate::sheet::Sheet).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::selection::Range;

// ── Types ───────────────────────────────────────────────────────────────

/// The visual style of a sparkline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SparklineType {
    /// A polyline connecting data points.
    Line,
    /// Tiny vertical bars, one per data point.
    Bar,
    /// Bars above/below a midline: positive = up, negative = down.
    WinLoss,
}

/// Configuration for a single sparkline attached to a cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SparklineConfig {
    /// Visual type (line, bar, or win/loss).
    pub spark_type: SparklineType,
    /// The data range that feeds this sparkline.
    pub data_range: Range,
    /// Primary colour (CSS hex, e.g. `"#4e79a7"`).
    pub color: Option<String>,
    /// Highlight colour for the maximum value.
    pub high_color: Option<String>,
    /// Highlight colour for the minimum value.
    pub low_color: Option<String>,
    /// Colour used for negative values (win/loss and bar types).
    pub negative_color: Option<String>,
    /// Whether to show point markers (line type only).
    pub show_markers: bool,
    /// Stroke width for line-type sparklines.
    pub line_width: f32,
}

impl Default for SparklineConfig {
    fn default() -> Self {
        Self {
            spark_type: SparklineType::Line,
            data_range: Range {
                start: crate::selection::CellRef { row: 0, col: 0 },
                end: crate::selection::CellRef { row: 0, col: 0 },
            },
            color: None,
            high_color: None,
            low_color: None,
            negative_color: None,
            show_markers: false,
            line_width: 1.5,
        }
    }
}

// ── Store ───────────────────────────────────────────────────────────────

/// Per-sheet storage for sparklines, keyed by (row, col) cell position.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SparklineStore {
    sparklines: HashMap<(u32, u32), SparklineConfig>,
}

impl SparklineStore {
    /// Create an empty sparkline store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a sparkline to the cell at `(row, col)`.
    ///
    /// If a sparkline already exists at that position it is replaced.
    pub fn add(&mut self, row: u32, col: u32, config: SparklineConfig) {
        self.sparklines.insert((row, col), config);
    }

    /// Remove the sparkline at `(row, col)`, if any.
    ///
    /// Returns `true` if a sparkline was removed.
    pub fn remove(&mut self, row: u32, col: u32) -> bool {
        self.sparklines.remove(&(row, col)).is_some()
    }

    /// Get a reference to the sparkline config at `(row, col)`.
    pub fn get(&self, row: u32, col: u32) -> Option<&SparklineConfig> {
        self.sparklines.get(&(row, col))
    }

    /// List all sparklines as `((row, col), config)` pairs.
    pub fn list(&self) -> Vec<((u32, u32), &SparklineConfig)> {
        self.sparklines
            .iter()
            .map(|(&pos, cfg)| (pos, cfg))
            .collect()
    }

    /// Return the number of sparklines in this store.
    pub fn len(&self) -> usize {
        self.sparklines.len()
    }

    /// Return `true` if the store contains no sparklines.
    pub fn is_empty(&self) -> bool {
        self.sparklines.is_empty()
    }
}

// ── SVG rendering ───────────────────────────────────────────────────────

const DEFAULT_COLOR: &str = "#4e79a7";
const DEFAULT_NEGATIVE_COLOR: &str = "#e15759";
const DEFAULT_HIGH_COLOR: &str = "#59a14f";
const DEFAULT_LOW_COLOR: &str = "#e15759";

/// Render a sparkline as an SVG string.
///
/// The SVG fits within the given `width` x `height` pixel box.
/// The `values` slice provides the data points to visualise.
///
/// # Examples
/// ```
/// use lattice_core::sparkline::{SparklineConfig, SparklineType, render_sparkline_svg};
/// let cfg = SparklineConfig {
///     spark_type: SparklineType::Line,
///     show_markers: true,
///     ..SparklineConfig::default()
/// };
/// let svg = render_sparkline_svg(&[1.0, 3.0, 2.0, 5.0], &cfg, 120.0, 24.0);
/// assert!(svg.contains("<svg"));
/// assert!(svg.contains("polyline"));
/// ```
pub fn render_sparkline_svg(
    values: &[f64],
    config: &SparklineConfig,
    width: f32,
    height: f32,
) -> String {
    if values.is_empty() {
        return format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}"></svg>"#,
        );
    }

    match config.spark_type {
        SparklineType::Line => render_line(values, config, width, height),
        SparklineType::Bar => render_bar(values, config, width, height),
        SparklineType::WinLoss => render_winloss(values, config, width, height),
    }
}

/// Find min/max indices in a values slice.
fn min_max_indices(values: &[f64]) -> (usize, usize) {
    let mut min_idx = 0;
    let mut max_idx = 0;
    for (i, &v) in values.iter().enumerate() {
        if v < values[min_idx] {
            min_idx = i;
        }
        if v > values[max_idx] {
            max_idx = i;
        }
    }
    (min_idx, max_idx)
}

/// Render a line-type sparkline.
fn render_line(values: &[f64], config: &SparklineConfig, width: f32, height: f32) -> String {
    let color = config.color.as_deref().unwrap_or(DEFAULT_COLOR);
    let lw = config.line_width;

    let padding = 2.0_f32;
    let pw = width - padding * 2.0;
    let ph = height - padding * 2.0;

    let vmin = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let vmax = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let vrange = if (vmax - vmin).abs() < f64::EPSILON {
        1.0
    } else {
        vmax - vmin
    };

    let n = values.len();
    let x_step = if n > 1 { pw / (n - 1) as f32 } else { 0.0 };

    // Build polyline points
    let points: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = padding + i as f32 * x_step;
            let frac = ((v - vmin) / vrange) as f32;
            let y = padding + ph * (1.0 - frac);
            format!("{x:.1},{y:.1}")
        })
        .collect();

    let mut svg = String::with_capacity(512);
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">"#,
    ));
    svg.push_str(&format!(
        r#"<polyline points="{}" fill="none" stroke="{color}" stroke-width="{lw}" stroke-linejoin="round"/>"#,
        points.join(" "),
    ));

    // Optional markers (including high/low highlighting)
    if config.show_markers {
        let (min_idx, max_idx) = min_max_indices(values);
        let marker_r = (lw * 1.5).max(2.0);
        for (i, &v) in values.iter().enumerate() {
            let x = padding + i as f32 * x_step;
            let frac = ((v - vmin) / vrange) as f32;
            let y = padding + ph * (1.0 - frac);
            let fill = if i == max_idx {
                config.high_color.as_deref().unwrap_or(DEFAULT_HIGH_COLOR)
            } else if i == min_idx {
                config.low_color.as_deref().unwrap_or(DEFAULT_LOW_COLOR)
            } else {
                color
            };
            svg.push_str(&format!(
                r#"<circle cx="{x:.1}" cy="{y:.1}" r="{marker_r}" fill="{fill}"/>"#,
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

/// Render a bar-type sparkline.
fn render_bar(values: &[f64], config: &SparklineConfig, width: f32, height: f32) -> String {
    let color = config.color.as_deref().unwrap_or(DEFAULT_COLOR);
    let neg_color = config
        .negative_color
        .as_deref()
        .unwrap_or(DEFAULT_NEGATIVE_COLOR);

    let padding = 1.0_f32;
    let pw = width - padding * 2.0;
    let ph = height - padding * 2.0;

    let n = values.len();
    let gap = 1.0_f32;
    let bar_w = (pw - gap * (n as f32 - 1.0).max(0.0)) / n as f32;

    let vmin = values
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min)
        .min(0.0);
    let vmax = values
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0);
    let vrange = if (vmax - vmin).abs() < f64::EPSILON {
        1.0
    } else {
        vmax - vmin
    };

    let zero_y = padding + ph * (1.0 - ((0.0 - vmin) / vrange) as f32);

    let (min_idx, max_idx) = min_max_indices(values);

    let mut svg = String::with_capacity(512);
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">"#,
    ));

    for (i, &v) in values.iter().enumerate() {
        let x = padding + i as f32 * (bar_w + gap);
        let frac = ((v - vmin) / vrange) as f32;
        let top_y = padding + ph * (1.0 - frac);

        let (bar_y, bar_h) = if v >= 0.0 {
            (top_y, zero_y - top_y)
        } else {
            (zero_y, top_y - zero_y)
        };
        let bar_h = bar_h.max(0.5); // ensure visible

        let fill = if i == max_idx {
            config.high_color.as_deref().unwrap_or(color)
        } else if i == min_idx {
            config
                .low_color
                .as_deref()
                .unwrap_or(if v < 0.0 { neg_color } else { color })
        } else if v < 0.0 {
            neg_color
        } else {
            color
        };

        svg.push_str(&format!(
            r#"<rect x="{x:.1}" y="{bar_y:.1}" width="{bar_w:.1}" height="{bar_h:.1}" fill="{fill}"/>"#,
        ));
    }

    svg.push_str("</svg>");
    svg
}

/// Render a win/loss sparkline.
///
/// All positive values are drawn as equal-height bars above the midline;
/// all negative (or zero) values are drawn as equal-height bars below.
fn render_winloss(values: &[f64], config: &SparklineConfig, width: f32, height: f32) -> String {
    let color = config.color.as_deref().unwrap_or(DEFAULT_COLOR);
    let neg_color = config
        .negative_color
        .as_deref()
        .unwrap_or(DEFAULT_NEGATIVE_COLOR);

    let padding = 1.0_f32;
    let pw = width - padding * 2.0;
    let ph = height - padding * 2.0;

    let n = values.len();
    let gap = 1.0_f32;
    let bar_w = (pw - gap * (n as f32 - 1.0).max(0.0)) / n as f32;

    let mid_y = padding + ph / 2.0;
    let bar_h = ph / 2.0 - 1.0; // leave 1px gap from edge

    let mut svg = String::with_capacity(512);
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">"#,
    ));

    for (i, &v) in values.iter().enumerate() {
        let x = padding + i as f32 * (bar_w + gap);
        let (fill, bar_y) = if v > 0.0 {
            (color, mid_y - bar_h)
        } else {
            (neg_color, mid_y)
        };
        svg.push_str(&format!(
            r#"<rect x="{x:.1}" y="{bar_y:.1}" width="{bar_w:.1}" height="{bar_h:.1}" fill="{fill}"/>"#,
        ));
    }

    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::{CellRef, Range};

    fn make_range() -> Range {
        Range {
            start: CellRef { row: 0, col: 0 },
            end: CellRef { row: 0, col: 4 },
        }
    }

    fn line_config() -> SparklineConfig {
        SparklineConfig {
            spark_type: SparklineType::Line,
            data_range: make_range(),
            show_markers: true,
            ..SparklineConfig::default()
        }
    }

    fn bar_config() -> SparklineConfig {
        SparklineConfig {
            spark_type: SparklineType::Bar,
            data_range: make_range(),
            ..SparklineConfig::default()
        }
    }

    fn winloss_config() -> SparklineConfig {
        SparklineConfig {
            spark_type: SparklineType::WinLoss,
            data_range: make_range(),
            ..SparklineConfig::default()
        }
    }

    // ── Store tests ─────────────────────────────────────────────────

    #[test]
    fn test_store_add_get() {
        let mut store = SparklineStore::new();
        assert!(store.is_empty());
        store.add(0, 5, line_config());
        assert_eq!(store.len(), 1);
        assert!(store.get(0, 5).is_some());
        assert_eq!(store.get(0, 5).unwrap().spark_type, SparklineType::Line);
    }

    #[test]
    fn test_store_remove() {
        let mut store = SparklineStore::new();
        store.add(1, 2, bar_config());
        assert!(store.remove(1, 2));
        assert!(!store.remove(1, 2)); // already removed
        assert!(store.is_empty());
    }

    #[test]
    fn test_store_replace() {
        let mut store = SparklineStore::new();
        store.add(0, 0, line_config());
        store.add(0, 0, bar_config());
        assert_eq!(store.len(), 1);
        assert_eq!(store.get(0, 0).unwrap().spark_type, SparklineType::Bar,);
    }

    #[test]
    fn test_store_list() {
        let mut store = SparklineStore::new();
        store.add(0, 0, line_config());
        store.add(1, 1, bar_config());
        let items = store.list();
        assert_eq!(items.len(), 2);
    }

    // ── SVG rendering: empty data ───────────────────────────────────

    #[test]
    fn test_render_empty_values() {
        let svg = render_sparkline_svg(&[], &line_config(), 100.0, 20.0);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(!svg.contains("polyline"));
    }

    // ── Line sparkline tests ────────────────────────────────────────

    #[test]
    fn test_render_line_basic() {
        let svg = render_sparkline_svg(&[1.0, 3.0, 2.0, 5.0, 4.0], &line_config(), 120.0, 24.0);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("polyline"));
        assert!(svg.contains("circle")); // markers enabled
    }

    #[test]
    fn test_render_line_no_markers() {
        let mut cfg = line_config();
        cfg.show_markers = false;
        let svg = render_sparkline_svg(&[1.0, 2.0, 3.0], &cfg, 100.0, 20.0);
        assert!(svg.contains("polyline"));
        assert!(!svg.contains("circle"));
    }

    #[test]
    fn test_render_line_custom_color() {
        let mut cfg = line_config();
        cfg.color = Some("#ff0000".to_string());
        let svg = render_sparkline_svg(&[1.0, 2.0], &cfg, 80.0, 16.0);
        assert!(svg.contains("#ff0000"));
    }

    #[test]
    fn test_render_line_single_value() {
        let svg = render_sparkline_svg(&[42.0], &line_config(), 80.0, 16.0);
        assert!(svg.contains("polyline"));
    }

    #[test]
    fn test_render_line_constant_values() {
        // All same value — should not panic or produce NaN
        let svg = render_sparkline_svg(&[5.0, 5.0, 5.0], &line_config(), 80.0, 16.0);
        assert!(svg.contains("polyline"));
        assert!(!svg.contains("NaN"));
    }

    #[test]
    fn test_render_line_high_low_colors() {
        let mut cfg = line_config();
        cfg.high_color = Some("#00ff00".to_string());
        cfg.low_color = Some("#0000ff".to_string());
        let svg = render_sparkline_svg(&[1.0, 5.0, 2.0], &cfg, 100.0, 20.0);
        assert!(svg.contains("#00ff00")); // high color for max
        assert!(svg.contains("#0000ff")); // low color for min
    }

    // ── Bar sparkline tests ─────────────────────────────────────────

    #[test]
    fn test_render_bar_basic() {
        let svg = render_sparkline_svg(&[3.0, 1.0, 4.0, 2.0], &bar_config(), 120.0, 24.0);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("<rect"));
    }

    #[test]
    fn test_render_bar_negative_values() {
        let svg = render_sparkline_svg(&[3.0, -2.0, 1.0, -4.0], &bar_config(), 120.0, 24.0);
        assert!(svg.contains("<rect"));
        // Default negative colour should appear
        assert!(svg.contains(DEFAULT_NEGATIVE_COLOR));
    }

    #[test]
    fn test_render_bar_custom_negative_color() {
        let mut cfg = bar_config();
        cfg.negative_color = Some("#aa00aa".to_string());
        let svg = render_sparkline_svg(&[1.0, -1.0], &cfg, 80.0, 16.0);
        assert!(svg.contains("#aa00aa"));
    }

    // ── Win/loss sparkline tests ────────────────────────────────────

    #[test]
    fn test_render_winloss_basic() {
        let svg =
            render_sparkline_svg(&[1.0, -1.0, 1.0, -1.0, 1.0], &winloss_config(), 120.0, 24.0);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("<rect"));
        // Should have both positive and negative colors
        assert!(svg.contains(DEFAULT_COLOR));
        assert!(svg.contains(DEFAULT_NEGATIVE_COLOR));
    }

    #[test]
    fn test_render_winloss_all_positive() {
        let svg = render_sparkline_svg(&[1.0, 2.0, 3.0], &winloss_config(), 100.0, 20.0);
        // All bars should use the positive colour
        assert!(svg.contains(DEFAULT_COLOR));
    }

    #[test]
    fn test_render_winloss_zero_is_negative() {
        // Zero should be treated as a "loss" (below midline)
        let svg = render_sparkline_svg(&[0.0], &winloss_config(), 40.0, 20.0);
        assert!(svg.contains(DEFAULT_NEGATIVE_COLOR));
    }

    // ── Serde round-trip ────────────────────────────────────────────

    #[test]
    fn test_sparkline_type_serde() {
        let json = serde_json::to_string(&SparklineType::WinLoss).unwrap();
        assert_eq!(json, "\"win_loss\"");
        let parsed: SparklineType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SparklineType::WinLoss);
    }

    #[test]
    fn test_sparkline_config_serde() {
        let cfg = line_config();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: SparklineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.spark_type, SparklineType::Line);
        assert_eq!(parsed.show_markers, true);
    }
}
