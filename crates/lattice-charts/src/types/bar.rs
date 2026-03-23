//! Bar chart SVG rendering.
//!
//! Renders vertical bar charts with support for grouped multiple series,
//! x-axis category labels, y-axis scale with grid lines, and legends.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    Margins, compute_axis_scale, data_range_with_zero, series_color, svg_axis_labels, svg_close,
    svg_grid_lines, svg_legend, svg_open, svg_text, svg_title, xml_escape,
};

/// Render bar chart data as an SVG string.
///
/// Produces a vertical grouped bar chart (or stacked when `options.stacked`
/// is true). When grouped, multiple series are placed side by side within
/// each category. When stacked, series are layered on top of each other.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    if options.stacked {
        render_stacked(data, options)
    } else {
        render_grouped(data, options)
    }
}

/// Render a grouped (side-by-side) bar chart.
fn render_grouped(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    // Compute y-axis scale (always include zero for bar charts)
    let (dmin, dmax) = data_range_with_zero(data);
    let scale = compute_axis_scale(dmin, dmax);
    let y_range = scale.max - scale.min;

    let n_categories = data.labels.len().max(1);
    let n_series = data.series.len().max(1);
    let category_width = pw / n_categories as f64;
    let bar_gap = category_width * 0.1;
    let group_width = category_width - bar_gap * 2.0;
    let bar_width = group_width / n_series as f64;

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');
    svg.push_str(&svg_grid_lines(&scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    // Draw bars
    for (ci, label) in data.labels.iter().enumerate() {
        let cat_x = margins.left + ci as f64 * category_width;

        for (si, series) in data.series.iter().enumerate() {
            let value = series.values.get(ci).copied().unwrap_or(0.0);
            let color = series.color.as_deref().unwrap_or_else(|| series_color(si));

            // Calculate bar position and height
            let frac = if y_range.abs() > f64::EPSILON {
                (value - scale.min) / y_range
            } else {
                0.5
            };
            let zero_frac = if y_range.abs() > f64::EPSILON {
                (0.0 - scale.min) / y_range
            } else {
                0.5
            };

            let bar_top = margins.top + ph * (1.0 - frac);
            let zero_y = margins.top + ph * (1.0 - zero_frac);
            let bar_x = cat_x + bar_gap + si as f64 * bar_width;
            let bar_h = (zero_y - bar_top).abs();
            let bar_y = bar_top.min(zero_y);

            svg.push_str(&format!(
                r#"<rect x="{bx:.1}" y="{by:.1}" width="{bw:.1}" height="{bh:.1}" fill="{color}" rx="1"/>"#,
                bx = bar_x,
                by = bar_y,
                bw = bar_width * 0.9,
                bh = bar_h,
            ));
            svg.push('\n');
        }

        // X-axis label
        let label_x = cat_x + category_width / 2.0;
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

    svg.push_str(&svg_legend(data, &margins, options));
    svg.push_str(svg_close());
    svg
}

/// Render a stacked bar chart. Series values accumulate on top of each other.
fn render_stacked(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    // Compute stacked totals per category for the y-axis scale.
    let n_categories = data.labels.len().max(1);
    let mut stacked_max = 0.0_f64;
    for ci in 0..n_categories {
        let total: f64 = data
            .series
            .iter()
            .map(|s| s.values.get(ci).copied().unwrap_or(0.0).max(0.0))
            .sum();
        if total > stacked_max {
            stacked_max = total;
        }
    }

    let scale = compute_axis_scale(0.0, stacked_max);
    let y_range = scale.max - scale.min;
    let category_width = pw / n_categories as f64;
    let bar_gap = category_width * 0.15;
    let bar_width = category_width - bar_gap * 2.0;

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');
    svg.push_str(&svg_grid_lines(&scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    // Draw stacked bars per category
    for (ci, label) in data.labels.iter().enumerate() {
        let cat_x = margins.left + ci as f64 * category_width;
        let bar_x = cat_x + bar_gap;

        let mut cumulative = 0.0_f64;
        for (si, series) in data.series.iter().enumerate() {
            let value = series.values.get(ci).copied().unwrap_or(0.0).max(0.0);
            if value <= 0.0 {
                continue;
            }
            let color = series.color.as_deref().unwrap_or_else(|| series_color(si));

            let bottom_frac = if y_range.abs() > f64::EPSILON {
                (cumulative - scale.min) / y_range
            } else {
                0.0
            };
            cumulative += value;
            let top_frac = if y_range.abs() > f64::EPSILON {
                (cumulative - scale.min) / y_range
            } else {
                1.0
            };

            let bar_y = margins.top + ph * (1.0 - top_frac);
            let bar_bottom = margins.top + ph * (1.0 - bottom_frac);
            let bar_h = (bar_bottom - bar_y).abs();

            svg.push_str(&format!(
                r#"<rect x="{bx:.1}" y="{by:.1}" width="{bw:.1}" height="{bh:.1}" fill="{color}" rx="1"/>"#,
                bx = bar_x,
                by = bar_y,
                bw = bar_width,
                bh = bar_h,
            ));
            svg.push('\n');
        }

        // X-axis label
        let label_x = cat_x + category_width / 2.0;
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

    // X-axis baseline
    let baseline_y = margins.top + ph;
    svg.push_str(&format!(
        r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="#333333" stroke-width="1"/>"##,
        x1 = margins.left,
        x2 = margins.left + pw,
        y = baseline_y,
    ));
    svg.push('\n');

    svg.push_str(&svg_legend(data, &margins, options));
    svg.push_str(svg_close());
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::DataSeries;

    fn sample_data() -> ChartData {
        ChartData {
            labels: vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
            series: vec![DataSeries {
                name: "Revenue".into(),
                values: vec![100.0, 200.0, 150.0, 250.0],
                color: None,
            }],
        }
    }

    fn multi_series_data() -> ChartData {
        ChartData {
            labels: vec!["Jan".into(), "Feb".into(), "Mar".into()],
            series: vec![
                DataSeries {
                    name: "Product A".into(),
                    values: vec![30.0, 50.0, 40.0],
                    color: Some("#ff0000".into()),
                },
                DataSeries {
                    name: "Product B".into(),
                    values: vec![20.0, 35.0, 45.0],
                    color: Some("#0000ff".into()),
                },
            ],
        }
    }

    #[test]
    fn test_bar_chart_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_bar_chart_contains_bars() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // Should contain rect elements for bars
        assert!(svg.contains("<rect"));
        // Should contain x-axis labels
        assert!(svg.contains("Q1"));
        assert!(svg.contains("Q4"));
    }

    #[test]
    fn test_bar_chart_with_title() {
        let opts = ChartOptions {
            title: Some("Sales Report".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Sales Report"));
    }

    #[test]
    fn test_bar_chart_multi_series() {
        let svg = render(&multi_series_data(), &ChartOptions::default());
        assert!(svg.contains("#ff0000"));
        assert!(svg.contains("#0000ff"));
        assert!(svg.contains("Jan"));
        assert!(svg.contains("Mar"));
    }

    #[test]
    fn test_bar_chart_has_grid_lines() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("<line"));
    }

    #[test]
    fn test_bar_chart_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }
}
