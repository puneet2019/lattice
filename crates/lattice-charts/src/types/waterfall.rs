//! Waterfall chart SVG rendering.
//!
//! Renders a waterfall chart showing the cumulative effect of positive
//! and negative values. Green bars indicate positive changes, red bars
//! indicate negative changes, and gray bars show the running total.
//! Connector lines between bars illustrate the cumulative level.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    Margins, compute_axis_scale, format_axis_value,
    svg_axis_labels, svg_close, svg_open, svg_text, svg_title, xml_escape,
};

/// Default color for positive (increase) bars.
const COLOR_POSITIVE: &str = "#59a14f";
/// Default color for negative (decrease) bars.
const COLOR_NEGATIVE: &str = "#e15759";
/// Default color for total/subtotal bars.
const COLOR_TOTAL: &str = "#888888";
/// Connector line color.
const COLOR_CONNECTOR: &str = "#999999";

/// Render waterfall chart data as an SVG string.
///
/// Uses the first data series for the waterfall values. A value is treated
/// as a **total bar** (gray, anchored to zero) if its label starts with
/// `"total"` (case-insensitive). Otherwise, positive values render green
/// and negative values render red, stacking cumulatively.
///
/// When a custom `color_palette` is provided, it overrides the default
/// green/red/gray colors in order: `[positive, negative, total]`.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let values: &[f64] = data
        .series
        .first()
        .map(|s| s.values.as_slice())
        .unwrap_or(&[]);

    // Determine which bars are "total" bars vs incremental.
    let is_total: Vec<bool> = data
        .labels
        .iter()
        .map(|l| l.trim().to_lowercase().starts_with("total"))
        .collect();

    // Compute cumulative positions and bar ranges.
    // Each bar has a (bottom, top) in data-space.
    let mut bars: Vec<(f64, f64)> = Vec::with_capacity(values.len());
    let mut running = 0.0_f64;
    for (i, &v) in values.iter().enumerate() {
        if is_total.get(i).copied().unwrap_or(false) {
            // Total bar goes from 0 to the running total
            bars.push((0.0, running));
        } else {
            let old = running;
            running += v;
            if v >= 0.0 {
                bars.push((old, running));
            } else {
                bars.push((running, old));
            }
        }
    }

    // Compute y-axis scale
    let mut all_vals: Vec<f64> = bars.iter().flat_map(|&(lo, hi)| vec![lo, hi]).collect();
    all_vals.push(0.0);
    let dmin = all_vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let dmax = all_vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let (dmin, dmax) = if dmin > dmax { (0.0, 1.0) } else { (dmin.min(0.0), dmax.max(0.0)) };
    let scale = compute_axis_scale(dmin, dmax);
    let y_range = scale.max - scale.min;

    let n = values.len().max(1);
    let cat_width = pw / n as f64;
    let bar_gap = cat_width * 0.15;
    let bar_width = cat_width - bar_gap * 2.0;

    // Resolve colors
    let custom_palette = options.color_palette.as_deref();
    let color_pos = custom_palette
        .and_then(|p| p.first().map(|s| s.as_str()))
        .unwrap_or(COLOR_POSITIVE);
    let color_neg = custom_palette
        .and_then(|p| p.get(1).map(|s| s.as_str()))
        .unwrap_or(COLOR_NEGATIVE);
    let color_total = custom_palette
        .and_then(|p| p.get(2).map(|s| s.as_str()))
        .unwrap_or(COLOR_TOTAL);

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push_str(&svg_axis_labels(options, &margins));

    // Grid lines and y-axis labels
    if options.show_grid {
        for &tick in &scale.ticks {
            let frac = if y_range.abs() > f64::EPSILON {
                (tick - scale.min) / y_range
            } else {
                0.5
            };
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

    // Draw bars and connectors
    let mut running = 0.0_f64;
    for (i, &v) in values.iter().enumerate() {
        let total = is_total.get(i).copied().unwrap_or(false);
        let (bar_lo, bar_hi) = bars[i];

        let color = if total {
            color_total
        } else if v >= 0.0 {
            color_pos
        } else {
            color_neg
        };

        let frac_lo = if y_range.abs() > f64::EPSILON {
            (bar_lo - scale.min) / y_range
        } else {
            0.5
        };
        let frac_hi = if y_range.abs() > f64::EPSILON {
            (bar_hi - scale.min) / y_range
        } else {
            0.5
        };

        let px_top = margins.top + ph * (1.0 - frac_hi);
        let px_bot = margins.top + ph * (1.0 - frac_lo);
        let bar_h = (px_bot - px_top).abs().max(1.0);
        let bar_y = px_top.min(px_bot);
        let bar_x = margins.left + i as f64 * cat_width + bar_gap;

        svg.push_str(&format!(
            r#"<rect x="{bx:.1}" y="{by:.1}" width="{bw:.1}" height="{bh:.1}" fill="{color}" rx="1"/>"#,
            bx = bar_x,
            by = bar_y,
            bw = bar_width,
            bh = bar_h,
        ));
        svg.push('\n');

        // Data label
        if options.show_data_labels {
            let label_y = if v >= 0.0 { bar_y - 4.0 } else { bar_y + bar_h + 12.0 };
            svg.push_str(&svg_text(
                bar_x + bar_width / 2.0,
                label_y,
                "middle",
                9,
                "#333333",
                &format_axis_value(v),
            ));
            svg.push('\n');
        }

        // Connector line to next bar
        if !total {
            running += v;
        }
        if i + 1 < values.len() {
            let connector_val = running;
            let conn_frac = if y_range.abs() > f64::EPSILON {
                (connector_val - scale.min) / y_range
            } else {
                0.5
            };
            let conn_y = margins.top + ph * (1.0 - conn_frac);
            let x1 = bar_x + bar_width;
            let x2 = margins.left + (i + 1) as f64 * cat_width + bar_gap;
            svg.push_str(&format!(
                r##"<line x1="{x1:.1}" y1="{conn_y:.1}" x2="{x2:.1}" y2="{conn_y:.1}" stroke="{COLOR_CONNECTOR}" stroke-width="1" stroke-dasharray="3,2"/>"##,
            ));
            svg.push('\n');
        }

        // X-axis label
        if let Some(label) = data.labels.get(i) {
            let label_x = margins.left + i as f64 * cat_width + cat_width / 2.0;
            let label_y = margins.top + ph + 18.0;
            svg.push_str(&svg_text(
                label_x,
                label_y,
                "middle",
                10,
                "#666666",
                &xml_escape(label),
            ));
            svg.push('\n');
        }
    }

    // X-axis baseline
    let zero_frac = if y_range.abs() > f64::EPSILON {
        (0.0 - scale.min) / y_range
    } else {
        0.5
    };
    let baseline_y = margins.top + ph * (1.0 - zero_frac);
    svg.push_str(&format!(
        r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="#333333" stroke-width="1"/>"##,
        x1 = margins.left,
        x2 = margins.left + pw,
        y = baseline_y,
    ));
    svg.push('\n');

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
                "Revenue".into(),
                "COGS".into(),
                "Gross Profit".into(),
                "OpEx".into(),
                "Total".into(),
            ],
            series: vec![DataSeries {
                name: "Waterfall".into(),
                values: vec![500.0, -200.0, 100.0, -150.0, 50.0],
                color: None,
            }],
        }
    }

    #[test]
    fn test_waterfall_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_waterfall_contains_bars() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // 5 values = 5 bars
        assert_eq!(svg.matches("<rect x=").count(), 5);
    }

    #[test]
    fn test_waterfall_positive_color() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains(COLOR_POSITIVE));
    }

    #[test]
    fn test_waterfall_negative_color() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains(COLOR_NEGATIVE));
    }

    #[test]
    fn test_waterfall_total_bar() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // "Total" label triggers total bar (gray)
        assert!(svg.contains(COLOR_TOTAL));
    }

    #[test]
    fn test_waterfall_connector_lines() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // Connectors use dashed lines
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn test_waterfall_x_labels() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("Revenue"));
        assert!(svg.contains("COGS"));
        assert!(svg.contains("Total"));
    }

    #[test]
    fn test_waterfall_with_title() {
        let opts = ChartOptions {
            title: Some("P&L Breakdown".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("P&amp;L Breakdown"));
    }

    #[test]
    fn test_waterfall_data_labels() {
        let opts = ChartOptions {
            show_data_labels: true,
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("500"));
    }

    #[test]
    fn test_waterfall_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_waterfall_custom_palette() {
        let opts = ChartOptions {
            color_palette: Some(vec![
                "#00ff00".into(),
                "#ff0000".into(),
                "#0000ff".into(),
            ]),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("#00ff00")); // positive
        assert!(svg.contains("#ff0000")); // negative
        assert!(svg.contains("#0000ff")); // total
    }

    #[test]
    fn test_waterfall_all_positive() {
        let data = ChartData {
            labels: vec!["A".into(), "B".into(), "C".into()],
            series: vec![DataSeries {
                name: "S".into(),
                values: vec![10.0, 20.0, 30.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // All bars should be green (positive)
        assert!(!svg.contains(COLOR_NEGATIVE));
        assert!(!svg.contains(COLOR_TOTAL));
    }
}
