//! Radar (spider) chart SVG rendering.
//!
//! Renders a multi-axis radar chart with spokes radiating from the center,
//! concentric grid circles, filled polygons for each data series, and
//! axis labels at each spoke end.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    palette_color, svg_close, svg_open, svg_text, svg_title, xml_escape,
};
use std::f64::consts::PI;

/// Number of concentric grid circles.
const GRID_RINGS: usize = 5;

/// Render radar chart data as an SVG string.
///
/// Each label defines a spoke (axis) of the radar chart. Each data series
/// is rendered as a filled polygon connecting its values on each axis.
/// Grid circles are drawn at regular intervals from center to max.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');

    let n_axes = data.labels.len();
    if n_axes < 3 {
        // Radar charts need at least 3 axes to form a polygon
        svg.push_str(svg_close());
        return svg;
    }

    // Chart center and radius
    let title_offset = if options.title.is_some() { 30.0 } else { 0.0 };
    let subtitle_offset = if options.subtitle.is_some() { 18.0 } else { 0.0 };
    let legend_margin = if options.show_legend && data.series.len() > 1 {
        100.0
    } else {
        20.0
    };
    let available_w = options.width as f64 - 60.0 - legend_margin;
    let available_h = options.height as f64 - title_offset - subtitle_offset - 60.0;
    let radius = (available_w.min(available_h) / 2.0).max(40.0);
    let cx = 30.0 + available_w / 2.0;
    let cy = title_offset + subtitle_offset + 30.0 + available_h / 2.0;

    let angle_step = 2.0 * PI / n_axes as f64;

    // Find max value across all series for normalization
    let max_val = data
        .series
        .iter()
        .flat_map(|s| &s.values)
        .cloned()
        .fold(0.0_f64, f64::max)
        .max(1.0);

    // Draw concentric grid circles
    if options.show_grid {
        for ring in 1..=GRID_RINGS {
            let r = radius * ring as f64 / GRID_RINGS as f64;
            svg.push_str(&format!(
                r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="none" stroke="#e0e0e0" stroke-width="1"/>"##,
            ));
            svg.push('\n');
        }
    }

    // Draw axis spokes and labels
    for i in 0..n_axes {
        let angle = -PI / 2.0 + i as f64 * angle_step;
        let x_end = cx + radius * angle.cos();
        let y_end = cy + radius * angle.sin();

        // Spoke line
        svg.push_str(&format!(
            r##"<line x1="{cx:.1}" y1="{cy:.1}" x2="{x_end:.1}" y2="{y_end:.1}" stroke="#cccccc" stroke-width="1"/>"##,
        ));
        svg.push('\n');

        // Axis label
        if let Some(label) = data.labels.get(i) {
            let label_r = radius + 14.0;
            let lx = cx + label_r * angle.cos();
            let ly = cy + label_r * angle.sin() + 4.0;
            let anchor = if angle.cos().abs() < 0.01 {
                "middle"
            } else if angle.cos() > 0.0 {
                "start"
            } else {
                "end"
            };
            svg.push_str(&svg_text(lx, ly, anchor, 10, "#333333", &xml_escape(label)));
            svg.push('\n');
        }
    }

    // Draw data polygons
    let custom_palette = options.color_palette.as_deref();
    for (si, series) in data.series.iter().enumerate() {
        let color = series
            .color
            .as_deref()
            .unwrap_or_else(|| palette_color(si, custom_palette));

        let mut points = String::new();
        for (ai, &val) in series.values.iter().take(n_axes).enumerate() {
            let angle = -PI / 2.0 + ai as f64 * angle_step;
            let normalized = (val / max_val).clamp(0.0, 1.0);
            let r = radius * normalized;
            let x = cx + r * angle.cos();
            let y = cy + r * angle.sin();
            if !points.is_empty() {
                points.push(' ');
            }
            points.push_str(&format!("{x:.1},{y:.1}"));
        }

        // Filled polygon with semi-transparent fill
        svg.push_str(&format!(
            r#"<polygon points="{points}" fill="{color}" fill-opacity="0.2" stroke="{color}" stroke-width="2"/>"#,
        ));
        svg.push('\n');

        // Draw data point markers
        for (ai, &val) in series.values.iter().take(n_axes).enumerate() {
            let angle = -PI / 2.0 + ai as f64 * angle_step;
            let normalized = (val / max_val).clamp(0.0, 1.0);
            let r = radius * normalized;
            let x = cx + r * angle.cos();
            let y = cy + r * angle.sin();
            svg.push_str(&format!(
                r#"<circle cx="{x:.1}" cy="{y:.1}" r="3" fill="{color}"/>"#,
            ));
            svg.push('\n');
        }
    }

    // Legend
    if options.show_legend && data.series.len() > 1 {
        let legend_x = cx + radius + 30.0;
        for (i, s) in data.series.iter().enumerate() {
            let color = s
                .color
                .as_deref()
                .unwrap_or_else(|| palette_color(i, custom_palette));
            let y = cy - radius + 10.0 + i as f64 * 20.0;
            svg.push_str(&format!(
                r#"<rect x="{legend_x:.1}" y="{ry:.1}" width="12" height="12" fill="{color}" rx="2"/>"#,
                ry = y - 9.0,
            ));
            svg.push_str(&svg_text(
                legend_x + 16.0,
                y,
                "start",
                11,
                "#333333",
                &s.name,
            ));
            svg.push('\n');
        }
    }

    svg.push_str(svg_close());
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::DataSeries;

    fn sample_data() -> ChartData {
        ChartData {
            labels: vec![
                "Speed".into(),
                "Power".into(),
                "Range".into(),
                "Armor".into(),
                "Magic".into(),
            ],
            series: vec![DataSeries {
                name: "Warrior".into(),
                values: vec![7.0, 9.0, 3.0, 8.0, 2.0],
                color: None,
            }],
        }
    }

    fn multi_series_data() -> ChartData {
        ChartData {
            labels: vec![
                "Speed".into(),
                "Power".into(),
                "Range".into(),
                "Armor".into(),
                "Magic".into(),
            ],
            series: vec![
                DataSeries {
                    name: "Warrior".into(),
                    values: vec![7.0, 9.0, 3.0, 8.0, 2.0],
                    color: None,
                },
                DataSeries {
                    name: "Mage".into(),
                    values: vec![3.0, 2.0, 5.0, 1.0, 10.0],
                    color: None,
                },
            ],
        }
    }

    #[test]
    fn test_radar_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_radar_contains_polygon() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("<polygon"));
    }

    #[test]
    fn test_radar_contains_grid_circles() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // 5 grid rings
        let circle_count = svg.matches(r##"fill="none" stroke="#e0e0e0""##).count();
        assert_eq!(circle_count, GRID_RINGS);
    }

    #[test]
    fn test_radar_contains_spokes() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // 5 axes = 5 spoke lines (+ circles for points)
        let spoke_count = svg.matches(r##"stroke="#cccccc""##).count();
        assert_eq!(spoke_count, 5);
    }

    #[test]
    fn test_radar_contains_axis_labels() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("Speed"));
        assert!(svg.contains("Power"));
        assert!(svg.contains("Magic"));
    }

    #[test]
    fn test_radar_data_points() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // 5 data point circles (separate from grid circles)
        let point_circles = svg.matches(r#"r="3""#).count();
        assert_eq!(point_circles, 5);
    }

    #[test]
    fn test_radar_multi_series() {
        let svg = render(&multi_series_data(), &ChartOptions::default());
        // 2 polygons
        assert_eq!(svg.matches("<polygon").count(), 2);
        // 10 data point circles (5 per series)
        assert_eq!(svg.matches(r#"r="3""#).count(), 10);
    }

    #[test]
    fn test_radar_multi_series_legend() {
        let svg = render(&multi_series_data(), &ChartOptions::default());
        assert!(svg.contains("Warrior"));
        assert!(svg.contains("Mage"));
    }

    #[test]
    fn test_radar_with_title() {
        let opts = ChartOptions {
            title: Some("Character Stats".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Character Stats"));
    }

    #[test]
    fn test_radar_too_few_axes() {
        let data = ChartData {
            labels: vec!["A".into(), "B".into()],
            series: vec![DataSeries {
                name: "S".into(),
                values: vec![5.0, 3.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // No polygon for < 3 axes
        assert!(!svg.contains("<polygon"));
    }

    #[test]
    fn test_radar_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_radar_semi_transparent_fill() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("fill-opacity=\"0.2\""));
    }
}
