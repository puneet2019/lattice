//! Shared SVG rendering utilities for chart generation.
//!
//! Provides color palettes, axis scale computation, text helpers,
//! margin calculations, and reusable SVG element builders used
//! across all chart types.

use crate::chart::{ChartData, ChartOptions};

// ── Default color palette (10 distinct, accessible colors) ──────────────

/// Default color palette for chart series.
///
/// These are chosen for visual distinction and reasonable contrast.
pub const PALETTE: &[&str] = &[
    "#4e79a7", // steel blue
    "#f28e2b", // orange
    "#e15759", // red
    "#76b7b2", // teal
    "#59a14f", // green
    "#edc948", // yellow
    "#b07aa1", // purple
    "#ff9da7", // pink
    "#9c755f", // brown
    "#bab0ac", // grey
];

/// Return the palette color for a given series index (wraps around).
pub fn series_color(index: usize) -> &'static str {
    PALETTE[index % PALETTE.len()]
}

/// Return a color from a custom palette (if provided) or fall back to the default palette.
///
/// This lets callers pass `options.color_palette.as_ref()` and get the right color
/// regardless of whether a custom palette was provided.
pub fn palette_color(index: usize, custom: Option<&[String]>) -> &str {
    match custom {
        Some(p) if !p.is_empty() => &p[index % p.len()],
        _ => series_color(index),
    }
}

// ── Axis scale calculation ──────────────────────────────────────────────

/// Computed axis scale with nice round numbers.
#[derive(Debug, Clone)]
pub struct AxisScale {
    /// Minimum value on the axis.
    pub min: f64,
    /// Maximum value on the axis.
    pub max: f64,
    /// Step size between tick marks.
    pub step: f64,
    /// Tick values from min to max.
    pub ticks: Vec<f64>,
}

/// Compute a nice axis scale for a given data range.
///
/// Produces round min/max/step values that encompass all data points
/// and look clean on an axis (multiples of 1, 2, 5, 10, etc.).
pub fn compute_axis_scale(data_min: f64, data_max: f64) -> AxisScale {
    if (data_max - data_min).abs() < f64::EPSILON {
        let v = data_min;
        let padding = if v.abs() < f64::EPSILON {
            1.0
        } else {
            v.abs() * 0.1
        };
        return compute_axis_scale(v - padding, v + padding);
    }

    let range = data_max - data_min;
    let rough_step = range / 5.0;
    let magnitude = 10_f64.powf(rough_step.log10().floor());
    let residual = rough_step / magnitude;

    let nice_step = if residual <= 1.5 {
        magnitude
    } else if residual <= 3.0 {
        2.0 * magnitude
    } else if residual <= 7.0 {
        5.0 * magnitude
    } else {
        10.0 * magnitude
    };

    let nice_min = (data_min / nice_step).floor() * nice_step;
    let nice_max = (data_max / nice_step).ceil() * nice_step;

    let mut ticks = Vec::new();
    let mut v = nice_min;
    while v <= nice_max + nice_step * 0.001 {
        ticks.push((v * 1e10).round() / 1e10);
        v += nice_step;
    }

    AxisScale {
        min: nice_min,
        max: nice_max,
        step: nice_step,
        ticks,
    }
}

// ── XML / SVG text helpers ──────────────────────────────────────────────

/// Escape special XML characters in text content.
pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Format a number for display on axes (no unnecessary trailing zeros).
pub fn format_axis_value(v: f64) -> String {
    if v.abs() >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v.abs() >= 1_000.0 {
        format!("{:.1}K", v / 1_000.0)
    } else if v.fract().abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}

/// Build an SVG `<text>` element.
pub fn svg_text(x: f64, y: f64, anchor: &str, font_size: u32, fill: &str, content: &str) -> String {
    format!(
        r#"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" font-family="sans-serif" font-size="{font_size}" fill="{fill}">{}</text>"#,
        xml_escape(content)
    )
}

// ── Plot area margins ───────────────────────────────────────────────────

/// Plot area margins (left, top, right, bottom) in pixels.
#[derive(Debug, Clone, Copy)]
pub struct Margins {
    /// Left margin (y-axis labels).
    pub left: f64,
    /// Top margin (title).
    pub top: f64,
    /// Right margin (legend).
    pub right: f64,
    /// Bottom margin (x-axis labels).
    pub bottom: f64,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            left: 60.0,
            top: 40.0,
            right: 20.0,
            bottom: 50.0,
        }
    }
}

impl Margins {
    /// Widen margins when legend, title, subtitle, or axis labels are present.
    pub fn for_options(options: &ChartOptions) -> Self {
        let mut m = Self::default();
        if options.title.is_some() {
            m.top = 55.0;
        }
        if options.subtitle.is_some() {
            m.top += 18.0;
        }
        if options.y_axis_label.is_some() {
            m.left = 75.0;
        }
        if options.x_axis_label.is_some() {
            m.bottom = 65.0;
        }
        if options.show_legend {
            m.right = 130.0;
        }
        m
    }

    /// Width of the plot area.
    pub fn plot_width(&self, total_width: u32) -> f64 {
        total_width as f64 - self.left - self.right
    }

    /// Height of the plot area.
    pub fn plot_height(&self, total_height: u32) -> f64 {
        total_height as f64 - self.top - self.bottom
    }
}

// ── Reusable SVG structure builders ─────────────────────────────────────

/// Build the opening `<svg>` tag and background rect.
///
/// Uses `options.background_color` if set, otherwise defaults to white.
pub fn svg_open(options: &ChartOptions) -> String {
    let bg = options.background_color.as_deref().unwrap_or("#ffffff");
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<rect width="100%" height="100%" fill="{bg}" rx="4"/>"##,
        w = options.width,
        h = options.height,
    )
}

/// Build the closing `</svg>` tag.
pub fn svg_close() -> &'static str {
    "</svg>"
}

/// Render the chart title and optional subtitle.
pub fn svg_title(options: &ChartOptions) -> String {
    let mut out = String::new();
    if let Some(t) = &options.title {
        out.push_str(&svg_text(
            options.width as f64 / 2.0,
            28.0,
            "middle",
            16,
            "#333333",
            t,
        ));
        out.push('\n');
    }
    if let Some(sub) = &options.subtitle {
        let y = if options.title.is_some() { 46.0 } else { 28.0 };
        out.push_str(&svg_text(
            options.width as f64 / 2.0,
            y,
            "middle",
            12,
            "#666666",
            sub,
        ));
        out.push('\n');
    }
    out
}

/// Render axis labels (x and y).
pub fn svg_axis_labels(options: &ChartOptions, margins: &Margins) -> String {
    let mut out = String::new();
    if let Some(label) = &options.x_axis_label {
        let x = margins.left + margins.plot_width(options.width) / 2.0;
        let y = options.height as f64 - 8.0;
        out.push_str(&svg_text(x, y, "middle", 12, "#666666", label));
        out.push('\n');
    }
    if let Some(label) = &options.y_axis_label {
        let y = margins.top + margins.plot_height(options.height) / 2.0;
        out.push_str(&format!(
            r##"<text x="14" y="{y:.1}" text-anchor="middle" font-family="sans-serif" font-size="12" fill="#666666" transform="rotate(-90, 14, {y:.1})">{}</text>"##,
            xml_escape(label)
        ));
        out.push('\n');
    }
    out
}

/// Render horizontal grid lines for the y-axis.
pub fn svg_grid_lines(scale: &AxisScale, margins: &Margins, options: &ChartOptions) -> String {
    if !options.show_grid {
        return String::new();
    }
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);
    let range = scale.max - scale.min;
    let mut out = String::new();
    for &tick in &scale.ticks {
        let frac = if range.abs() > f64::EPSILON {
            (tick - scale.min) / range
        } else {
            0.5
        };
        let y = margins.top + ph * (1.0 - frac);
        out.push_str(&format!(
            r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="#e0e0e0" stroke-width="1"/>"##,
            x1 = margins.left,
            x2 = margins.left + pw,
        ));
        out.push('\n');
        out.push_str(&svg_text(
            margins.left - 8.0,
            y + 4.0,
            "end",
            10,
            "#666666",
            &format_axis_value(tick),
        ));
        out.push('\n');
    }
    out
}

/// Render a legend for multiple data series.
pub fn svg_legend(data: &ChartData, margins: &Margins, options: &ChartOptions) -> String {
    if !options.show_legend || data.series.len() <= 1 {
        return String::new();
    }
    let x = margins.left + margins.plot_width(options.width) + 12.0;
    let mut out = String::new();
    for (i, s) in data.series.iter().enumerate() {
        let color = s.color.as_deref().unwrap_or_else(|| series_color(i));
        let y = margins.top + 10.0 + i as f64 * 20.0;
        out.push_str(&format!(
            r#"<rect x="{x:.1}" y="{ry:.1}" width="12" height="12" fill="{color}" rx="2"/>"#,
            ry = y - 9.0,
        ));
        out.push_str(&svg_text(x + 16.0, y, "start", 11, "#333333", &s.name));
        out.push('\n');
    }
    out
}

// ── Data range helpers ──────────────────────────────────────────────────

/// Find the global min and max values across all series.
pub fn data_range(data: &ChartData) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for s in &data.series {
        for &v in &s.values {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
    }
    if min > max { (0.0, 1.0) } else { (min, max) }
}

/// Find the global min/max, ensuring 0 is included (useful for bar/area charts).
pub fn data_range_with_zero(data: &ChartData) -> (f64, f64) {
    let (min, max) = data_range(data);
    (min.min(0.0), max.max(0.0))
}

/// Compute y-axis range for stacked charts (sum of all series values per category).
///
/// Returns `(min, max)` with zero always included.
pub fn data_range_stacked(data: &ChartData) -> (f64, f64) {
    if data.series.is_empty() || data.labels.is_empty() {
        return (0.0, 1.0);
    }
    let n_categories = data.labels.len();
    let mut max_sum = f64::NEG_INFINITY;
    for ci in 0..n_categories {
        let sum: f64 = data
            .series
            .iter()
            .map(|s| s.values.get(ci).copied().unwrap_or(0.0).max(0.0))
            .sum();
        if sum > max_sum {
            max_sum = sum;
        }
    }
    if max_sum < 0.0 || max_sum < f64::EPSILON {
        max_sum = 1.0;
    }
    (0.0, max_sum)
}

/// Format a number for data labels (compact display).
pub fn format_data_label(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::DataSeries;

    #[test]
    fn test_series_color_wraps() {
        assert_eq!(series_color(0), "#4e79a7");
        assert_eq!(series_color(10), "#4e79a7");
    }

    #[test]
    fn test_compute_axis_scale_basic() {
        let scale = compute_axis_scale(0.0, 100.0);
        assert!(scale.min <= 0.0);
        assert!(scale.max >= 100.0);
        assert!(scale.step > 0.0);
        assert!(!scale.ticks.is_empty());
    }

    #[test]
    fn test_compute_axis_scale_same_values() {
        let scale = compute_axis_scale(50.0, 50.0);
        assert!(scale.min < 50.0);
        assert!(scale.max > 50.0);
    }

    #[test]
    fn test_compute_axis_scale_negative() {
        let scale = compute_axis_scale(-50.0, 50.0);
        assert!(scale.min <= -50.0);
        assert!(scale.max >= 50.0);
    }

    #[test]
    fn test_format_axis_value() {
        assert_eq!(format_axis_value(0.0), "0");
        assert_eq!(format_axis_value(100.0), "100");
        assert_eq!(format_axis_value(1500.0), "1.5K");
        assert_eq!(format_axis_value(2_000_000.0), "2.0M");
        assert_eq!(format_axis_value(3.5), "3.5");
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("a & b"), "a &amp; b");
        assert_eq!(xml_escape("<b>"), "&lt;b&gt;");
    }

    #[test]
    fn test_data_range_empty() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let (min, max) = data_range(&data);
        assert_eq!(min, 0.0);
        assert_eq!(max, 1.0);
    }

    #[test]
    fn test_data_range_basic() {
        let data = ChartData {
            labels: vec!["A".into()],
            series: vec![DataSeries {
                name: "S1".into(),
                values: vec![10.0, 50.0, 30.0],
                color: None,
            }],
        };
        let (min, max) = data_range(&data);
        assert_eq!(min, 10.0);
        assert_eq!(max, 50.0);
    }

    #[test]
    fn test_data_range_with_zero() {
        let data = ChartData {
            labels: vec!["A".into()],
            series: vec![DataSeries {
                name: "S1".into(),
                values: vec![10.0, 50.0],
                color: None,
            }],
        };
        let (min, max) = data_range_with_zero(&data);
        assert_eq!(min, 0.0);
        assert_eq!(max, 50.0);
    }

    #[test]
    fn test_margins_default() {
        let m = Margins::default();
        assert_eq!(m.left, 60.0);
        assert_eq!(m.top, 40.0);
        assert_eq!(m.right, 20.0);
        assert_eq!(m.bottom, 50.0);
    }

    #[test]
    fn test_margins_plot_dimensions() {
        let m = Margins::default();
        let pw = m.plot_width(600);
        let ph = m.plot_height(400);
        assert_eq!(pw, 600.0 - 60.0 - 20.0);
        assert_eq!(ph, 400.0 - 40.0 - 50.0);
    }

    #[test]
    fn test_svg_open_close() {
        let opts = ChartOptions::default();
        let open = svg_open(&opts);
        assert!(open.starts_with("<svg"));
        assert!(open.contains("width=\"600\""));
        assert!(open.contains("fill=\"#ffffff\""));
        assert_eq!(svg_close(), "</svg>");
    }

    #[test]
    fn test_svg_open_custom_background() {
        let opts = ChartOptions {
            background_color: Some("#f0f0f0".into()),
            ..ChartOptions::default()
        };
        let open = svg_open(&opts);
        assert!(open.contains("fill=\"#f0f0f0\""));
    }

    #[test]
    fn test_svg_title_with_subtitle() {
        let opts = ChartOptions {
            title: Some("Main Title".into()),
            subtitle: Some("Sub Title".into()),
            ..ChartOptions::default()
        };
        let title_svg = svg_title(&opts);
        assert!(title_svg.contains("Main Title"));
        assert!(title_svg.contains("Sub Title"));
    }

    #[test]
    fn test_svg_subtitle_only() {
        let opts = ChartOptions {
            subtitle: Some("Just Subtitle".into()),
            ..ChartOptions::default()
        };
        let title_svg = svg_title(&opts);
        assert!(title_svg.contains("Just Subtitle"));
        assert!(!title_svg.contains("font-size=\"16\""));
    }

    #[test]
    fn test_palette_color_default() {
        assert_eq!(palette_color(0, None), "#4e79a7");
        assert_eq!(palette_color(1, None), "#f28e2b");
    }

    #[test]
    fn test_palette_color_custom() {
        let custom = vec!["#aaa".to_string(), "#bbb".to_string()];
        assert_eq!(palette_color(0, Some(&custom)), "#aaa");
        assert_eq!(palette_color(1, Some(&custom)), "#bbb");
        assert_eq!(palette_color(2, Some(&custom)), "#aaa"); // wraps
    }

    #[test]
    fn test_palette_color_empty_custom_falls_back() {
        let custom: Vec<String> = vec![];
        assert_eq!(palette_color(0, Some(&custom)), "#4e79a7");
    }

    #[test]
    fn test_margins_with_subtitle() {
        let opts = ChartOptions {
            title: Some("Title".into()),
            subtitle: Some("Subtitle".into()),
            ..ChartOptions::default()
        };
        let m = Margins::for_options(&opts);
        assert_eq!(m.top, 55.0 + 18.0); // title + subtitle
    }
}
