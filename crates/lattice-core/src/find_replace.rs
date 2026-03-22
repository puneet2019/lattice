//! Find & Replace operations across workbook cells.
//!
//! Supports case-sensitive/insensitive search, whole-cell or substring
//! matching, regex patterns, and searching formula text or display values.
//! Searches can be scoped to a single sheet or span the entire workbook.

use regex::Regex;

use crate::cell::CellValue;
use crate::error::{LatticeError, Result};
use crate::workbook::Workbook;

/// Options that control how a find (or find-and-replace) operation behaves.
#[derive(Debug, Clone)]
pub struct FindOptions {
    /// The search query (plain text or regex pattern).
    pub query: String,
    /// If `true`, matching is case-sensitive.
    pub case_sensitive: bool,
    /// If `true`, the query must match the entire cell content (not a substring).
    pub whole_cell: bool,
    /// If `true`, the query is interpreted as a regular expression.
    pub use_regex: bool,
    /// If `true`, search formula text instead of display values.
    pub search_formulas: bool,
    /// Restrict the search to a specific sheet. `None` searches all sheets.
    pub sheet_name: Option<String>,
}

/// A single match found during a find operation.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchLocation {
    /// The sheet where the match was found.
    pub sheet: String,
    /// 0-based row of the matching cell.
    pub row: u32,
    /// 0-based column of the matching cell.
    pub col: u32,
    /// The text that was matched.
    pub matched_text: String,
}

/// Summary of a replace-all operation.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceResult {
    /// Total number of matches found.
    pub matches_found: usize,
    /// Number of replacements actually made.
    pub replacements_made: usize,
}

/// Convert a `CellValue` to its display string representation.
fn cell_value_to_string(val: &CellValue) -> String {
    match val {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => n.to_string(),
        CellValue::Boolean(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        CellValue::Empty => String::new(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
    }
}

/// Build a `Regex` from the find options.
///
/// When `use_regex` is false the query is escaped so it is treated as a
/// literal string. When `whole_cell` is true anchors are added so the
/// pattern must match the entire cell content.
fn build_regex(options: &FindOptions) -> Result<Regex> {
    let mut pattern = if options.use_regex {
        options.query.clone()
    } else {
        regex::escape(&options.query)
    };

    if options.whole_cell {
        pattern = format!("^{pattern}$");
    }

    let regex = if options.case_sensitive {
        Regex::new(&pattern)
    } else {
        Regex::new(&format!("(?i){pattern}"))
    };

    regex.map_err(|e| LatticeError::Internal(format!("invalid regex: {e}")))
}

/// Find all cells in the workbook that match the given options.
///
/// Results are returned in sheet-tab order, then row-major order within
/// each sheet.
pub fn find(workbook: &Workbook, options: &FindOptions) -> Result<Vec<MatchLocation>> {
    if options.query.is_empty() {
        return Ok(Vec::new());
    }

    let re = build_regex(options)?;
    let mut matches = Vec::new();

    let sheet_names: Vec<String> = match &options.sheet_name {
        Some(name) => {
            // Validate the sheet exists.
            workbook.get_sheet(name)?;
            vec![name.clone()]
        }
        None => workbook.sheet_names(),
    };

    for sheet_name in &sheet_names {
        let sheet = workbook.get_sheet(sheet_name)?;

        // Collect and sort cell positions for deterministic order.
        let mut positions: Vec<(u32, u32)> = sheet.cells().keys().copied().collect();
        positions.sort();

        for (row, col) in positions {
            let cell = sheet.get_cell(row, col).unwrap();

            let text = if options.search_formulas {
                cell.formula.clone().unwrap_or_else(|| cell_value_to_string(&cell.value))
            } else {
                cell_value_to_string(&cell.value)
            };

            if text.is_empty() {
                continue;
            }

            if let Some(m) = re.find(&text) {
                matches.push(MatchLocation {
                    sheet: sheet_name.clone(),
                    row,
                    col,
                    matched_text: m.as_str().to_string(),
                });
            }
        }
    }

    Ok(matches)
}

/// Replace all matching occurrences in the workbook.
///
/// Only cells with `CellValue::Text` values (or formula text when
/// `search_formulas` is true) are modified. Number, boolean, date, and
/// error cells are reported as matches but not replaced to avoid
/// corrupting typed data.
pub fn replace_all(
    workbook: &mut Workbook,
    options: &FindOptions,
    replacement: &str,
) -> Result<ReplaceResult> {
    if options.query.is_empty() {
        return Ok(ReplaceResult {
            matches_found: 0,
            replacements_made: 0,
        });
    }

    let re = build_regex(options)?;

    let sheet_names: Vec<String> = match &options.sheet_name {
        Some(name) => {
            workbook.get_sheet(name)?;
            vec![name.clone()]
        }
        None => workbook.sheet_names(),
    };

    let mut matches_found: usize = 0;
    let mut replacements_made: usize = 0;

    for sheet_name in &sheet_names {
        let sheet = workbook.get_sheet(sheet_name)?;

        // Collect matching positions first (borrow checker: read then write).
        let mut positions: Vec<(u32, u32)> = sheet.cells().keys().copied().collect();
        positions.sort();

        let mut to_replace: Vec<(u32, u32)> = Vec::new();

        for &(row, col) in &positions {
            let cell = sheet.get_cell(row, col).unwrap();

            let text = if options.search_formulas {
                cell.formula.clone().unwrap_or_else(|| cell_value_to_string(&cell.value))
            } else {
                cell_value_to_string(&cell.value)
            };

            if text.is_empty() {
                continue;
            }

            if re.is_match(&text) {
                matches_found += 1;
                to_replace.push((row, col));
            }
        }

        // Now perform replacements.
        let sheet = workbook.get_sheet_mut(sheet_name)?;
        for (row, col) in to_replace {
            let cell = sheet.get_cell_mut(row, col).unwrap();

            if options.search_formulas {
                if let Some(ref formula) = cell.formula {
                    let new_formula = re.replace_all(formula, replacement).to_string();
                    cell.formula = Some(new_formula);
                    replacements_made += 1;
                } else if let CellValue::Text(ref text) = cell.value {
                    let new_text = re.replace_all(text, replacement).to_string();
                    cell.value = CellValue::Text(new_text);
                    replacements_made += 1;
                }
                // Non-text, non-formula cells: matched but not replaced.
            } else {
                match &cell.value {
                    CellValue::Text(text) => {
                        let new_text = re.replace_all(text, replacement).to_string();
                        cell.value = CellValue::Text(new_text);
                        replacements_made += 1;
                    }
                    _ => {
                        // Non-text cells: found but not replaced.
                    }
                }
            }
        }
    }

    Ok(ReplaceResult {
        matches_found,
        replacements_made,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a workbook with test data across different value types.
    fn test_workbook() -> Workbook {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Hello World".into())).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(42.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("hello there".into())).unwrap();
        wb.set_cell("Sheet1", 1, 1, CellValue::Text("Goodbye".into())).unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Boolean(true)).unwrap();
        wb
    }

    /// Helper: build FindOptions with common defaults.
    fn opts(query: &str) -> FindOptions {
        FindOptions {
            query: query.into(),
            case_sensitive: false,
            whole_cell: false,
            use_regex: false,
            search_formulas: false,
            sheet_name: None,
        }
    }

    #[test]
    fn test_find_case_insensitive() {
        let results = find(&test_workbook(), &opts("hello")).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!((results[0].row, results[0].col), (0, 0));
        assert_eq!((results[1].row, results[1].col), (1, 0));
    }

    #[test]
    fn test_find_case_sensitive() {
        let mut o = opts("Hello");
        o.case_sensitive = true;
        assert_eq!(find(&test_workbook(), &o).unwrap().len(), 1);
    }

    #[test]
    fn test_find_whole_cell_and_regex() {
        let mut o = opts("hello there");
        o.whole_cell = true;
        assert_eq!(find(&test_workbook(), &o).unwrap().len(), 1);

        let mut o2 = opts(r"[Hh]ello\s\w+");
        o2.use_regex = true;
        o2.case_sensitive = true;
        assert_eq!(find(&test_workbook(), &o2).unwrap().len(), 2);
    }

    #[test]
    fn test_find_numbers_booleans_empty() {
        assert_eq!(find(&test_workbook(), &opts("42")).unwrap().len(), 1);
        assert_eq!(find(&test_workbook(), &opts("TRUE")).unwrap().len(), 1);
        assert!(find(&test_workbook(), &opts("")).unwrap().is_empty());
    }

    #[test]
    fn test_find_scoped_and_invalid_sheet() {
        let mut wb = test_workbook();
        wb.add_sheet("Sheet2").unwrap();
        wb.set_cell("Sheet2", 0, 0, CellValue::Text("Hello S2".into())).unwrap();
        let mut o = opts("Hello");
        o.sheet_name = Some("Sheet2".into());
        assert_eq!(find(&wb, &o).unwrap().len(), 1);

        let mut o2 = opts("x");
        o2.sheet_name = Some("NoSuch".into());
        assert!(find(&wb, &o2).is_err());
    }

    #[test]
    fn test_find_in_formulas_and_invalid_regex() {
        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        let mut cell = crate::cell::Cell::default();
        cell.value = CellValue::Number(10.0);
        cell.formula = Some("SUM(A1:A5)".into());
        sheet.set_cell(0, 0, cell);

        let mut o = opts("SUM");
        o.search_formulas = true;
        assert_eq!(find(&wb, &o).unwrap()[0].matched_text, "SUM");

        let mut o2 = opts("[bad");
        o2.use_regex = true;
        assert!(find(&test_workbook(), &o2).is_err());
    }

    #[test]
    fn test_replace_all_text() {
        let mut wb = test_workbook();
        let result = replace_all(&mut wb, &opts("hello"), "Hi").unwrap();
        assert_eq!(result.matches_found, 2);
        assert_eq!(result.replacements_made, 2);
        assert_eq!(wb.get_cell("Sheet1", 0, 0).unwrap().unwrap().value, CellValue::Text("Hi World".into()));
    }

    #[test]
    fn test_replace_skips_numbers() {
        let mut wb = test_workbook();
        let result = replace_all(&mut wb, &opts("42"), "99").unwrap();
        assert_eq!((result.matches_found, result.replacements_made), (1, 0));
    }

    #[test]
    fn test_replace_in_formulas() {
        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        let mut cell = crate::cell::Cell::default();
        cell.value = CellValue::Number(10.0);
        cell.formula = Some("SUM(A1:A5)".into());
        sheet.set_cell(0, 0, cell);

        let mut o = opts("SUM");
        o.search_formulas = true;
        assert_eq!(replace_all(&mut wb, &o, "AVERAGE").unwrap().replacements_made, 1);
        assert_eq!(wb.get_cell("Sheet1", 0, 0).unwrap().unwrap().formula.as_deref(), Some("AVERAGE(A1:A5)"));
    }

    #[test]
    fn test_replace_regex_and_empty_query() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("foo123bar".into())).unwrap();
        let mut o = opts(r"\d+");
        o.use_regex = true;
        assert_eq!(replace_all(&mut wb, &o, "NUM").unwrap().replacements_made, 1);
        assert_eq!(wb.get_cell("Sheet1", 0, 0).unwrap().unwrap().value, CellValue::Text("fooNUMbar".into()));

        let mut wb2 = test_workbook();
        assert_eq!(replace_all(&mut wb2, &opts(""), "X").unwrap().matches_found, 0);
    }

    #[test]
    fn test_find_across_multiple_sheets() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("apple".into())).unwrap();
        wb.add_sheet("Sheet2").unwrap();
        wb.set_cell("Sheet2", 0, 0, CellValue::Text("apple pie".into())).unwrap();
        let results = find(&wb, &opts("apple")).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].sheet, "Sheet1");
        assert_eq!(results[1].sheet, "Sheet2");
    }
}
