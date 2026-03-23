//! Gauge (speedometer) chart SVG rendering.
//!
//! Renders a single-value gauge chart with a semicircular arc,
//! colored zones (green/yellow/red), a needle pointing to the
//! current value, and the value displayed as large text below.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{svg_close, svg_open, svg_text, svg_title};
use std::f64::consts::PI;

/// Default gauge zone colors: green, yellow, red.
const ZONE_COLORS: [&str; 3] = ["#59a14f", "#edc948", "#e15759"];

/// Render gauge chart data as an SVG string.
///
/// Uses the first value of the first series as the gauge value.
/// The gauge range defaults to 0..100 but is auto-scaled based on the value.
///
/// When a custom `color_palette` is provided with 3 colors, they
/// replace the default green/yellow/red zone colors.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let mut svg = String::with_capacity(1024);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');

    // Extract gauge value
    let value = data
        .series
        .first()
        .and_then(|s| s.values.first().copied())
        .unwrap_or(0.0);

    // Determine gauge range (min..max)
    let gauge_min = 0.0_f64;
    let gauge_max = if value <= 100.0 { 100.0 } else { nice_max(value) };

    // Chart center and radius
    let title_offset = if options.title.is_some() { 30.0 } else { 0.0 };
    let subtitle_offset = if options.subtitle.is_some() { 18.0 } else { 0.0 };
    let top_margin = title_offset + subtitle_offset + 20.0;
    let available_w = options.width as f64 - 40.0;
    let available_h = options.height as f64 - top_margin - 60.0;
    // Gauge is a half-circle, so height = radius
    let radius = (available_w / 2.0).min(available_h).min(160.0).max(40.0);
    let cx = options.width as f64 / 2.0;
    let cy = top_margin + radius + 10.0;

    // Arc sweep: from PI (left) to 0 (right) — a top semicircle
    let arc_start = PI;
    let arc_thickness = radius * 0.18;

    // Resolve zone colors
    let custom = options.color_palette.as_deref();
    let zone_colors: [&str; 3] = [
        custom.and_then(|p| p.first().map(|s| s.as_str())).unwrap_or(ZONE_COLORS[0]),
        custom.and_then(|p| p.get(1).map(|s| s.as_str())).unwrap_or(ZONE_COLORS[1]),
        custom.and_then(|p| p.get(2).map(|s| s.as_str())).unwrap_or(ZONE_COLORS[2]),
    ];

    // Draw three zone arcs (each covering 1/3 of the semicircle)
    let zone_fractions = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];
    for z in 0..3 {
        let start_frac = zone_fractions[z];
        let end_frac = zone_fractions[z + 1];
        let a1 = arc_start - start_frac * PI;
        let a2 = arc_start - end_frac * PI;

        let outer_r = radius;
        let inner_r = radius - arc_thickness;

        let path = gauge_arc_path(cx, cy, outer_r, inner_r, a1, a2);
        svg.push_str(&format!(
            r#"<path d="{path}" fill="{}" stroke="none"/>"#,
            zone_colors[z],
        ));
        svg.push('\n');
    }

    // Draw tick marks and labels
    let n_ticks = 5;
    for i in 0..=n_ticks {
        let frac = i as f64 / n_ticks as f64;
        let angle = arc_start - frac * PI;
        let tick_outer = radius + 4.0;
        let tick_inner = radius - arc_thickness - 4.0;

        let x1 = cx + tick_outer * angle.cos();
        let y1 = cy - tick_outer * angle.sin();
        let x2 = cx + tick_inner * angle.cos();
        let y2 = cy - tick_inner * angle.sin();

        svg.push_str(&format!(
            r##"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="#666666" stroke-width="1"/>"##,
        ));
        svg.push('\n');

        // Tick label
        let label_r = radius + 16.0;
        let lx = cx + label_r * angle.cos();
        let ly = cy - label_r * angle.sin() + 4.0;
        let tick_val = gauge_min + frac * (gauge_max - gauge_min);
        let label = if tick_val.fract().abs() < 1e-9 {
            format!("{}", tick_val as i64)
        } else {
            format!("{:.0}", tick_val)
        };
        svg.push_str(&svg_text(lx, ly, "middle", 10, "#666666", &label));
        svg.push('\n');
    }

    // Draw needle
    let clamped = value.clamp(gauge_min, gauge_max);
    let needle_frac = (clamped - gauge_min) / (gauge_max - gauge_min);
    let needle_angle = arc_start - needle_frac * PI;
    let needle_len = radius - arc_thickness * 0.3;

    let nx = cx + needle_len * needle_angle.cos();
    let ny = cy - needle_len * needle_angle.sin();

    // Needle as a thin triangle
    let perp = needle_angle + PI / 2.0;
    let base_w = 4.0;
    let bx1 = cx + base_w * perp.cos();
    let by1 = cy - base_w * perp.sin();
    let bx2 = cx - base_w * perp.cos();
    let by2 = cy + base_w * perp.sin();

    svg.push_str(&format!(
        r##"<polygon points="{nx:.1},{ny:.1} {bx1:.1},{by1:.1} {bx2:.1},{by2:.1}" fill="#333333"/>"##,
    ));
    svg.push('\n');

    // Needle center cap
    svg.push_str(&format!(
        r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="6" fill="#333333"/>"##,
    ));
    svg.push('\n');

    // Value display
    let value_text = if value.fract().abs() < 1e-9 {
        format!("{}", value as i64)
    } else {
        format!("{:.1}", value)
    };
    svg.push_str(&svg_text(cx, cy + 35.0, "middle", 24, "#333333", &value_text));
    svg.push('\n');

    // Optional label from first series name
    if let Some(series) = data.series.first() {
        if !series.name.is_empty() {
            svg.push_str(&svg_text(cx, cy + 55.0, "middle", 12, "#666666", &series.name));
            svg.push('\n');
        }
    }

    svg.push_str(svg_close());
    svg
}

/// Compute a nice round maximum for the gauge range.
fn nice_max(value: f64) -> f64 {
    let magnitude = 10_f64.powf(value.log10().floor());
    let normalized = value / magnitude;
    let nice = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * magnitude
}

/// Build an SVG arc path for a gauge zone segment (thick arc band).
fn gauge_arc_path(
    cx: f64,
    cy: f64,
    outer_r: f64,
    inner_r: f64,
    start_angle: f64,
    end_angle: f64,
) -> String {
    // Angles here: 0 = right, PI = left, measured counter-clockwise from right
    // For the gauge we render the arc in screen coordinates where y is flipped
    let ox1 = cx + outer_r * start_angle.cos();
    let oy1 = cy - outer_r * start_angle.sin();
    let ox2 = cx + outer_r * end_angle.cos();
    let oy2 = cy - outer_r * end_angle.sin();
    let ix1 = cx + inner_r * start_angle.cos();
    let iy1 = cy - inner_r * start_angle.sin();
    let ix2 = cx + inner_r * end_angle.cos();
    let iy2 = cy - inner_r * end_angle.sin();

    let sweep_degrees = ((start_angle - end_angle) * 180.0 / PI).abs();
    let large_arc = if sweep_degrees > 180.0 { 1 } else { 0 };

    // Outer arc goes clockwise (sweep-flag=0 in SVG since we flip y),
    // inner arc goes counter-clockwise
    format!(
        "M {ox1:.1},{oy1:.1} A {outer_r:.1},{outer_r:.1} 0 {large_arc} 1 {ox2:.1},{oy2:.1} L {ix2:.1},{iy2:.1} A {inner_r:.1},{inner_r:.1} 0 {large_arc} 0 {ix1:.1},{iy1:.1} Z"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::DataSeries;

    fn sample_data(value: f64) -> ChartData {
        ChartData {
            labels: vec![],
            series: vec![DataSeries {
                name: "Speed".into(),
                values: vec![value],
                color: None,
            }],
        }
    }

    #[test]
    fn test_gauge_valid_svg() {
        let svg = render(&sample_data(75.0), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_gauge_contains_zones() {
        let svg = render(&sample_data(50.0), &ChartOptions::default());
        // 3 zone arcs
        assert_eq!(svg.matches("<path").count(), 3);
    }

    #[test]
    fn test_gauge_zone_colors() {
        let svg = render(&sample_data(50.0), &ChartOptions::default());
        assert!(svg.contains(ZONE_COLORS[0])); // green
        assert!(svg.contains(ZONE_COLORS[1])); // yellow
        assert!(svg.contains(ZONE_COLORS[2])); // red
    }

    #[test]
    fn test_gauge_contains_needle() {
        let svg = render(&sample_data(75.0), &ChartOptions::default());
        // Needle is a polygon
        assert!(svg.contains("<polygon"));
        // Needle center cap
        assert!(svg.contains(r#"r="6""#));
    }

    #[test]
    fn test_gauge_displays_value() {
        let svg = render(&sample_data(75.0), &ChartOptions::default());
        assert!(svg.contains("75"));
    }

    #[test]
    fn test_gauge_displays_label() {
        let svg = render(&sample_data(50.0), &ChartOptions::default());
        assert!(svg.contains("Speed"));
    }

    #[test]
    fn test_gauge_with_title() {
        let opts = ChartOptions {
            title: Some("Performance".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(60.0), &opts);
        assert!(svg.contains("Performance"));
    }

    #[test]
    fn test_gauge_tick_marks() {
        let svg = render(&sample_data(50.0), &ChartOptions::default());
        // 6 tick lines (0, 20, 40, 60, 80, 100) + zone arcs
        let tick_count = svg.matches(r##"stroke="#666666""##).count();
        assert_eq!(tick_count, 6);
    }

    #[test]
    fn test_gauge_zero_value() {
        let svg = render(&sample_data(0.0), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_gauge_max_value() {
        let svg = render(&sample_data(100.0), &ChartOptions::default());
        assert!(svg.contains("100"));
    }

    #[test]
    fn test_gauge_exceeds_100() {
        let svg = render(&sample_data(250.0), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("250"));
    }

    #[test]
    fn test_gauge_custom_zone_colors() {
        let opts = ChartOptions {
            color_palette: Some(vec![
                "#0000ff".into(),
                "#00ff00".into(),
                "#ff00ff".into(),
            ]),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(50.0), &opts);
        assert!(svg.contains("#0000ff"));
        assert!(svg.contains("#00ff00"));
        assert!(svg.contains("#ff00ff"));
    }

    #[test]
    fn test_gauge_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_nice_max() {
        assert_eq!(nice_max(75.0), 100.0);
        assert_eq!(nice_max(150.0), 200.0);
        assert_eq!(nice_max(450.0), 500.0);
        assert_eq!(nice_max(800.0), 1000.0);
    }
}
