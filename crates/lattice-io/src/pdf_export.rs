//! Print-ready HTML export for Lattice workbooks.
//!
//! Generates an HTML page containing a `<table>` with cell values,
//! formatting (bold, italic, colors), and column widths. Includes
//! `@media print` CSS rules for proper page breaks. The HTML can be
//! opened in a browser and printed to PDF.

use lattice_core::{CellValue, HAlign, VAlign, Workbook};
use serde::{Deserialize, Serialize};

use crate::{IoError, Result};

/// Print settings that control page layout, margins, and display options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintSettings {
    /// Paper size: "letter", "a4", "legal", "tabloid". Default: "letter".
    #[serde(default = "default_paper_size")]
    pub paper_size: String,
    /// Orientation: "portrait" or "landscape". Default: "portrait".
    #[serde(default = "default_orientation")]
    pub orientation: String,
    /// Whether to show gridlines in the printout. Default: true.
    #[serde(default = "default_true")]
    pub show_gridlines: bool,
    /// Whether to show row/column headers (A, B, C / 1, 2, 3). Default: false.
    #[serde(default)]
    pub show_headers: bool,
    /// Scale factor (1.0 = 100%). Default: 1.0.
    #[serde(default = "default_scale")]
    pub scale: f64,
    /// Margin preset: "normal", "narrow", "wide", or "custom". Default: "normal".
    #[serde(default = "default_margins")]
    pub margins: String,
    /// Custom margins in cm (top, bottom, left, right). Used when margins = "custom".
    #[serde(default)]
    pub custom_margins: Option<[f64; 4]>,
}

fn default_paper_size() -> String { "letter".to_string() }
fn default_orientation() -> String { "portrait".to_string() }
fn default_true() -> bool { true }
fn default_scale() -> f64 { 1.0 }
fn default_margins() -> String { "normal".to_string() }

impl Default for PrintSettings {
    fn default() -> Self {
        Self {
            paper_size: default_paper_size(),
            orientation: default_orientation(),
            show_gridlines: true,
            show_headers: false,
            scale: 1.0,
            margins: default_margins(),
            custom_margins: None,
        }
    }
}

/// Export a sheet as a self-contained, print-ready HTML page.
///
/// The generated HTML includes:
/// - A `<table>` with all cell values and inline formatting.
/// - `@media print` CSS for proper page breaks and margins.
/// - Column widths based on the sheet's `col_widths` map.
/// - Bold, italic, font color, background color, and alignment.
///
/// If `sheet_name` is `None`, the active sheet is exported.
/// If `settings` is `None`, default print settings are used.
///
/// # Example
///
/// ```no_run
/// use lattice_core::Workbook;
/// use lattice_io::pdf_export::export_print_html;
///
/// let wb = Workbook::new();
/// let html = export_print_html(&wb, None, None).unwrap();
/// std::fs::write("output.html", &html).unwrap();
/// // Open output.html in a browser and print to PDF.
/// ```
pub fn export_print_html(
    workbook: &Workbook,
    sheet_name: Option<&str>,
    settings: Option<&PrintSettings>,
) -> Result<String> {
    let defaults = PrintSettings::default();
    let s = settings.unwrap_or(&defaults);

    let name = sheet_name.unwrap_or(&workbook.active_sheet);
    let sheet = workbook.get_sheet(name).map_err(IoError::Core)?;

    let (max_row, max_col) = sheet.used_range();

    let mut html = String::with_capacity(4096);

    // Build dynamic CSS based on print settings.
    let dynamic_css = build_print_css(s);

    // HTML head with print CSS.
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<title>");
    html.push_str(&escape_html(name));
    html.push_str("</title>\n");
    html.push_str("<style>\n");
    html.push_str(&dynamic_css);
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");

    // Sheet title.
    html.push_str("<h1>");
    html.push_str(&escape_html(name));
    html.push_str("</h1>\n");

    // If the sheet is empty, show a message.
    if sheet.cells().is_empty() {
        html.push_str("<p>This sheet is empty.</p>\n");
        html.push_str("</body>\n</html>\n");
        return Ok(html);
    }

    // Table.
    html.push_str("<table>\n");

    // Optional column headers row (A, B, C...).
    if s.show_headers {
        html.push_str("<thead><tr><th class=\"row-header\"></th>");
        for col in 0..=max_col {
            html.push_str("<th class=\"col-header\">");
            html.push_str(&col_to_letter(col));
            html.push_str("</th>\n");
        }
        html.push_str("</tr></thead>\n");
    }

    // Colgroup for column widths.
    html.push_str("<colgroup>\n");
    if s.show_headers {
        html.push_str("<col style=\"width:40px\">\n"); // row number column
    }
    for col in 0..=max_col {
        let width = sheet.col_widths.get(&col).copied().unwrap_or(80.0);
        // Excel column width units are roughly 7px per unit.
        let px = (width * 7.0).round() as u32;
        html.push_str(&format!("<col style=\"width:{}px\">\n", px));
    }
    html.push_str("</colgroup>\n");

    // Table body.
    html.push_str("<tbody>\n");
    for row in 0..=max_row {
        html.push_str("<tr");
        // Row height.
        if let Some(&height) = sheet.row_heights.get(&row) {
            html.push_str(&format!(" style=\"height:{}px\"", height.round() as u32));
        }
        html.push_str(">\n");

        // Row number header.
        if s.show_headers {
            html.push_str(&format!("<td class=\"row-header\">{}</td>\n", row + 1));
        }

        for col in 0..=max_col {
            let (content, style) = match sheet.get_cell(row, col) {
                Some(cell) => {
                    let text = cell_value_to_html(&cell.value);
                    let css = cell_format_to_css(&cell.format);
                    (text, css)
                }
                None => (String::new(), String::new()),
            };

            if style.is_empty() {
                html.push_str("<td>");
            } else {
                html.push_str("<td style=\"");
                html.push_str(&style);
                html.push_str("\">");
            }
            html.push_str(&content);
            html.push_str("</td>\n");
        }

        html.push_str("</tr>\n");
    }
    html.push_str("</tbody>\n");
    html.push_str("</table>\n");

    html.push_str("</body>\n</html>\n");

    Ok(html)
}

/// Convert a 0-based column index to a letter (A, B, ..., Z, AA, AB, ...).
fn col_to_letter(col: u32) -> String {
    let mut result = String::new();
    let mut c = col as usize;
    loop {
        result.insert(0, (b'A' + (c % 26) as u8) as char);
        if c < 26 {
            break;
        }
        c = c / 26 - 1;
    }
    result
}

/// Build the CSS string based on `PrintSettings`.
fn build_print_css(s: &PrintSettings) -> String {
    let border_rule = if s.show_gridlines {
        "border: 1px solid #ccc;"
    } else {
        "border: none;"
    };

    let scale_transform = if (s.scale - 1.0).abs() > 0.001 {
        format!(
            "body {{ transform: scale({:.3}); transform-origin: top left; }}\n",
            s.scale
        )
    } else {
        String::new()
    };

    // Resolve page size CSS value.
    let page_size = match s.paper_size.as_str() {
        "letter" => "8.5in 11in",
        "legal" => "8.5in 14in",
        "tabloid" => "11in 17in",
        "a4" => "210mm 297mm",
        _ => "8.5in 11in",
    };

    let page_orientation = match s.orientation.as_str() {
        "landscape" => " landscape",
        _ => " portrait",
    };

    // Resolve margins.
    let margin_css = match s.margins.as_str() {
        "narrow" => "margin: 0.5cm;".to_string(),
        "wide" => "margin: 2.5cm;".to_string(),
        "custom" => {
            if let Some([top, bottom, left, right]) = s.custom_margins {
                format!(
                    "margin: {:.2}cm {:.2}cm {:.2}cm {:.2}cm;",
                    top, right, bottom, left
                )
            } else {
                "margin: 1.5cm;".to_string()
            }
        }
        _ => "margin: 1.5cm;".to_string(), // "normal"
    };

    // Header style for row/col headers.
    let header_css = if s.show_headers {
        ".row-header, .col-header { background: #f0f0f0; font-weight: 600; text-align: center; font-size: 9pt; color: #666; border: 1px solid #ccc; padding: 2px 4px; }\n"
    } else {
        ""
    };

    format!(
        r#"* {{
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}}
body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
                 "Helvetica Neue", Arial, sans-serif;
    font-size: 11pt;
    color: #000;
    padding: 20px;
}}
h1 {{
    font-size: 14pt;
    margin-bottom: 12px;
    font-weight: 600;
}}
table {{
    border-collapse: collapse;
    width: 100%;
    table-layout: fixed;
}}
td {{
    {border_rule}
    padding: 4px 6px;
    vertical-align: bottom;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11pt;
}}
{header_css}{scale_transform}@media print {{
    body {{
        padding: 0;
    }}
    h1 {{
        page-break-after: avoid;
    }}
    table {{
        page-break-inside: auto;
    }}
    tr {{
        page-break-inside: avoid;
        page-break-after: auto;
    }}
    td {{
        border-color: #999;
    }}
    @page {{
        {margin_css}
        size: {page_size}{page_orientation};
    }}
}}
"#
    )
}

/// Convert a `CellValue` to an HTML-safe string.
fn cell_value_to_html(value: &CellValue) -> String {
    match value {
        CellValue::Empty => String::new(),
        CellValue::Text(s) => escape_html(s),
        CellValue::Number(n) => {
            if *n == n.trunc() && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        CellValue::Boolean(b) | CellValue::Checkbox(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        CellValue::Error(e) => escape_html(&e.to_string()),
        CellValue::Date(s) => escape_html(s),
        CellValue::Array(_) => escape_html("{array}"),
    }
}

/// Convert a `CellFormat` to inline CSS properties.
fn cell_format_to_css(cf: &lattice_core::CellFormat) -> String {
    let mut parts = Vec::new();

    if cf.bold {
        parts.push("font-weight:bold".to_string());
    }
    if cf.italic {
        parts.push("font-style:italic".to_string());
    }
    if (cf.font_size - 11.0).abs() > 0.01 {
        parts.push(format!("font-size:{}pt", cf.font_size));
    }
    if let Some(ref fc) = cf.font_color {
        parts.push(format!("color:{}", fc));
    }
    if let Some(ref bg) = cf.bg_color {
        parts.push(format!("background-color:{}", bg));
    }

    match cf.h_align {
        HAlign::Left => {} // default
        HAlign::Center => parts.push("text-align:center".to_string()),
        HAlign::Right => parts.push("text-align:right".to_string()),
    }

    match cf.v_align {
        VAlign::Bottom => {} // default in our CSS
        VAlign::Top => parts.push("vertical-align:top".to_string()),
        VAlign::Middle => parts.push("vertical-align:middle".to_string()),
    }

    parts.join(";")
}

/// Escape HTML special characters.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::{CellFormat, CellValue, HAlign, VAlign, Workbook};

    #[test]
    fn test_export_empty_sheet() {
        let wb = Workbook::new();
        let html = export_print_html(&wb, None, None).unwrap();
        assert!(html.contains("This sheet is empty."));
        assert!(html.contains("<title>Sheet1</title>"));
    }

    #[test]
    fn test_export_basic_data() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Hello".into()))
            .unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(42.0))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Boolean(true))
            .unwrap();

        let html = export_print_html(&wb, None, None).unwrap();
        assert!(html.contains("<table>"));
        assert!(html.contains("Hello"));
        assert!(html.contains("42"));
        assert!(html.contains("TRUE"));
    }

    #[test]
    fn test_export_with_formatting() {
        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        let cell = lattice_core::Cell {
            value: CellValue::Text("Bold Red".into()),
            format: CellFormat {
                bold: true,
                italic: true,
                font_size: 14.0,
                font_color: Some("#FF0000".to_string()),
                bg_color: Some("#FFFF00".to_string()),
                h_align: HAlign::Center,
                v_align: VAlign::Middle,
                number_format: None,
                ..CellFormat::default()
            },
            ..Default::default()
        };
        sheet.set_cell(0, 0, cell);

        let html = export_print_html(&wb, None, None).unwrap();
        assert!(html.contains("font-weight:bold"));
        assert!(html.contains("font-style:italic"));
        assert!(html.contains("font-size:14pt"));
        assert!(html.contains("color:#FF0000"));
        assert!(html.contains("background-color:#FFFF00"));
        assert!(html.contains("text-align:center"));
        assert!(html.contains("vertical-align:middle"));
    }

    #[test]
    fn test_export_html_escaping() {
        let mut wb = Workbook::new();
        wb.set_cell(
            "Sheet1",
            0,
            0,
            CellValue::Text("<script>alert('xss')</script>".into()),
        )
        .unwrap();

        let html = export_print_html(&wb, None, None).unwrap();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_export_specific_sheet() {
        let mut wb = Workbook::new();
        wb.add_sheet("Data").unwrap();
        wb.set_cell("Data", 0, 0, CellValue::Text("data value".into()))
            .unwrap();

        let html = export_print_html(&wb, Some("Data"), None).unwrap();
        assert!(html.contains("<title>Data</title>"));
        assert!(html.contains("data value"));
    }

    #[test]
    fn test_export_sheet_not_found() {
        let wb = Workbook::new();
        let result = export_print_html(&wb, Some("NonExistent"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_export_has_print_css() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();

        let html = export_print_html(&wb, None, None).unwrap();
        assert!(html.contains("@media print"));
        assert!(html.contains("@page"));
        assert!(html.contains("page-break-inside"));
    }

    #[test]
    fn test_export_column_widths() {
        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        sheet.col_widths.insert(0, 20.0);
        sheet.set_value(0, 0, CellValue::Text("wide".into()));

        let html = export_print_html(&wb, None, None).unwrap();
        assert!(html.contains("width:140px")); // 20 * 7 = 140
    }

    #[test]
    fn test_export_row_heights() {
        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        sheet.row_heights.insert(0, 30.0);
        sheet.set_value(0, 0, CellValue::Text("tall".into()));

        let html = export_print_html(&wb, None, None).unwrap();
        assert!(html.contains("height:30px"));
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<b>hi</b>"), "&lt;b&gt;hi&lt;/b&gt;");
        assert_eq!(escape_html("a&b"), "a&amp;b");
        assert_eq!(escape_html("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_cell_value_to_html_variants() {
        assert_eq!(cell_value_to_html(&CellValue::Empty), "");
        assert_eq!(cell_value_to_html(&CellValue::Number(3.14)), "3.14");
        assert_eq!(cell_value_to_html(&CellValue::Number(100.0)), "100");
        assert_eq!(cell_value_to_html(&CellValue::Boolean(false)), "FALSE");
        assert_eq!(
            cell_value_to_html(&CellValue::Date("2024-01-01".into())),
            "2024-01-01"
        );
    }

    #[test]
    fn test_export_with_landscape_a4() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        let settings = PrintSettings {
            paper_size: "a4".to_string(),
            orientation: "landscape".to_string(),
            ..PrintSettings::default()
        };
        let html = export_print_html(&wb, None, Some(&settings)).unwrap();
        assert!(html.contains("210mm 297mm"));
        assert!(html.contains("landscape"));
    }

    #[test]
    fn test_export_no_gridlines() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        let settings = PrintSettings {
            show_gridlines: false,
            ..PrintSettings::default()
        };
        let html = export_print_html(&wb, None, Some(&settings)).unwrap();
        assert!(html.contains("border: none;"));
    }

    #[test]
    fn test_export_with_headers() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("val".into())).unwrap();
        let settings = PrintSettings {
            show_headers: true,
            ..PrintSettings::default()
        };
        let html = export_print_html(&wb, None, Some(&settings)).unwrap();
        assert!(html.contains("col-header"));
        assert!(html.contains("row-header"));
        assert!(html.contains(">A<"));
    }

    #[test]
    fn test_export_custom_margins() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        let settings = PrintSettings {
            margins: "custom".to_string(),
            custom_margins: Some([1.0, 2.0, 1.5, 2.5]),
            ..PrintSettings::default()
        };
        let html = export_print_html(&wb, None, Some(&settings)).unwrap();
        assert!(html.contains("1.00cm"));
    }

    #[test]
    fn test_col_to_letter_fn() {
        assert_eq!(col_to_letter(0), "A");
        assert_eq!(col_to_letter(25), "Z");
        assert_eq!(col_to_letter(26), "AA");
        assert_eq!(col_to_letter(27), "AB");
    }
}
