//! Bubble chart SVG rendering.
//!
//! Renders a scatter-like chart with a third dimension represented by
//! bubble (circle) size. Expects data series in triplets: X values,
//! Y values, and Size values. Bubbles are semi-transparent circles
//! scaled proportionally to the size values.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    Margins, compute_axis_scale, format_axis_value, palette_color, svg_axis_labels, svg_close,
    svg_open, svg_text, svg_title,
};

/// Minimum bubble radius in pixels.
const MIN_BUBBLE_R: f64 = 4.0;
/// Maximum bubble radius in pixels.
const MAX_BUBBLE_R: f64 = 40.0;

/// Render bubble chart data as an SVG string.
///
/// Series are grouped in triples: `[X, Y, Size]`. Missing series default
/// to indices (X) or constants (size). Multiple triples create colored groups.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    // Parse bubble groups (each group is 3 series: X, Y, Size)
    let groups = parse_bubble_groups(data);

    // Compute axis scales from all groups
    let (x_min, x_max, y_min, y_max, size_max) = compute_ranges(&groups);

    let x_scale = compute_axis_scale(x_min, x_max);
    let y_scale = compute_axis_scale(y_min, y_max);
    let x_range = x_scale.max - x_scale.min;
    let y_range = y_scale.max - y_scale.min;

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');

    // Y-axis grid lines
    if options.show_grid {
        for &tick in &y_scale.ticks {
            let frac = safe_frac(tick, y_scale.min, y_range);
            let y = margins.top + ph * (1.0 - frac);
            svg.push_str(&format!(
                r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="#e0e0e0" stroke-width="1"/>"##,
                x1 = margins.left,
                x2 = margins.left + pw,
            ));
            svg.push('\n');
            svg.push_str(&svg_text(
                margins.left - 8.0,
                y + 4.0,
                "end",
                10,
                "#666666",
                &format_axis_value(tick),
            ));
            svg.push('\n');
        }
    }

    // X-axis tick labels
    for &tick in &x_scale.ticks {
        let frac = safe_frac(tick, x_scale.min, x_range);
        let x = margins.left + frac * pw;
        let y = margins.top + ph + 18.0;
        svg.push_str(&svg_text(
            x,
            y,
            "middle",
            10,
            "#666666",
            &format_axis_value(tick),
        ));
        svg.push('\n');
    }

    svg.push_str(&svg_axis_labels(options, &margins));

    // Draw bubbles for each group
    let custom_palette = options.color_palette.as_deref();
    for (gi, group) in groups.iter().enumerate() {
        let color = palette_color(gi, custom_palette);

        for i in 0..group.x.len() {
            let xv = group.x[i];
            let yv = group.y[i];
            let sv = group.size[i];

            let x_frac = safe_frac(xv, x_scale.min, x_range);
            let y_frac = safe_frac(yv, y_scale.min, y_range);

            let px = margins.left + x_frac * pw;
            let py = margins.top + ph * (1.0 - y_frac);

            // Scale bubble radius proportionally
            let r = if size_max > 0.0 {
                MIN_BUBBLE_R + (MAX_BUBBLE_R - MIN_BUBBLE_R) * (sv / size_max).sqrt()
            } else {
                MIN_BUBBLE_R
            };

            svg.push_str(&format!(
                r#"<circle cx="{px:.1}" cy="{py:.1}" r="{r:.1}" fill="{color}" fill-opacity="0.5" stroke="{color}" stroke-width="1"/>"#,
            ));
            svg.push('\n');

            // Data label
            if options.show_data_labels {
                svg.push_str(&svg_text(
                    px,
                    py + 4.0,
                    "middle",
                    9,
                    "#333333",
                    &format_axis_value(sv),
                ));
                svg.push('\n');
            }
        }
    }

    // Axes
    svg.push_str(&format!(
        r##"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="#333333" stroke-width="1"/>"##,
        x1 = margins.left,
        y1 = margins.top + ph,
        x2 = margins.left + pw,
        y2 = margins.top + ph,
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r##"<line x1="{x:.1}" y1="{y1:.1}" x2="{x:.1}" y2="{y2:.1}" stroke="#333333" stroke-width="1"/>"##,
        x = margins.left,
        y1 = margins.top,
        y2 = margins.top + ph,
    ));
    svg.push('\n');

    // Legend (when multiple groups)
    if options.show_legend && groups.len() > 1 {
        let lx = margins.left + pw + 12.0;
        for (i, group) in groups.iter().enumerate() {
            let color = palette_color(i, custom_palette);
            let y = margins.top + 10.0 + i as f64 * 20.0;
            svg.push_str(&format!(
                r#"<rect x="{lx:.1}" y="{ry:.1}" width="12" height="12" fill="{color}" rx="2"/>"#,
                ry = y - 9.0,
            ));
            svg.push_str(&svg_text(
                lx + 16.0,
                y,
                "start",
                11,
                "#333333",
                &group.name,
            ));
            svg.push('\n');
        }
    }

    svg.push_str(svg_close());
    svg
}

/// A group of bubble data (x, y, size triples).
struct BubbleGroup {
    name: String,
    x: Vec<f64>,
    y: Vec<f64>,
    size: Vec<f64>,
}

/// Parse data series into bubble groups.
///
/// Each group of 3 consecutive series forms one bubble group:
/// `[X_values, Y_values, Size_values]`.
/// If only 1 or 2 series exist, fill in defaults.
fn parse_bubble_groups(data: &ChartData) -> Vec<BubbleGroup> {
    if data.series.is_empty() {
        return vec![];
    }

    let n_series = data.series.len();
    if n_series < 3 {
        // Single group with defaults for missing series
        let x_vals: Vec<f64> = if !data.series.is_empty() {
            data.series[0].values.clone()
        } else {
            vec![]
        };
        let n = x_vals.len();
        let y_vals: Vec<f64> = if n_series >= 2 {
            data.series[1].values.clone()
        } else {
            (0..n).map(|i| i as f64).collect()
        };
        let size_vals: Vec<f64> = vec![10.0; n];
        let name = data.series[0].name.clone();
        return vec![BubbleGroup {
            name,
            x: x_vals,
            y: y_vals,
            size: size_vals,
        }];
    }

    // Group series in triples
    let mut groups = Vec::new();
    let mut i = 0;
    while i + 2 < n_series {
        let n = data.series[i].values.len();
        let name = data.series[i + 1].name.clone();
        groups.push(BubbleGroup {
            name,
            x: data.series[i].values.clone(),
            y: data.series[i + 1].values.clone(),
            size: data.series[i + 2].values.iter().take(n).cloned().collect(),
        });
        i += 3;
    }

    groups
}

/// Compute global ranges across all bubble groups.
fn compute_ranges(groups: &[BubbleGroup]) -> (f64, f64, f64, f64, f64) {
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    let mut s_max = 0.0_f64;

    for g in groups {
        for &v in &g.x {
            x_min = x_min.min(v);
            x_max = x_max.max(v);
        }
        for &v in &g.y {
            y_min = y_min.min(v);
            y_max = y_max.max(v);
        }
        for &v in &g.size {
            s_max = s_max.max(v.abs());
        }
    }

    if x_min > x_max {
        x_min = 0.0;
        x_max = 1.0;
    }
    if y_min > y_max {
        y_min = 0.0;
        y_max = 1.0;
    }

    (x_min, x_max, y_min, y_max, s_max)
}

/// Safe fraction computation that handles zero range.
fn safe_frac(val: f64, min: f64, range: f64) -> f64 {
    if range.abs() > f64::EPSILON {
        (val - min) / range
    } else {
        0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::DataSeries;

    fn sample_data() -> ChartData {
        ChartData {
            labels: vec![],
            series: vec![
                DataSeries {
                    name: "X".into(),
                    values: vec![10.0, 20.0, 30.0, 40.0, 50.0],
                    color: None,
                },
                DataSeries {
                    name: "Y".into(),
                    values: vec![5.0, 15.0, 10.0, 25.0, 20.0],
                    color: None,
                },
                DataSeries {
                    name: "Size".into(),
                    values: vec![100.0, 200.0, 50.0, 300.0, 150.0],
                    color: None,
                },
            ],
        }
    }

    #[test]
    fn test_bubble_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_bubble_contains_circles() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // 5 data points = 5 semi-transparent bubbles
        let bubble_count = svg.matches("fill-opacity=\"0.5\"").count();
        assert_eq!(bubble_count, 5);
    }

    #[test]
    fn test_bubble_has_axes() {
        let svg = render(&sample_data(), &ChartOptions::default());
        let axis_lines = svg.matches(r##"stroke="#333333""##).count();
        assert!(axis_lines >= 2);
    }

    #[test]
    fn test_bubble_with_title() {
        let opts = ChartOptions {
            title: Some("Market Analysis".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Market Analysis"));
    }

    #[test]
    fn test_bubble_data_labels() {
        let opts = ChartOptions {
            show_data_labels: true,
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("100"));
        assert!(svg.contains("300"));
    }

    #[test]
    fn test_bubble_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_bubble_single_series_fallback() {
        let data = ChartData {
            labels: vec![],
            series: vec![DataSeries {
                name: "Values".into(),
                values: vec![10.0, 20.0, 30.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // Should still render 3 bubbles with default sizes
        let bubble_count = svg.matches("fill-opacity=\"0.5\"").count();
        assert_eq!(bubble_count, 3);
    }

    #[test]
    fn test_parse_bubble_groups_triplets() {
        let data = sample_data();
        let groups = parse_bubble_groups(&data);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].x.len(), 5);
        assert_eq!(groups[0].y.len(), 5);
        assert_eq!(groups[0].size.len(), 5);
    }

    #[test]
    fn test_compute_ranges() {
        let groups = parse_bubble_groups(&sample_data());
        let (x_min, x_max, y_min, y_max, s_max) = compute_ranges(&groups);
        assert_eq!(x_min, 10.0);
        assert_eq!(x_max, 50.0);
        assert_eq!(y_min, 5.0);
        assert_eq!(y_max, 25.0);
        assert_eq!(s_max, 300.0);
    }
}
