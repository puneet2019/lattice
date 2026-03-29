//! Read charts from `.xlsx` files.
//!
//! Opens an xlsx file as a ZIP archive, discovers chart XML entries via OPC
//! relationships (`xl/drawings/_rels/*.rels` -> `xl/charts/chart*.xml`), and
//! parses each chart using the parser in `xlsx_chart_parser`.

use std::io::Read;
use std::path::Path;

pub use crate::xlsx_chart_parser::ImportedChart;
use crate::xlsx_chart_parser::{
    extract_relationship_targets, parse_chart_xml, resolve_relative_path,
};
use crate::{IoError, Result};

/// Read all charts from an `.xlsx` file.
///
/// Returns a (possibly empty) list of `ImportedChart` values. Errors are
/// non-fatal for individual charts -- if a chart XML cannot be parsed it is
/// silently skipped.
///
/// # How it works
///
/// 1. Opens the xlsx file as a ZIP archive.
/// 2. For each worksheet, reads `xl/worksheets/_rels/sheet*.xml.rels` to
///    find drawing relationships.
/// 3. For each drawing, reads `xl/drawings/_rels/drawing*.xml.rels` to find
///    chart relationships.
/// 4. Reads and parses each chart XML entry via `parse_chart_xml`.
pub fn read_xlsx_charts(path: &Path) -> Result<Vec<ImportedChart>> {
    if !path.exists() {
        return Err(IoError::FileNotFound(path.display().to_string()));
    }

    let file = std::fs::File::open(path).map_err(IoError::Io)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| IoError::XlsxRead(format!("zip error: {}", e)))?;

    let mut charts = Vec::new();

    // Collect all entry names up front to avoid borrow issues.
    let entry_names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .collect();

    // Find worksheet relationship files to discover drawings.
    let sheet_rels: Vec<String> = entry_names
        .iter()
        .filter(|n| n.starts_with("xl/worksheets/_rels/") && n.ends_with(".xml.rels"))
        .cloned()
        .collect();

    for sheet_rel_path in &sheet_rels {
        // Derive the sheet name from the rels file name (e.g. "sheet1.xml.rels" -> "sheet1").
        let sheet_file = sheet_rel_path
            .rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".rels");
        // A reasonable default sheet name; the caller can remap if needed.
        let sheet_name = sheet_file.trim_end_matches(".xml").to_string();

        let rels_xml = read_zip_entry(&mut archive, sheet_rel_path)?;

        // Find drawing targets (e.g. "../drawings/drawing1.xml").
        let drawing_targets = extract_relationship_targets(&rels_xml, "drawing");

        for drawing_target in &drawing_targets {
            let drawing_path = resolve_relative_path("xl/worksheets", drawing_target);

            // Read the drawing's own rels file.
            let drawing_rels_path = to_rels_path(&drawing_path);

            let drawing_rels_xml = match read_zip_entry(&mut archive, &drawing_rels_path) {
                Ok(xml) => xml,
                Err(_) => continue, // No rels file -> no charts in this drawing.
            };

            // Find chart targets (e.g. "../charts/chart1.xml").
            let chart_targets = extract_relationship_targets(&drawing_rels_xml, "chart");

            let drawing_dir = drawing_path
                .rsplit_once('/')
                .map(|(d, _)| d)
                .unwrap_or("xl/drawings");

            for chart_target in &chart_targets {
                let chart_path = resolve_relative_path(drawing_dir, chart_target);

                let chart_xml = match read_zip_entry(&mut archive, &chart_path) {
                    Ok(xml) => xml,
                    Err(_) => continue,
                };

                if let Some(imported) = parse_chart_xml(&chart_xml, &sheet_name) {
                    charts.push(imported);
                }
            }
        }
    }

    Ok(charts)
}

/// Convert a zip entry path like `"xl/drawings/drawing1.xml"` to its
/// relationships path `"xl/drawings/_rels/drawing1.xml.rels"`.
fn to_rels_path(entry_path: &str) -> String {
    match entry_path.rsplit_once('/') {
        Some((dir, file)) => format!("{}/_rels/{}.rels", dir, file),
        None => format!("_rels/{}.rels", entry_path),
    }
}

/// Read a zip entry as a UTF-8 string.
fn read_zip_entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Result<String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| IoError::XlsxRead(format!("zip entry '{}': {}", name, e)))?;
    let mut buf = String::new();
    entry
        .read_to_string(&mut buf)
        .map_err(|e| IoError::XlsxRead(format!("reading '{}': {}", name, e)))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_rels_path() {
        assert_eq!(
            to_rels_path("xl/drawings/drawing1.xml"),
            "xl/drawings/_rels/drawing1.xml.rels"
        );
        assert_eq!(
            to_rels_path("xl/worksheets/sheet1.xml"),
            "xl/worksheets/_rels/sheet1.xml.rels"
        );
    }

    #[test]
    fn test_to_rels_path_no_dir() {
        assert_eq!(to_rels_path("file.xml"), "_rels/file.xml.rels");
    }

    #[test]
    fn test_read_xlsx_charts_nonexistent_file() {
        let result = read_xlsx_charts(Path::new("/tmp/nonexistent_file_12345.xlsx"));
        assert!(result.is_err());
    }
}
