use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_core::{Border, BorderStyle, CellFormat, HAlign, Operation, TextWrap};

use crate::state::AppState;

/// A single border edge update from the frontend.
#[derive(Debug, Clone, Deserialize)]
pub struct BorderEdgeUpdate {
    pub style: Option<String>,
    pub color: Option<String>,
}

/// Borders update from the frontend.
#[derive(Debug, Clone, Deserialize)]
pub struct BordersUpdate {
    pub top: Option<BorderEdgeUpdate>,
    pub bottom: Option<BorderEdgeUpdate>,
    pub left: Option<BorderEdgeUpdate>,
    pub right: Option<BorderEdgeUpdate>,
}

/// Format properties to apply to cells.
#[derive(Debug, Clone, Deserialize)]
pub struct FormatUpdate {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub font_size: Option<f64>,
    pub font_family: Option<String>,
    pub font_color: Option<String>,
    pub bg_color: Option<String>,
    pub h_align: Option<String>,
    pub number_format: Option<String>,
    pub text_wrap: Option<String>,
    pub borders: Option<BordersUpdate>,
}

/// Apply formatting to a range of cells.
///
/// Creates default cells for empty positions so users can pre-format
/// cells before typing. Pushes a `FormatCells` operation to the undo
/// stack so the change is reversible.
#[tauri::command]
pub async fn format_cells(
    state: State<'_, AppState>,
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
    format: FormatUpdate,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;

    // Collect old/new formats for undo.
    let mut changed: Vec<(u32, u32, CellFormat, CellFormat)> = Vec::new();

    for row in start_row..=end_row {
        for col in start_col..=end_col {
            // Get existing cell or create a default one for empty positions.
            let mut cell = s.get_cell(row, col).cloned().unwrap_or_default();
            let old_format = cell.format.clone();

            if let Some(bold) = format.bold {
                cell.format.bold = bold;
            }
            if let Some(italic) = format.italic {
                cell.format.italic = italic;
            }
            if let Some(size) = format.font_size {
                cell.format.font_size = size;
            }
            if let Some(ref family) = format.font_family {
                cell.format.font_family = family.clone();
            }
            if let Some(ref color) = format.font_color {
                cell.format.font_color = Some(color.clone());
            }
            if let Some(underline) = format.underline {
                cell.format.underline = underline;
            }
            if let Some(strikethrough) = format.strikethrough {
                cell.format.strikethrough = strikethrough;
            }
            if let Some(ref bg) = format.bg_color {
                if bg.is_empty() {
                    // Empty string means "clear background color"
                    cell.format.bg_color = None;
                } else {
                    cell.format.bg_color = Some(bg.clone());
                }
            }
            if let Some(ref align) = format.h_align {
                cell.format.h_align = match align.as_str() {
                    "center" => HAlign::Center,
                    "right" => HAlign::Right,
                    _ => HAlign::Left,
                };
            }
            if let Some(ref nf) = format.number_format {
                cell.format.number_format = Some(nf.clone());
            }
            if let Some(ref tw) = format.text_wrap {
                cell.format.text_wrap = match tw.as_str() {
                    "Wrap" => TextWrap::Wrap,
                    "Clip" => TextWrap::Clip,
                    _ => TextWrap::Overflow,
                };
            }

            if let Some(ref borders) = format.borders {
                if let Some(ref edge) = borders.top {
                    cell.format.borders.top = parse_border_edge(edge);
                }
                if let Some(ref edge) = borders.bottom {
                    cell.format.borders.bottom = parse_border_edge(edge);
                }
                if let Some(ref edge) = borders.left {
                    cell.format.borders.left = parse_border_edge(edge);
                }
                if let Some(ref edge) = borders.right {
                    cell.format.borders.right = parse_border_edge(edge);
                }
            }

            let new_format = cell.format.clone();

            // Only record if format actually changed.
            if old_format != new_format {
                changed.push((row, col, old_format, new_format));
            }

            s.set_cell(row, col, cell);
        }
    }

    // Push to undo stack if any formats changed.
    if !changed.is_empty() {
        let mut stack = state.undo_stack.write().await;
        stack.push(Operation::FormatCells {
            sheet,
            cells: changed,
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Merge / Unmerge cells
// ---------------------------------------------------------------------------

/// A merged region returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedRegionData {
    pub start_row: u32,
    pub start_col: u32,
    pub end_row: u32,
    pub end_col: u32,
}

/// Merge a rectangular region of cells.
///
/// The value of the merged cell comes from the top-left cell. All other
/// cells in the region are cleared. Returns an error if the region
/// overlaps an existing merged region.
#[tauri::command]
pub async fn merge_cells(
    state: State<'_, AppState>,
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.merge_cells(start_row, start_col, end_row, end_col)
        .map_err(|e| e.to_string())
}

/// Unmerge any merged region containing the given cell.
///
/// Returns `true` if a region was unmerged, `false` if the cell was
/// not part of any merged region.
#[tauri::command]
pub async fn unmerge_cells(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
) -> Result<bool, String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.unmerge_cell(row, col).map_err(|e| e.to_string())
}

/// Return all merged regions for a sheet.
#[tauri::command]
pub async fn get_merged_regions(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<Vec<MergedRegionData>, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;
    Ok(s.merged_regions()
        .iter()
        .map(|r| MergedRegionData {
            start_row: r.start_row,
            start_col: r.start_col,
            end_row: r.end_row,
            end_col: r.end_col,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Banded (alternating) rows
// ---------------------------------------------------------------------------

/// Banded row configuration returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandedRowsData {
    pub enabled: bool,
    pub even_color: String,
    pub odd_color: String,
    pub header_color: Option<String>,
}

/// Set banded (alternating) row colours on a sheet.
#[tauri::command]
pub async fn set_banded_rows(
    state: State<'_, AppState>,
    sheet: String,
    enabled: bool,
    even_color: String,
    odd_color: String,
    header_color: Option<String>,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    if enabled {
        s.banded_rows = Some(lattice_core::BandedRows {
            enabled: true,
            even_color,
            odd_color,
            header_color,
        });
    } else {
        s.banded_rows = None;
    }
    Ok(())
}

/// Get the banded row configuration for a sheet.
#[tauri::command]
pub async fn get_banded_rows(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<Option<BandedRowsData>, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;
    Ok(s.banded_rows.as_ref().map(|b| BandedRowsData {
        enabled: b.enabled,
        even_color: b.even_color.clone(),
        odd_color: b.odd_color.clone(),
        header_color: b.header_color.clone(),
    }))
}

/// Parse a border edge update into a core `Border`, or `None` if the
/// style is "none" (meaning remove the border).
fn parse_border_edge(edge: &BorderEdgeUpdate) -> Option<Border> {
    let style_str = edge.style.as_deref().unwrap_or("thin");
    let style = match style_str {
        "none" => return None,
        "thin" => BorderStyle::Thin,
        "medium" => BorderStyle::Medium,
        "thick" => BorderStyle::Thick,
        "dashed" => BorderStyle::Dashed,
        "dotted" => BorderStyle::Dotted,
        "double" => BorderStyle::Double,
        _ => BorderStyle::Thin,
    };
    let color = edge.color.as_deref().unwrap_or("#000000").to_string();
    Some(Border { style, color })
}
