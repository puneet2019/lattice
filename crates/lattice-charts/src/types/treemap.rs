//! Treemap chart SVG rendering.
//!
//! Implements the squarified treemap algorithm to lay out data as
//! nested rectangles whose area is proportional to each value.
//! Labels are placed inside each rectangle.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{series_color, svg_close, svg_open, svg_text, svg_title, xml_escape, Margins};

/// Render treemap chart data as an SVG string.
///
/// Each data point becomes a coloured rectangle whose area is
/// proportional to its value. Labels are placed at the centre of each
/// rectangle. Negative values are treated as zero.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins {
        left: 10.0,
        top: if options.title.is_some() { 40.0 } else { 10.0 },
        right: 10.0,
        bottom: 10.0,
    };
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');

    // Collect (label, value, color_index) — use first series values
    let values: Vec<(String, f64, usize)> = if let Some(series) = data.series.first() {
        data.labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let v = series.values.get(i).copied().unwrap_or(0.0).max(0.0);
                (label.clone(), v, i)
            })
            .collect()
    } else {
        Vec::new()
    };

    if values.is_empty() || values.iter().all(|(_, v, _)| *v <= 0.0) {
        svg.push_str(svg_close());
        return svg;
    }

    // Sort descending by value for squarified algorithm
    let mut sorted: Vec<(String, f64, usize)> = values;
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Remove zero-value items
    sorted.retain(|(_, v, _)| *v > 0.0);

    let rects = squarify(&sorted, margins.left, margins.top, pw, ph);

    for rect in &rects {
        let color = data
            .series
            .first()
            .and_then(|s| s.color.as_deref())
            .unwrap_or_else(|| series_color(rect.color_idx));

        // Rectangle
        svg.push_str(&format!(
            r##"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" fill="{color}" stroke="#ffffff" stroke-width="2" rx="2"/>"##,
            x = rect.x,
            y = rect.y,
            w = rect.w,
            h = rect.h,
        ));
        svg.push('\n');

        // Label (only if rectangle is large enough to fit text)
        if rect.w > 20.0 && rect.h > 14.0 {
            let cx = rect.x + rect.w / 2.0;
            let cy = rect.y + rect.h / 2.0 + 4.0;
            let font_size = font_size_for_rect(rect.w, rect.h);
            svg.push_str(&svg_text(
                cx,
                cy,
                "middle",
                font_size,
                "#ffffff",
                &xml_escape(&rect.label),
            ));
            svg.push('\n');
        }
    }

    svg.push_str(svg_close());
    svg
}

/// A laid-out rectangle in the treemap.
#[derive(Debug, Clone)]
struct TreemapRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    label: String,
    color_idx: usize,
}

/// Choose a font size that fits within the rectangle.
fn font_size_for_rect(w: f64, h: f64) -> u32 {
    let size = (w.min(h) / 3.0).min(14.0).max(8.0);
    size as u32
}

/// Squarified treemap layout.
///
/// Lays out items into a rectangle of the given dimensions, producing
/// rectangles with aspect ratios as close to 1 as possible.
fn squarify(items: &[(String, f64, usize)], x: f64, y: f64, w: f64, h: f64) -> Vec<TreemapRect> {
    if items.is_empty() || w <= 0.0 || h <= 0.0 {
        return Vec::new();
    }

    let total: f64 = items.iter().map(|(_, v, _)| *v).sum();
    if total <= 0.0 {
        return Vec::new();
    }

    // Normalise values to areas within the bounding box
    let area = w * h;
    let areas: Vec<f64> = items.iter().map(|(_, v, _)| v / total * area).collect();

    let mut result = Vec::with_capacity(items.len());
    layout_row(items, &areas, x, y, w, h, &mut result);
    result
}

/// Recursively lay out items using the squarified algorithm.
fn layout_row(
    items: &[(String, f64, usize)],
    areas: &[f64],
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    result: &mut Vec<TreemapRect>,
) {
    if items.is_empty() {
        return;
    }
    if items.len() == 1 {
        result.push(TreemapRect {
            x,
            y,
            w,
            h,
            label: items[0].0.clone(),
            color_idx: items[0].2,
        });
        return;
    }

    // Determine whether to lay out horizontally or vertically
    let is_wide = w >= h;

    // Find the best split: add items to the current row until the
    // worst aspect ratio starts increasing
    let mut best_split = 1;
    let mut best_worst_ratio = f64::INFINITY;

    for split in 1..=items.len() {
        let row_area: f64 = areas[..split].iter().sum();
        let worst = worst_aspect_ratio(&areas[..split], row_area, w, h, is_wide);
        if worst <= best_worst_ratio {
            best_worst_ratio = worst;
            best_split = split;
        } else {
            // Aspect ratios are getting worse — stop
            break;
        }
    }

    let row_area: f64 = areas[..best_split].iter().sum();

    // Lay out the row
    if is_wide {
        let row_w = if h > 0.0 { row_area / h } else { 0.0 };
        let mut cy = y;
        for i in 0..best_split {
            let rect_h = if row_w > 0.0 { areas[i] / row_w } else { 0.0 };
            result.push(TreemapRect {
                x,
                y: cy,
                w: row_w,
                h: rect_h,
                label: items[i].0.clone(),
                color_idx: items[i].2,
            });
            cy += rect_h;
        }
        // Recurse with remaining items
        layout_row(
            &items[best_split..],
            &areas[best_split..],
            x + row_w,
            y,
            w - row_w,
            h,
            result,
        );
    } else {
        let row_h = if w > 0.0 { row_area / w } else { 0.0 };
        let mut cx = x;
        for i in 0..best_split {
            let rect_w = if row_h > 0.0 { areas[i] / row_h } else { 0.0 };
            result.push(TreemapRect {
                x: cx,
                y,
                w: rect_w,
                h: row_h,
                label: items[i].0.clone(),
                color_idx: items[i].2,
            });
            cx += rect_w;
        }
        // Recurse with remaining items
        layout_row(
            &items[best_split..],
            &areas[best_split..],
            x,
            y + row_h,
            w,
            h - row_h,
            result,
        );
    }
}

/// Compute the worst (largest) aspect ratio for a set of areas laid out
/// in a row within the given bounding box.
fn worst_aspect_ratio(areas: &[f64], row_area: f64, w: f64, h: f64, is_wide: bool) -> f64 {
    if areas.is_empty() || row_area <= 0.0 {
        return f64::INFINITY;
    }

    // The fixed side length of the row
    let side = if is_wide {
        if h > 0.0 {
            row_area / h
        } else {
            return f64::INFINITY;
        }
    } else if w > 0.0 {
        row_area / w
    } else {
        return f64::INFINITY;
    };

    let mut worst = 0.0_f64;
    for &a in areas {
        let other = if side > 0.0 { a / side } else { 0.0 };
        let ratio = if side > other && other > 0.0 {
            side / other
        } else if other > 0.0 {
            other / side
        } else {
            f64::INFINITY
        };
        worst = worst.max(ratio);
    }
    worst
}

/// Render treemap data to SVG (public convenience wrapper).
///
/// This is the top-level entry point used by the chart rendering dispatcher.
pub fn render_treemap(data: &ChartData, options: &ChartOptions) -> String {
    render(data, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{ChartData, ChartOptions, DataSeries};

    fn sample_data() -> ChartData {
        ChartData {
            labels: vec![
                "Alpha".into(),
                "Beta".into(),
                "Gamma".into(),
                "Delta".into(),
            ],
            series: vec![DataSeries {
                name: "Sizes".into(),
                values: vec![40.0, 30.0, 20.0, 10.0],
                color: None,
            }],
        }
    }

    #[test]
    fn test_treemap_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_treemap_contains_rects() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // Should have 4 rects for 4 data points (plus background rect)
        let rect_count = svg.matches("<rect").count();
        assert!(
            rect_count >= 5,
            "expected at least 5 rects (1 bg + 4 data), got {rect_count}"
        );
    }

    #[test]
    fn test_treemap_contains_labels() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("Alpha"));
        assert!(svg.contains("Beta"));
        assert!(svg.contains("Gamma"));
    }

    #[test]
    fn test_treemap_with_title() {
        let opts = ChartOptions {
            title: Some("Market Share".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Market Share"));
    }

    #[test]
    fn test_treemap_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_treemap_all_zeros() {
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
        // Should not contain data rects (only bg rect)
        let rect_count = svg.matches("<rect").count();
        assert_eq!(
            rect_count, 1,
            "only background rect expected for all-zero data"
        );
    }

    #[test]
    fn test_treemap_single_item() {
        let data = ChartData {
            labels: vec!["Only".into()],
            series: vec![DataSeries {
                name: "S".into(),
                values: vec![100.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.contains("Only"));
        // 1 bg rect + 1 data rect
        let rect_count = svg.matches("<rect").count();
        assert_eq!(rect_count, 2);
    }

    #[test]
    fn test_treemap_negative_values_treated_as_zero() {
        let data = ChartData {
            labels: vec!["Pos".into(), "Neg".into()],
            series: vec![DataSeries {
                name: "S".into(),
                values: vec![50.0, -10.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        // Only 1 data rect for the positive value
        assert!(svg.contains("Pos"));
        // bg rect + 1 data rect
        let rect_count = svg.matches("<rect").count();
        assert_eq!(rect_count, 2);
    }

    #[test]
    fn test_treemap_white_stroke_separators() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains(r##"stroke="#ffffff""##));
    }

    #[test]
    fn test_render_treemap_alias() {
        let svg = render_treemap(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_squarify_produces_correct_count() {
        let items = vec![
            ("A".into(), 6.0, 0),
            ("B".into(), 6.0, 1),
            ("C".into(), 4.0, 2),
            ("D".into(), 3.0, 3),
            ("E".into(), 2.0, 4),
            ("F".into(), 1.0, 5),
        ];
        let rects = squarify(&items, 0.0, 0.0, 600.0, 400.0);
        assert_eq!(rects.len(), 6);
    }

    #[test]
    fn test_squarify_total_area_matches() {
        let items = vec![
            ("A".into(), 40.0, 0),
            ("B".into(), 30.0, 1),
            ("C".into(), 20.0, 2),
            ("D".into(), 10.0, 3),
        ];
        let rects = squarify(&items, 0.0, 0.0, 100.0, 100.0);
        let total_area: f64 = rects.iter().map(|r| r.w * r.h).sum();
        assert!(
            (total_area - 10000.0).abs() < 1.0,
            "total area {total_area} should be close to 10000"
        );
    }

    #[test]
    fn test_squarify_no_overlapping() {
        let items = vec![
            ("A".into(), 50.0, 0),
            ("B".into(), 30.0, 1),
            ("C".into(), 20.0, 2),
        ];
        let rects = squarify(&items, 0.0, 0.0, 200.0, 100.0);
        // All rects should be within bounds
        for r in &rects {
            assert!(r.x >= -0.1, "x out of bounds: {}", r.x);
            assert!(r.y >= -0.1, "y out of bounds: {}", r.y);
            assert!(
                r.x + r.w <= 200.1,
                "right edge out of bounds: {}",
                r.x + r.w
            );
            assert!(
                r.y + r.h <= 100.1,
                "bottom edge out of bounds: {}",
                r.y + r.h
            );
        }
    }
}
