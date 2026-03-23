import { invoke } from '@tauri-apps/api/core';

/** A single border edge from the backend. */
export interface BorderEdgeData {
  style: string; // 'thin' | 'medium' | 'thick' | 'dashed' | 'dotted' | 'double' | 'none'
  color: string;
}

/** Cell borders from the backend. */
export interface CellBordersData {
  top?: BorderEdgeData | null;
  bottom?: BorderEdgeData | null;
  left?: BorderEdgeData | null;
  right?: BorderEdgeData | null;
}

/** Cell data returned from the Rust backend. */
export interface CellData {
  value: string;
  formula: string | null;
  format_id: number;
  bold: boolean;
  italic: boolean;
  underline: boolean;
  strikethrough: boolean;
  number_format: string | null;
  font_color: string | null;
  bg_color: string | null;
  font_family: string;
  h_align: string;
  font_size: number;
  text_wrap?: 'Overflow' | 'Wrap' | 'Clip';
  borders?: CellBordersData | null;
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

/** A border edge update sent to the backend. */
export interface BorderEdgeUpdate {
  style?: string; // 'none' | 'thin' | 'medium' | 'thick' | 'dashed' | 'dotted' | 'double'
  color?: string;
}

/** Borders update sent to the backend. */
export interface BordersUpdate {
  top?: BorderEdgeUpdate;
  bottom?: BorderEdgeUpdate;
  left?: BorderEdgeUpdate;
  right?: BorderEdgeUpdate;
}

/** Format options for format_cells command. */
export interface FormatOptions {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strikethrough?: boolean;
  font_size?: number;
  font_family?: string;
  font_color?: string;
  bg_color?: string;
  h_align?: 'left' | 'center' | 'right';
  v_align?: 'top' | 'middle' | 'bottom';
  number_format?: string;
  text_wrap?: 'Overflow' | 'Wrap' | 'Clip';
  borders?: BordersUpdate;
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
// Merge / Unmerge cells
// ---------------------------------------------------------------------------

/** A merged region returned from the backend. */
export interface MergedRegionData {
  start_row: number;
  start_col: number;
  end_row: number;
  end_col: number;
}

export async function mergeCells(
  sheet: string,
  startRow: number,
  startCol: number,
  endRow: number,
  endCol: number,
): Promise<void> {
  return invoke('merge_cells', { sheet, startRow, startCol, endRow, endCol });
}

export async function unmergeCells(
  sheet: string,
  row: number,
  col: number,
): Promise<boolean> {
  return invoke('unmerge_cells', { sheet, row, col });
}

export async function getMergedRegions(
  sheet: string,
): Promise<MergedRegionData[]> {
  return invoke('get_merged_regions', { sheet });
}

// ---------------------------------------------------------------------------
// Banded (alternating) rows
// ---------------------------------------------------------------------------

/** Banded row configuration from the backend. */
export interface BandedRowsData {
  enabled: boolean;
  even_color: string;
  odd_color: string;
  header_color: string | null;
}

export async function setBandedRows(
  sheet: string,
  enabled: boolean,
  evenColor: string,
  oddColor: string,
  headerColor?: string | null,
): Promise<void> {
  return invoke('set_banded_rows', {
    sheet,
    enabled,
    evenColor,
    oddColor,
    headerColor: headerColor ?? null,
  });
}

export async function getBandedRows(
  sheet: string,
): Promise<BandedRowsData | null> {
  return invoke('get_banded_rows', { sheet });
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
// Column/Row sizing (persist to backend)
// ---------------------------------------------------------------------------

export async function setColWidth(
  sheet: string,
  col: number,
  width: number,
): Promise<void> {
  return invoke('set_col_width', { sheet, col, width });
}

export async function setRowHeight(
  sheet: string,
  row: number,
  height: number,
): Promise<void> {
  return invoke('set_row_height', { sheet, row, height });
}

export async function getColWidths(
  sheet: string,
): Promise<Record<number, number>> {
  return invoke('get_col_widths', { sheet });
}

export async function getRowHeights(
  sheet: string,
): Promise<Record<number, number>> {
  return invoke('get_row_heights', { sheet });
}

// ---------------------------------------------------------------------------
// Sheet tab color and reorder
// ---------------------------------------------------------------------------

export async function setSheetTabColor(
  name: string,
  color: string | null,
): Promise<void> {
  return invoke('set_sheet_tab_color', { name, color });
}

export async function moveSheet(
  name: string,
  toIndex: number,
): Promise<void> {
  return invoke('move_sheet', { name, toIndex });
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

export async function openCsv(path: string): Promise<WorkbookInfo> {
  return invoke('open_csv', { path });
}

export async function openTsv(path: string): Promise<WorkbookInfo> {
  return invoke('open_tsv', { path });
}

export async function saveFile(path: string): Promise<void> {
  return invoke('save_file', { path });
}

export async function exportCsv(sheet: string, path: string): Promise<void> {
  return invoke('export_csv', { sheet, path });
}

export async function exportTsv(sheet: string, path: string): Promise<void> {
  return invoke('export_tsv', { sheet, path });
}

export async function exportHtml(sheet: string): Promise<string> {
  return invoke('export_html', { sheet });
}

export async function newWorkbook(): Promise<WorkbookInfo> {
  return invoke('new_workbook');
}

// ---------------------------------------------------------------------------
// Recent files
// ---------------------------------------------------------------------------

/** A recently opened file entry. */
export interface RecentFile {
  path: string;
  name: string;
  last_opened: string;
}

export async function getRecentFiles(): Promise<RecentFile[]> {
  return invoke('get_recent_files');
}

export async function addRecentFile(path: string, name: string): Promise<void> {
  return invoke('add_recent_file', { path, name });
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

// ---------------------------------------------------------------------------
// Validation commands
// ---------------------------------------------------------------------------

/** Validation rule data returned from the backend. */
export interface ValidationData {
  rule_type: string;
  list_items: string | null;
  min: number | null;
  max: number | null;
  min_date: string | null;
  max_date: string | null;
  formula: string | null;
  allow_blank: boolean;
  error_message: string | null;
}

export async function setValidation(
  sheet: string,
  row: number,
  col: number,
  ruleType: string,
  listItems?: string,
  min?: number,
  max?: number,
  minDate?: string,
  maxDate?: string,
  formula?: string,
  allowBlank?: boolean,
  errorMessage?: string,
): Promise<void> {
  return invoke('set_validation', {
    sheet,
    row,
    col,
    ruleType,
    listItems: listItems ?? null,
    min: min ?? null,
    max: max ?? null,
    minDate: minDate ?? null,
    maxDate: maxDate ?? null,
    formula: formula ?? null,
    allowBlank: allowBlank ?? true,
    errorMessage: errorMessage ?? null,
  });
}

export async function getValidation(
  sheet: string,
  row: number,
  col: number,
): Promise<ValidationData | null> {
  return invoke('get_validation', { sheet, row, col });
}

export async function removeValidation(
  sheet: string,
  row: number,
  col: number,
): Promise<void> {
  return invoke('remove_validation', { sheet, row, col });
}

export async function listValidations(
  sheet: string,
): Promise<[number, number, ValidationData][]> {
  return invoke('list_validations', { sheet });
}

// ---------------------------------------------------------------------------
// Filter commands
// ---------------------------------------------------------------------------

/** Filter state information returned from the backend. */
export interface FilterInfo {
  active: boolean;
  start_col: number;
  end_col: number;
  header_row: number;
  filtered_cols: number[];
  total_rows: number;
  visible_rows: number;
}

export async function setAutoFilter(
  sheet: string,
): Promise<FilterInfo> {
  return invoke('set_auto_filter', { sheet });
}

export async function getColumnValues(
  sheet: string,
  col: number,
): Promise<string[]> {
  return invoke('get_column_values', { sheet, col });
}

export async function applyColumnFilter(
  sheet: string,
  col: number,
  values: string[],
): Promise<FilterInfo> {
  return invoke('apply_column_filter', { sheet, col, values });
}

export async function clearFilter(
  sheet: string,
): Promise<void> {
  return invoke('clear_filter', { sheet });
}

export async function getFilterInfo(
  sheet: string,
): Promise<FilterInfo> {
  return invoke('get_filter_info', { sheet });
}

export async function getHiddenRows(
  sheet: string,
): Promise<number[]> {
  return invoke('get_hidden_rows', { sheet });
}

export async function hideRows(
  sheet: string,
  startRow: number,
  count: number,
): Promise<void> {
  return invoke('hide_rows', { sheet, startRow, count });
}

export async function unhideRows(
  sheet: string,
  startRow: number,
  count: number,
): Promise<void> {
  return invoke('unhide_rows', { sheet, startRow, count });
}

export async function hideCols(
  sheet: string,
  startCol: number,
  count: number,
): Promise<void> {
  return invoke('hide_cols', { sheet, startCol, count });
}

export async function unhideCols(
  sheet: string,
  startCol: number,
  count: number,
): Promise<void> {
  return invoke('unhide_cols', { sheet, startCol, count });
}

export async function getHiddenCols(
  sheet: string,
): Promise<number[]> {
  return invoke('get_hidden_cols', { sheet });
}

// ---------------------------------------------------------------------------
// Sort commands
// ---------------------------------------------------------------------------

/** A sort key specifying column and direction. */
export interface SortKeyInput {
  col: number;
  direction: 'asc' | 'desc';
}

export async function sortRange(
  sheet: string,
  range: string | null,
  sortKeys: SortKeyInput[],
): Promise<void> {
  return invoke('sort_range', { sheet, range, sortKeys });
}

// ---------------------------------------------------------------------------
// Named range commands
// ---------------------------------------------------------------------------

/** Named range info returned from the backend. */
export interface NamedRangeInfo {
  name: string;
  sheet: string | null;
  range: string;
}

export async function addNamedRange(
  name: string,
  range: string,
  sheet?: string,
): Promise<void> {
  return invoke('add_named_range', { name, range, sheet: sheet ?? null });
}

export async function listNamedRanges(): Promise<NamedRangeInfo[]> {
  return invoke('list_named_ranges');
}

export async function removeNamedRange(name: string): Promise<void> {
  return invoke('remove_named_range', { name });
}

export async function resolveNamedRange(
  name: string,
): Promise<NamedRangeInfo> {
  return invoke('resolve_named_range', { name });
}

// ---------------------------------------------------------------------------
// Conditional format commands
// ---------------------------------------------------------------------------

/** Rule type input for adding conditional format rules. */
export interface RuleTypeInput {
  kind: string; // 'cell_value' | 'text_contains' | 'is_blank' | 'is_not_blank' | 'is_error'
  operator?: string; // '>' | '<' | '>=' | '<=' | '=' | '!=' | 'between'
  value1?: number;
  value2?: number;
  text?: string;
}

/** Style to apply when a conditional format rule matches. */
export interface ConditionalStyleInput {
  bold?: boolean;
  italic?: boolean;
  font_color?: string;
  bg_color?: string;
}

/** A single rule in a conditional format range (from list). */
export interface RuleOutput {
  kind: string;
  description: string;
  bold?: boolean | null;
  italic?: boolean | null;
  font_color?: string | null;
  bg_color?: string | null;
}

/** A conditional format range (from list). */
export interface ConditionalFormatOutput {
  start_row: number;
  start_col: number;
  end_row: number;
  end_col: number;
  rules: RuleOutput[];
}

export async function addConditionalFormat(
  sheet: string,
  startRow: number,
  startCol: number,
  endRow: number,
  endCol: number,
  ruleType: RuleTypeInput,
  style: ConditionalStyleInput,
): Promise<void> {
  return invoke('add_conditional_format', {
    sheet,
    startRow,
    startCol,
    endRow,
    endCol,
    ruleType,
    style,
  });
}

export async function listConditionalFormats(
  sheet: string,
): Promise<ConditionalFormatOutput[]> {
  return invoke('list_conditional_formats', { sheet });
}

export async function removeConditionalFormat(
  sheet: string,
  startRow: number,
  startCol: number,
  endRow: number,
  endCol: number,
  ruleIndex: number,
): Promise<void> {
  return invoke('remove_conditional_format', {
    sheet,
    startRow,
    startCol,
    endRow,
    endCol,
    ruleIndex,
  });
}
