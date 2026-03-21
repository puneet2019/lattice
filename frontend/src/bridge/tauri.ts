import { invoke } from '@tauri-apps/api/core';

/** Cell data returned from the Rust backend. */
export interface CellData {
  value: string;
  formula: string | null;
  format_id: number;
  bold: boolean;
  italic: boolean;
}

/** Sheet summary information. */
export interface SheetInfo {
  name: string;
  is_active: boolean;
}

/** Workbook summary returned after open/new. */
export interface WorkbookInfo {
  sheets: string[];
  active_sheet: string;
}

// ---------------------------------------------------------------------------
// Cell commands
// ---------------------------------------------------------------------------

export async function getCell(
  sheet: string,
  row: number,
  col: number,
): Promise<CellData | null> {
  return invoke('get_cell', { sheet, row, col });
}

export async function setCell(
  sheet: string,
  row: number,
  col: number,
  value: string,
  formula?: string,
): Promise<void> {
  return invoke('set_cell', { sheet, row, col, value, formula });
}

export async function getRange(
  sheet: string,
  startRow: number,
  startCol: number,
  endRow: number,
  endCol: number,
): Promise<(CellData | null)[][]> {
  return invoke('get_range', {
    sheet,
    startRow,
    startCol,
    endRow,
    endCol,
  });
}

// ---------------------------------------------------------------------------
// Sheet commands
// ---------------------------------------------------------------------------

export async function listSheets(): Promise<SheetInfo[]> {
  return invoke('list_sheets');
}

export async function addSheet(name: string): Promise<void> {
  return invoke('add_sheet', { name });
}

export async function renameSheet(
  old: string,
  newName: string,
): Promise<void> {
  return invoke('rename_sheet', { old, newName });
}

export async function deleteSheet(name: string): Promise<void> {
  return invoke('delete_sheet', { name });
}

export async function setActiveSheet(name: string): Promise<void> {
  return invoke('set_active_sheet', { name });
}

// ---------------------------------------------------------------------------
// File commands
// ---------------------------------------------------------------------------

export async function openFile(path: string): Promise<WorkbookInfo> {
  return invoke('open_file', { path });
}

export async function saveFile(path: string): Promise<void> {
  return invoke('save_file', { path });
}

export async function newWorkbook(): Promise<WorkbookInfo> {
  return invoke('new_workbook');
}

// ---------------------------------------------------------------------------
// Edit commands
// ---------------------------------------------------------------------------

export async function undo(): Promise<void> {
  return invoke('undo');
}

export async function redo(): Promise<void> {
  return invoke('redo');
}
