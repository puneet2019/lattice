//! Histogram SVG rendering.
//!
//! Renders histograms from a single data series by auto-computing bins
//! using Sturges' rule and drawing adjacent bars with no gaps. The y-axis
//! shows frequency counts and the x-axis shows bin range boundaries.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    compute_axis_scale, format_axis_value, series_color, svg_axis_labels, svg_close,
    svg_grid_lines, svg_open, svg_text, svg_title, Margins,
};

/// Compute the number of bins using Sturges' rule: ceil(1 + 3.322 * log10(n)).
fn sturges_bins(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let k = 1.0 + 3.322 * (n as f64).log10();
    k.ceil().max(1.0) as usize
}

/// A single histogram bin with its range and count.
#[derive(Debug)]
struct Bin {
    /// Lower bound (inclusive).
    lo: f64,
    /// Upper bound (exclusive, except for the last bin).
    hi: f64,
    /// Number of values in this bin.
    count: usize,
}

/// Build histogram bins from raw data values.
fn build_bins(values: &[f64], n_bins: usize) -> Vec<Bin> {
    if values.is_empty() || n_bins == 0 {
        return vec![];
    }

    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &v in values {
        if v < min { min = v; }
        if v > max { max = v; }
    }

    // Handle case where all values are equal
    if (max - min).abs() < f64::EPSILON {
        return vec![Bin { lo: min - 0.5, hi: max + 0.5, count: values.len() }];
    }

    let bin_width = (max - min) / n_bins as f64;
    let mut bins: Vec<Bin> = (0..n_bins)
        .map(|i| Bin {
            lo: min + i as f64 * bin_width,
            hi: min + (i + 1) as f64 * bin_width,
            count: 0,
        })
        .collect();

    for &v in values {
        let idx = ((v - min) / bin_width).floor() as usize;
        // Clamp to last bin (inclusive upper bound on the last bin)
        let idx = idx.min(n_bins - 1);
        bins[idx].count += 1;
    }

    bins
}

/// Render histogram data as an SVG string.
///
/// Takes the first data series and auto-computes bins using Sturges' rule.
/// Bars are rendered adjacent (no gap) to visualize the distribution.
/// The y-axis shows frequency count and x-axis shows bin boundaries.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');

    // Use first series for the histogram
    let values: &[f64] = data
        .series
        .first()
        .map(|s| s.values.as_slice())
        .unwrap_or(&[]);

    let n_bins = sturges_bins(values.len());
    let bins = build_bins(values, n_bins);

    if bins.is_empty() {
        svg.push_str(&svg_axis_labels(options, &margins));
        svg.push_str(svg_close());
        return svg;
    }

    // Compute y-axis scale from frequency counts (always include zero)
    let max_count = bins.iter().map(|b| b.count).max().unwrap_or(0);
    let y_scale = compute_axis_scale(0.0, max_count as f64);
    let y_range = y_scale.max - y_scale.min;

    // Grid lines (y-axis)
    svg.push_str(&svg_grid_lines(&y_scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    // Compute x-axis range from bin boundaries
    let x_min = bins.first().map(|b| b.lo).unwrap_or(0.0);
    let x_max = bins.last().map(|b| b.hi).unwrap_or(1.0);
    let x_range = x_max - x_min;

    let color = data
        .series
        .first()
        .and_then(|s| s.color.as_deref())
        .unwrap_or_else(|| series_color(0));

    // Draw bars (adjacent, no gap)
    for bin in &bins {
        let x_frac_lo = if x_range.abs() > f64::EPSILON {
            (bin.lo - x_min) / x_range
        } else {
            0.0
        };
        let x_frac_hi = if x_range.abs() > f64::EPSILON {
            (bin.hi - x_min) / x_range
        } else {
            1.0
        };

        let bar_x = margins.left + x_frac_lo * pw;
        let bar_w = (x_frac_hi - x_frac_lo) * pw;

        let y_frac = if y_range.abs() > f64::EPSILON {
            (bin.count as f64 - y_scale.min) / y_range
        } else {
            0.5
        };
        let bar_top = margins.top + ph * (1.0 - y_frac);
        let bar_h = margins.top + ph - bar_top;

        svg.push_str(&format!(
            r##"<rect class="histogram-bar" x="{bx:.1}" y="{by:.1}" width="{bw:.1}" height="{bh:.1}" fill="{color}" stroke="#ffffff" stroke-width="0.5"/>"##,
            bx = bar_x,
            by = bar_top,
            bw = bar_w,
            bh = bar_h,
        ));
        svg.push('\n');
    }

    // X-axis boundary labels (show bin edges)
    let max_labels = 8;
    let label_step = if bins.len() + 1 > max_labels {
        (bins.len() + 1) / max_labels + 1
    } else {
        1
    };

    for (i, bin) in bins.iter().enumerate() {
        if i % label_step == 0 {
            let x_frac = if x_range.abs() > f64::EPSILON {
                (bin.lo - x_min) / x_range
            } else {
                0.0
            };
            let x = margins.left + x_frac * pw;
            let y = margins.top + ph + 18.0;
            svg.push_str(&svg_text(
                x, y, "middle", 9, "#666666",
                &format_axis_value(bin.lo),
            ));
            svg.push('\n');
        }
    }
    // Always label the last boundary
    {
        let x = margins.left + pw;
        let y = margins.top + ph + 18.0;
        svg.push_str(&svg_text(
            x, y, "middle", 9, "#666666",
            &format_axis_value(x_max),
        ));
        svg.push('\n');
    }

    // X-axis baseline
    svg.push_str(&format!(
        r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="#333333" stroke-width="1"/>"##,
        x1 = margins.left,
        x2 = margins.left + pw,
        y = margins.top + ph,
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
            labels: vec![],
            series: vec![DataSeries {
                name: "Scores".into(),
                values: vec![
                    55.0, 60.0, 62.0, 65.0, 67.0, 70.0, 72.0, 75.0, 78.0, 80.0,
                    82.0, 85.0, 88.0, 90.0, 92.0, 95.0, 97.0, 100.0, 58.0, 63.0,
                ],
                color: None,
            }],
        }
    }

    #[test]
    fn test_sturges_bins() {
        assert_eq!(sturges_bins(0), 1);
        assert_eq!(sturges_bins(1), 1);
        // 10 data points: ceil(1 + 3.322 * log10(10)) = ceil(1 + 3.322) = 5
        assert_eq!(sturges_bins(10), 5);
        // 100 data points: ceil(1 + 3.322 * 2) = ceil(7.644) = 8
        assert_eq!(sturges_bins(100), 8);
        // 1000: ceil(1 + 3.322 * 3) = ceil(10.966) = 11
        assert_eq!(sturges_bins(1000), 11);
    }

    #[test]
    fn test_build_bins_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let bins = build_bins(&values, 5);
        assert_eq!(bins.len(), 5);
        let total: usize = bins.iter().map(|b| b.count).sum();
        assert_eq!(total, 10);
        assert!((bins[0].lo - 1.0).abs() < f64::EPSILON);
        assert!((bins[4].hi - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_build_bins_equal_values() {
        let values = vec![5.0, 5.0, 5.0];
        let bins = build_bins(&values, 3);
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].count, 3);
    }

    #[test]
    fn test_build_bins_empty() {
        let bins = build_bins(&[], 5);
        assert!(bins.is_empty());
    }

    #[test]
    fn test_histogram_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_histogram_contains_bars() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("histogram-bar"));
        // Sturges for 20 data points = ceil(1 + 3.322*log10(20)) = ceil(5.32) = 6
        let bar_count = svg.matches("histogram-bar").count();
        assert_eq!(bar_count, sturges_bins(20));
    }

    #[test]
    fn test_histogram_bars_adjacent() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // Adjacent bars have thin white stroke separators
        assert!(svg.contains(r##"stroke="#ffffff""##));
        assert!(svg.contains(r#"stroke-width="0.5""#));
    }

    #[test]
    fn test_histogram_has_x_axis_labels() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // Should have bin boundary labels (numeric values)
        assert!(svg.contains("<text"));
    }

    #[test]
    fn test_histogram_with_title() {
        let opts = ChartOptions {
            title: Some("Score Distribution".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Score Distribution"));
    }

    #[test]
    fn test_histogram_has_grid_lines() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("<line"));
    }

    #[test]
    fn test_histogram_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_histogram_single_value() {
        let data = ChartData {
            labels: vec![],
            series: vec![DataSeries {
                name: "S".into(),
                values: vec![42.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("histogram-bar"));
    }

    #[test]
    fn test_histogram_custom_color() {
        let data = ChartData {
            labels: vec![],
            series: vec![DataSeries {
                name: "S".into(),
                values: vec![1.0, 2.0, 3.0, 4.0, 5.0],
                color: Some("#ff6600".into()),
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.contains("#ff6600"));
    }

    #[test]
    fn test_histogram_no_grid() {
        let opts = ChartOptions {
            show_grid: false,
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }
}
