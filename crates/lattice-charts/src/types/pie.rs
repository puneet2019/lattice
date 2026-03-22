//! Pie chart SVG rendering.
//!
//! Renders pie charts with wedge segments using SVG arc paths,
//! percentage labels, and color palette assignment. Supports a
//! donut variant when the first series is named "donut".

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{series_color, svg_close, svg_open, svg_text, svg_title, xml_escape};
use std::f64::consts::PI;

/// Inner radius ratio for donut charts (0.0 = full pie, 0.5 = donut).
const DONUT_INNER_RATIO: f64 = 0.55;

/// Render pie chart data as an SVG string.
///
/// Uses the first data series for wedge values. Labels come from
/// `data.labels`. Each wedge is colored from the palette (or custom
/// series colors applied per-slice via label index).
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');

    let values = if let Some(series) = data.series.first() {
        &series.values
    } else {
        return format!("{}\n{}", svg_open(options), svg_close());
    };

    let total: f64 = values.iter().filter(|v| **v > 0.0).sum();
    if total <= 0.0 {
        svg.push_str(svg_close());
        return svg;
    }

    // Chart center and radius
    let cx = options.width as f64 / 2.0;
    let title_offset = if options.title.is_some() { 20.0 } else { 0.0 };
    let available_h = options.height as f64 - title_offset - 20.0;
    let available_w = options.width as f64 - 40.0;
    let radius = (available_w.min(available_h) / 2.0).min(160.0);
    let cy = title_offset + 20.0 + available_h / 2.0;

    // Detect donut mode
    let is_donut = data
        .series
        .first()
        .map(|s| s.name.to_lowercase().contains("donut"))
        .unwrap_or(false);
    let inner_r = if is_donut {
        radius * DONUT_INNER_RATIO
    } else {
        0.0
    };

    let mut start_angle: f64 = -PI / 2.0; // start from top

    for (i, &value) in values.iter().enumerate() {
        if value <= 0.0 {
            continue;
        }
        let fraction = value / total;
        let sweep_angle = fraction * 2.0 * PI;
        let end_angle = start_angle + sweep_angle;

        let color = data
            .series
            .first()
            .and_then(|s| s.color.as_deref())
            .unwrap_or_else(|| series_color(i));

        // SVG arc path
        let path = arc_path(cx, cy, radius, inner_r, start_angle, end_angle);
        svg.push_str(&format!(
            r##"<path d="{path}" fill="{color}" stroke="#ffffff" stroke-width="2"/>"##,
        ));
        svg.push('\n');

        // Label at midpoint of arc
        let mid_angle = start_angle + sweep_angle / 2.0;
        let label_r = if is_donut {
            (radius + inner_r) / 2.0
        } else {
            radius * 0.65
        };
        let lx = cx + label_r * mid_angle.cos();
        let ly = cy + label_r * mid_angle.sin();
        let pct = format!("{:.0}%", fraction * 100.0);

        // Show label text (category name) outside the pie
        let outer_lx = cx + (radius + 18.0) * mid_angle.cos();
        let outer_ly = cy + (radius + 18.0) * mid_angle.sin();
        let anchor = if mid_angle.cos() >= 0.0 {
            "start"
        } else {
            "end"
        };

        if let Some(label) = data.labels.get(i) {
            svg.push_str(&svg_text(
                outer_lx,
                outer_ly,
                anchor,
                10,
                "#333333",
                &xml_escape(label),
            ));
            svg.push('\n');
        }

        // Percentage inside the wedge
        if fraction >= 0.04 {
            svg.push_str(&svg_text(lx, ly + 4.0, "middle", 11, "#ffffff", &pct));
            svg.push('\n');
        }

        start_angle = end_angle;
    }

    svg.push_str(svg_close());
    svg
}

/// Build an SVG arc path for a pie wedge (or donut segment).
fn arc_path(cx: f64, cy: f64, outer_r: f64, inner_r: f64, start: f64, end: f64) -> String {
    let large_arc = if (end - start).abs() > PI { 1 } else { 0 };

    let ox1 = cx + outer_r * start.cos();
    let oy1 = cy + outer_r * start.sin();
    let ox2 = cx + outer_r * end.cos();
    let oy2 = cy + outer_r * end.sin();

    if inner_r > 0.0 {
        // Donut: outer arc + line + inner arc (reversed) + close
        let ix1 = cx + inner_r * start.cos();
        let iy1 = cy + inner_r * start.sin();
        let ix2 = cx + inner_r * end.cos();
        let iy2 = cy + inner_r * end.sin();

        format!(
            "M {ox1:.1},{oy1:.1} A {outer_r:.1},{outer_r:.1} 0 {large_arc} 1 {ox2:.1},{oy2:.1} L {ix2:.1},{iy2:.1} A {inner_r:.1},{inner_r:.1} 0 {large_arc} 0 {ix1:.1},{iy1:.1} Z"
        )
    } else {
        // Full pie wedge from center
        format!(
            "M {cx:.1},{cy:.1} L {ox1:.1},{oy1:.1} A {outer_r:.1},{outer_r:.1} 0 {large_arc} 1 {ox2:.1},{oy2:.1} Z"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::DataSeries;

    fn sample_data() -> ChartData {
        ChartData {
            labels: vec![
                "Chrome".into(),
                "Firefox".into(),
                "Safari".into(),
                "Edge".into(),
            ],
            series: vec![DataSeries {
                name: "Market Share".into(),
                values: vec![65.0, 15.0, 12.0, 8.0],
                color: None,
            }],
        }
    }

    #[test]
    fn test_pie_chart_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_pie_chart_contains_paths() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // 4 wedges = 4 path elements
        assert_eq!(svg.matches("<path").count(), 4);
    }

    #[test]
    fn test_pie_chart_contains_labels() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("Chrome"));
        assert!(svg.contains("Safari"));
    }

    #[test]
    fn test_pie_chart_contains_percentages() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("65%"));
        assert!(svg.contains("15%"));
    }

    #[test]
    fn test_pie_chart_with_title() {
        let opts = ChartOptions {
            title: Some("Browser Usage".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Browser Usage"));
    }

    #[test]
    fn test_pie_chart_donut_variant() {
        let data = ChartData {
            labels: vec!["A".into(), "B".into()],
            series: vec![DataSeries {
                name: "donut".into(),
                values: vec![60.0, 40.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // Donut paths contain inner arc with reversed sweep
        assert!(svg.contains("<path"));
    }

    #[test]
    fn test_pie_chart_empty_series() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_pie_chart_zero_values() {
        let data = ChartData {
            labels: vec!["A".into(), "B".into()],
            series: vec![DataSeries {
                name: "S".into(),
                values: vec![0.0, 0.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // No wedges for zero values
        assert_eq!(svg.matches("<path").count(), 0);
    }
}
