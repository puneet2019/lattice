use serde::Deserialize;
use tauri::State;

use lattice_core::HAlign;

use crate::state::AppState;

/// Format properties to apply to cells.
#[derive(Debug, Clone, Deserialize)]
pub struct FormatUpdate {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub font_size: Option<f64>,
    pub font_color: Option<String>,
    pub bg_color: Option<String>,
    pub h_align: Option<String>,
}

/// Apply formatting to a range of cells.
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

    for row in start_row..=end_row {
        for col in start_col..=end_col {
            if let Some(cell) = s.get_cell(row, col) {
                let mut cell = cell.clone();
                if let Some(bold) = format.bold {
                    cell.format.bold = bold;
                }
                if let Some(italic) = format.italic {
                    cell.format.italic = italic;
                }
                if let Some(size) = format.font_size {
                    cell.format.font_size = size;
                }
                if let Some(ref color) = format.font_color {
                    cell.format.font_color = color.clone();
                }
                if let Some(underline) = format.underline {
                    cell.format.underline = underline;
                }
                if let Some(strikethrough) = format.strikethrough {
                    cell.format.strikethrough = strikethrough;
                }
                if let Some(ref bg) = format.bg_color {
                    cell.format.bg_color = Some(bg.clone());
                }
                if let Some(ref align) = format.h_align {
                    cell.format.h_align = match align.as_str() {
                        "center" => HAlign::Center,
                        "right" => HAlign::Right,
                        _ => HAlign::Left,
                    };
                }
                s.set_cell(row, col, cell);
            }
        }
    }

    Ok(())
}
