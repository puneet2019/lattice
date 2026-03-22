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

/** Format options for format_cells command. */
export interface FormatOptions {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  font_size?: number;
  font_color?: string;
  bg_color?: string;
  h_align?: 'left' | 'center' | 'right';
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

/**
 * Store an image (as a data URL) in a cell.
 * The image is stored as the cell value with a `data:image/...` prefix.
 */
export async function setCellImage(
  sheet: string,
  row: number,
  col: number,
  imageDataUrl: string,
): Promise<void> {
  return setCell(sheet, row, col, imageDataUrl, undefined);
}

// ---------------------------------------------------------------------------
// Format commands
// ---------------------------------------------------------------------------

export async function formatCells(
  sheet: string,
  startRow: number,
  startCol: number,
  endRow: number,
  endCol: number,
  format: FormatOptions,
): Promise<void> {
  return invoke('format_cells', {
    sheet,
    startRow,
    startCol,
    endRow,
    endCol,
    format,
  });
}

// ---------------------------------------------------------------------------
// Row/Column manipulation
// ---------------------------------------------------------------------------

export async function insertRows(
  sheet: string,
  row: number,
  count: number,
): Promise<void> {
  return invoke('insert_rows', { sheet, row, count });
}

export async function deleteRows(
  sheet: string,
  row: number,
  count: number,
): Promise<void> {
  return invoke('delete_rows', { sheet, row, count });
}

export async function insertCols(
  sheet: string,
  col: number,
  count: number,
): Promise<void> {
  return invoke('insert_cols', { sheet, col, count });
}

export async function deleteCols(
  sheet: string,
  col: number,
  count: number,
): Promise<void> {
  return invoke('delete_cols', { sheet, col, count });
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

export interface FindResult {
  row: number;
  col: number;
  value: string;
}

export async function findInSheet(
  sheet: string,
  query: string,
): Promise<FindResult[]> {
  return invoke('find_in_sheet', { sheet, query });
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

export async function duplicateSheet(
  source: string,
  newName: string,
): Promise<void> {
  return invoke('duplicate_sheet', { source, newName });
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

// ---------------------------------------------------------------------------
// Chart commands
// ---------------------------------------------------------------------------

/** Chart metadata returned from the backend. */
export interface ChartInfo {
  id: string;
  chart_type: string;
  data_range: string;
  sheet: string;
  title: string | null;
  width: number;
  height: number;
}

/** Valid chart type strings. */
export type ChartTypeStr =
  | 'bar'
  | 'line'
  | 'pie'
  | 'scatter'
  | 'area'
  | 'combo'
  | 'histogram'
  | 'candlestick';

export async function createChart(
  sheet: string,
  chartType: ChartTypeStr,
  dataRange: string,
  title?: string,
): Promise<string> {
  return invoke('create_chart', {
    sheet,
    chartType,
    dataRange,
    title: title ?? null,
  });
}

export async function renderChartSvg(chartId: string): Promise<string> {
  return invoke('render_chart_svg', { chartId });
}

export async function listCharts(
  sheet?: string,
): Promise<ChartInfo[]> {
  return invoke('list_charts', { sheet: sheet ?? null });
}

export async function deleteChart(chartId: string): Promise<void> {
  return invoke('delete_chart', { chartId });
}
