//! Candlestick chart SVG rendering.
//!
//! Renders financial OHLC (Open, High, Low, Close) candlestick charts.
//! Expects exactly 4 data series in order: Open, High, Low, Close.
//! Green candle bodies indicate close >= open (bullish), red bodies
//! indicate close < open (bearish). Wicks extend from high to low.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    compute_axis_scale, svg_axis_labels, svg_close, svg_grid_lines, svg_open, svg_text, svg_title,
    xml_escape, Margins,
};

/// Color for bullish candles (close >= open).
const BULLISH_COLOR: &str = "#26a69a";
/// Color for bearish candles (close < open).
const BEARISH_COLOR: &str = "#ef5350";

/// Render candlestick chart data as an SVG string.
///
/// Expects 4 series in order: Open, High, Low, Close. Each series must
/// have the same number of values, one per time period (category label).
/// - Green candle bodies: close >= open (bullish)
/// - Red candle bodies: close < open (bearish)
/// - Thin wicks extend vertically from high to low
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let mut svg = String::with_capacity(4096);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');

    // Require exactly 4 series: Open, High, Low, Close
    if data.series.len() < 4 {
        svg.push_str(&svg_text(
            options.width as f64 / 2.0,
            options.height as f64 / 2.0,
            "middle",
            12,
            "#999999",
            "Candlestick requires 4 series: Open, High, Low, Close",
        ));
        svg.push('\n');
        svg.push_str(svg_close());
        return svg;
    }

    let open = &data.series[0].values;
    let high = &data.series[1].values;
    let low = &data.series[2].values;
    let close = &data.series[3].values;

    let n_candles = open
        .len()
        .min(high.len())
        .min(low.len())
        .min(close.len());

    if n_candles == 0 {
        svg.push_str(&svg_axis_labels(options, &margins));
        svg.push_str(svg_close());
        return svg;
    }

    // Compute y-axis scale from all high/low values
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for i in 0..n_candles {
        if high[i] > y_max { y_max = high[i]; }
        if low[i] < y_min { y_min = low[i]; }
        // Also check open/close in case high/low are misspecified
        if open[i] > y_max { y_max = open[i]; }
        if open[i] < y_min { y_min = open[i]; }
        if close[i] > y_max { y_max = close[i]; }
        if close[i] < y_min { y_min = close[i]; }
    }

    let scale = compute_axis_scale(y_min, y_max);
    let y_range = scale.max - scale.min;

    // Grid lines
    svg.push_str(&svg_grid_lines(&scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    let candle_spacing = pw / n_candles as f64;
    let candle_width = candle_spacing * 0.6;
    let wick_x_offset = candle_spacing / 2.0;

    for i in 0..n_candles {
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];

        let is_bullish = c >= o;
        let color = if is_bullish { BULLISH_COLOR } else { BEARISH_COLOR };

        // Map values to pixel coordinates
        let to_y = |v: f64| -> f64 {
            let frac = if y_range.abs() > f64::EPSILON {
                (v - scale.min) / y_range
            } else {
                0.5
            };
            margins.top + ph * (1.0 - frac)
        };

        let high_y = to_y(h);
        let low_y = to_y(l);
        let open_y = to_y(o);
        let close_y = to_y(c);

        let wick_x = margins.left + i as f64 * candle_spacing + wick_x_offset;
        let body_x = margins.left + i as f64 * candle_spacing
            + (candle_spacing - candle_width) / 2.0;

        // Wick (thin vertical line from high to low)
        svg.push_str(&format!(
            r##"<line class="candlestick-wick" x1="{x:.1}" y1="{y1:.1}" x2="{x:.1}" y2="{y2:.1}" stroke="{color}" stroke-width="1"/>"##,
            x = wick_x,
            y1 = high_y,
            y2 = low_y,
        ));
        svg.push('\n');

        // Body (rectangle from open to close)
        let body_top = open_y.min(close_y);
        let body_height = (open_y - close_y).abs().max(1.0); // min 1px visible

        svg.push_str(&format!(
            r##"<rect class="candlestick-body" x="{bx:.1}" y="{by:.1}" width="{bw:.1}" height="{bh:.1}" fill="{fill}" stroke="{color}" stroke-width="1"/>"##,
            bx = body_x,
            by = body_top,
            bw = candle_width,
            bh = body_height,
            fill = if is_bullish { color } else { color },
        ));
        svg.push('\n');
    }

    // X-axis labels
    let max_labels = 12;
    let label_step = if n_candles > max_labels {
        n_candles / max_labels + 1
    } else {
        1
    };

    for (i, label) in data.labels.iter().take(n_candles).enumerate() {
        if i % label_step == 0 {
            let x = margins.left + (i as f64 + 0.5) * candle_spacing;
            let y = margins.top + ph + 18.0;
            svg.push_str(&svg_text(x, y, "middle", 9, "#666666", &xml_escape(label)));
            svg.push('\n');
        }
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

    fn sample_ohlc_data() -> ChartData {
        ChartData {
            labels: vec![
                "Mon".into(), "Tue".into(), "Wed".into(),
                "Thu".into(), "Fri".into(),
            ],
            series: vec![
                DataSeries {
                    name: "Open".into(),
                    values: vec![100.0, 105.0, 103.0, 108.0, 106.0],
                    color: None,
                },
                DataSeries {
                    name: "High".into(),
                    values: vec![110.0, 108.0, 107.0, 112.0, 110.0],
                    color: None,
                },
                DataSeries {
                    name: "Low".into(),
                    values: vec![95.0, 100.0, 98.0, 104.0, 102.0],
                    color: None,
                },
                DataSeries {
                    name: "Close".into(),
                    values: vec![105.0, 103.0, 108.0, 106.0, 109.0],
                    color: None,
                },
            ],
        }
    }

    #[test]
    fn test_candlestick_valid_svg() {
        let svg = render(&sample_ohlc_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_candlestick_contains_wicks() {
        let svg = render(&sample_ohlc_data(), &ChartOptions::default());
        let wick_count = svg.matches("candlestick-wick").count();
        assert_eq!(wick_count, 5);
    }

    #[test]
    fn test_candlestick_contains_bodies() {
        let svg = render(&sample_ohlc_data(), &ChartOptions::default());
        let body_count = svg.matches("candlestick-body").count();
        assert_eq!(body_count, 5);
    }

    #[test]
    fn test_candlestick_bullish_color() {
        let svg = render(&sample_ohlc_data(), &ChartOptions::default());
        // At least some candles are bullish (green)
        assert!(svg.contains(BULLISH_COLOR));
    }

    #[test]
    fn test_candlestick_bearish_color() {
        let svg = render(&sample_ohlc_data(), &ChartOptions::default());
        // At least some candles are bearish (red): Tue (open 105, close 103)
        assert!(svg.contains(BEARISH_COLOR));
    }

    #[test]
    fn test_candlestick_x_labels() {
        let svg = render(&sample_ohlc_data(), &ChartOptions::default());
        assert!(svg.contains("Mon"));
        assert!(svg.contains("Fri"));
    }

    #[test]
    fn test_candlestick_with_title() {
        let opts = ChartOptions {
            title: Some("Stock Price".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_ohlc_data(), &opts);
        assert!(svg.contains("Stock Price"));
    }

    #[test]
    fn test_candlestick_insufficient_series() {
        let data = ChartData {
            labels: vec!["A".into()],
            series: vec![
                DataSeries { name: "Open".into(), values: vec![10.0], color: None },
                DataSeries { name: "High".into(), values: vec![15.0], color: None },
            ],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("requires 4 series"));
    }

    #[test]
    fn test_candlestick_empty_values() {
        let data = ChartData {
            labels: vec![],
            series: vec![
                DataSeries { name: "O".into(), values: vec![], color: None },
                DataSeries { name: "H".into(), values: vec![], color: None },
                DataSeries { name: "L".into(), values: vec![], color: None },
                DataSeries { name: "C".into(), values: vec![], color: None },
            ],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // No candles rendered
        assert!(!svg.contains("candlestick-wick"));
    }

    #[test]
    fn test_candlestick_single_candle() {
        let data = ChartData {
            labels: vec!["Day1".into()],
            series: vec![
                DataSeries { name: "O".into(), values: vec![100.0], color: None },
                DataSeries { name: "H".into(), values: vec![110.0], color: None },
                DataSeries { name: "L".into(), values: vec![90.0], color: None },
                DataSeries { name: "C".into(), values: vec![105.0], color: None },
            ],
        };
        let svg = render(&data, &ChartOptions::default());
        assert_eq!(svg.matches("candlestick-wick").count(), 1);
        assert_eq!(svg.matches("candlestick-body").count(), 1);
        assert!(svg.contains(BULLISH_COLOR)); // close > open
    }

    #[test]
    fn test_candlestick_doji() {
        // Doji: open == close
        let data = ChartData {
            labels: vec!["Day1".into()],
            series: vec![
                DataSeries { name: "O".into(), values: vec![100.0], color: None },
                DataSeries { name: "H".into(), values: vec![110.0], color: None },
                DataSeries { name: "L".into(), values: vec![90.0], color: None },
                DataSeries { name: "C".into(), values: vec![100.0], color: None },
            ],
        };
        let svg = render(&data, &ChartOptions::default());
        // close >= open => bullish color
        assert!(svg.contains(BULLISH_COLOR));
        assert_eq!(svg.matches("candlestick-body").count(), 1);
    }

    #[test]
    fn test_candlestick_has_grid() {
        let svg = render(&sample_ohlc_data(), &ChartOptions::default());
        assert!(svg.contains("<line"));
    }
}
