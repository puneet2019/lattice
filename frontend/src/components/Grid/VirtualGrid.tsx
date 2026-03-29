import type { Component } from 'solid-js';
import { createSignal, createEffect, onMount, onCleanup, Show } from 'solid-js';
import { col_to_letter } from '../../bridge/tauri_helpers';
import type { CellData, MergedRegionData, BandedRowsData, FormatOptions, ConditionalFormatOutput, RowGroupData } from '../../bridge/tauri';
import type { PasteMode } from '../PasteSpecialDialog';
import {
  getCell,
  getRange,
  setCell,
  undo,
  redo,
  formatCells,
  insertRows,
  deleteRows,
  insertCols,
  deleteCols,
  setColWidth as tauriSetColWidth,
  setRowHeight as tauriSetRowHeight,
  getColWidths,
  getRowHeights,
  listValidations,
  getHiddenRows,
  hideRows,
  unhideRows,
  hideCols,
  unhideCols,
  getHiddenCols,
  sortRange,
  listConditionalFormats,
  getRowGroups,
  addRowGroup,
  removeRowGroup,
  toggleRowGroup,
} from '../../bridge/tauri';
import type { ValidationData } from '../../bridge/tauri';
import AutoComplete, { getColumnSuggestions } from './AutoComplete';
import FormulaAutoComplete, { extractCurrentToken, filterFormulaFunctions } from './FormulaAutoComplete';
import FormulaHint from './FormulaHint';
import {
  DEFAULT_COL_WIDTH,
  DEFAULT_ROW_HEIGHT,
  MIN_COL_WIDTH,
  MIN_ROW_HEIGHT,
  HEADER_HEIGHT,
  ROW_NUMBER_WIDTH,
  TOTAL_COLS,
  TOTAL_ROWS,
} from './constants';

// Re-export constants for backward compatibility.
export {
  DEFAULT_COL_WIDTH,
  DEFAULT_ROW_HEIGHT,
  MIN_COL_WIDTH,
  MIN_ROW_HEIGHT,
  HEADER_HEIGHT,
  ROW_NUMBER_WIDTH,
  TOTAL_COLS,
  TOTAL_ROWS,
} from './constants';

// Theme colors — kept in sync with grid.css CSS variables.
// Light and dark palettes are defined separately and switched
// based on the system `prefers-color-scheme` media query.

interface ThemeColors {
  headerBg: string;
  headerText: string;
  gridBorder: string;
  selectionBorder: string;
  selectionBg: string;
  cornerBg: string;
  cellText: string;
  cellBg: string;
  freezeBorder: string;
}

const LIGHT_COLORS: ThemeColors = {
  headerBg: '#f8f9fa',
  headerText: '#5f6368',
  gridBorder: '#e0e0e0',
  selectionBorder: '#1a73e8',
  selectionBg: 'rgba(26, 115, 232, 0.08)',
  cornerBg: '#f8f9fa',
  cellText: '#202124',
  cellBg: '#ffffff',
  freezeBorder: '#1a73e8',
};

const DARK_COLORS: ThemeColors = {
  headerBg: '#292a2d',
  headerText: '#9aa0a6',
  gridBorder: '#3c4043',
  selectionBorder: '#8ab4f8',
  selectionBg: 'rgba(138, 180, 248, 0.12)',
  cornerBg: '#292a2d',
  cellText: '#e8eaed',
  cellBg: '#202124',
  freezeBorder: '#8ab4f8',
};

/** Detect system dark mode preference. */
function getSystemDarkMode(): boolean {
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
}

/** Return the appropriate color palette for the current system preference. */
function getColors(): ThemeColors {
  return getSystemDarkMode() ? DARK_COLORS : LIGHT_COLORS;
}

// Module-level mutable reference; updated by the matchMedia listener inside
// the component. Canvas draw calls read from this.
let COLORS: ThemeColors = getColors();

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface VirtualGridProps {
  activeSheet: string;
  /** Increment to trigger a data refresh (e.g. after formula bar commit). */
  refreshTrigger?: number;
  /** Number of rows to freeze at the top (0 = none). */
  frozenRows?: number;
  /** Number of columns to freeze at the left (0 = none). */
  frozenCols?: number;
  /** Split pane: row index where the horizontal split occurs (0 = no split). */
  splitRow?: number;
  /** Split pane: column index where the vertical split occurs (0 = no split). */
  splitCol?: number;
  /** Zoom level (1.0 = 100%). Applied to the canvas rendering. */
  zoom?: number;
  onSelectionChange: (row: number, col: number, minRow?: number, minCol?: number, maxRow?: number, maxCol?: number) => void;
  onContentChange: (content: string) => void;
  onCellCommit: (row: number, col: number, value: string) => void;
  onStatusChange: (message: string) => void;
  onModeChange: (mode: 'Ready' | 'Edit') => void;
  onBoldToggle: () => void;
  onItalicToggle: () => void;
  onUnderlineToggle: () => void;
  onFindOpen?: () => void;
  onFindReplaceOpen?: () => void;
  onZoomIn?: () => void;
  onZoomOut?: () => void;
  onZoomReset?: () => void;
  /** Called when the user triggers Cmd+Shift+P to open the paste special dialog. */
  onPasteSpecialOpen?: () => void;
  /**
   * When set to a non-null PasteMode, VirtualGrid executes the corresponding
   * paste operation and then calls onPasteSpecialDone to reset.
   */
  pasteSpecialMode?: PasteMode | null;
  /** Called after a paste special operation completes so App can reset the signal. */
  onPasteSpecialDone?: () => void;
  /** Called when selection summary (Sum/Average/Count) changes for the status bar. */
  onSelectionSummary?: (summary: string) => void;
  /** Find match positions to highlight on canvas. */
  findMatches?: { row: number; col: number }[];
  /** Index of the active find match (highlighted differently). */
  findActiveIndex?: number;
  /** Called when the user selects "Format cells..." from context menu. */
  onFormatCellsOpen?: () => void;
  /** Called when the user selects "Data validation..." from context menu. */
  onDataValidationOpen?: () => void;
  /** Whether auto-filter is active on this sheet. */
  filterActive?: boolean;
  /** Start and end columns of the filter range (for drawing filter arrows). */
  filterStartCol?: number;
  filterEndCol?: number;
  /** Called when user clicks a filter arrow on a column header. */
  onFilterColumnClick?: (col: number, x: number, y: number) => void;
  /** Called when user selects "Custom sort..." from context menu. */
  onSortDialogOpen?: () => void;
  /** Called when the user opens the named ranges dialog (Ctrl+F3). */
  onNamedRangesOpen?: () => void;
  /** Whether to draw grid lines (default true). */
  showGridlines?: boolean;
  /** Whether to show formulas instead of values. Controlled by App. */
  showFormulas?: boolean;
  /** Called when the user toggles formula view via Ctrl+`. */
  onToggleFormulas?: (value: boolean) => void;
  /** Merged regions for the active sheet. */
  mergedRegions?: MergedRegionData[];
  /** Banded (alternating) row configuration for the active sheet. */
  bandedRows?: BandedRowsData | null;
  /** Called when the user presses Option+Down to switch to the next sheet tab. */
  onNextSheet?: () => void;
  /** Called when the user presses Option+Up to switch to the previous sheet tab. */
  onPrevSheet?: () => void;
  /** Called when the user presses Cmd+/ to open the keyboard shortcuts help dialog. */
  onKeyboardShortcutsOpen?: () => void;
  /** Called when the user presses Cmd+P to open the print preview dialog. */
  onPrintPreviewOpen?: () => void;
  /** When true, draw blue dashed lines at page break positions. */
  pageBreakPreview?: boolean;
  /** Paper size for page break calculation: 'letter'|'a4'|'legal'|'tabloid'. */
  pageBreakPaperSize?: string;
  /** Orientation for page break calculation: 'portrait'|'landscape'. */
  pageBreakOrientation?: string;
  /** External navigation request: [row, col, anchorRow, anchorCol, endRow, endCol]. Increment to trigger. */
  navigateTo?: [number, number, number, number, number, number] | null;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const VirtualGrid: Component<VirtualGridProps> = (props) => {
  let containerRef: HTMLDivElement | undefined;
  let canvasRef: HTMLCanvasElement | undefined;

  const [scrollX, setScrollX] = createSignal(0);
  const [scrollY, setScrollY] = createSignal(0);
  const [canvasWidth, setCanvasWidth] = createSignal(800);
  const [canvasHeight, setCanvasHeight] = createSignal(600);

  // Split pane scroll state: top-left pane has separate scroll from bottom-right.
  // scrollX/scrollY are used for the bottom-right pane.
  // splitScrollX/splitScrollY are used for the top/left panes.
  const [splitScrollX, setSplitScrollX] = createSignal(0);
  const [splitScrollY, setSplitScrollY] = createSignal(0);

  // Selection state
  const [selectedRow, setSelectedRow] = createSignal(0);
  const [selectedCol, setSelectedCol] = createSignal(0);
  const [rangeAnchor, setRangeAnchor] = createSignal<[number, number] | null>(null);
  const [rangeEnd, setRangeEnd] = createSignal<[number, number] | null>(null);

  // Marching ants (copy indicator) state
  let copiedRange: { minRow: number; maxRow: number; minCol: number; maxCol: number } | null = null;
  let marchingAntOffset = 0;
  let marchingAntAnimId: number | null = null;

  function startMarchingAnts() {
    if (marchingAntAnimId !== null) return;
    const animate = () => {
      marchingAntOffset = (marchingAntOffset + 0.5) % 16;
      scheduleDraw();
      marchingAntAnimId = requestAnimationFrame(animate);
    };
    marchingAntAnimId = requestAnimationFrame(animate);
  }

  function stopMarchingAnts() {
    if (marchingAntAnimId !== null) {
      cancelAnimationFrame(marchingAntAnimId);
      marchingAntAnimId = null;
    }
    copiedRange = null;
    marchingAntOffset = 0;
  }

  // Find match tracking for canvas highlights
  let findMatchSet: Set<string> | null = null;
  let findActiveRow = -1;
  let findActiveCol = -1;

  // Cell data cache: maps "row:col" to CellData
  const cellCache = new Map<string, CellData>();
  let lastFetchKey = ''; // tracks last fetched range to avoid duplicate calls

  // Image cache: maps data URL to loaded HTMLImageElement (or null if loading).
  const imageCache = new Map<string, HTMLImageElement | null>();

  // Validation cache: maps "row:col" to ValidationData for the current sheet.
  const validationCache = new Map<string, ValidationData>();

  // Conditional format rules for the current sheet.
  let conditionalFormatRules: ConditionalFormatOutput[] = [];

  // Row groups for the current sheet (collapsible sections).
  let rowGroups: RowGroupData[] = [];

  // Hidden rows set for the current sheet (from auto-filter and manual hide).
  const hiddenRows = new Set<number>();
  // Hidden cols set for the current sheet (from manual hide).
  const hiddenCols = new Set<number>();

  // Editing state
  const [editing, setEditing] = createSignal(false);
  const [editValue, setEditValue] = createSignal('');
  let editorRef: HTMLTextAreaElement | undefined;

  // Auto-complete state (cell value suggestions)
  const [acVisible, setAcVisible] = createSignal(false);
  const [acSuggestions, setAcSuggestions] = createSignal<string[]>([]);
  const [acSelectedIdx, setAcSelectedIdx] = createSignal(0);

  // Formula auto-complete state (function name suggestions)
  const [formulaAcVisible, setFormulaAcVisible] = createSignal(false);
  const [formulaAcSelectedIdx, setFormulaAcSelectedIdx] = createSignal(0);

  // Cursor position in the editor (for formula argument hints)
  const [editorCursorPos, setEditorCursorPos] = createSignal(0);

  // Context menu state
  const [ctxMenuVisible, setCtxMenuVisible] = createSignal(false);
  const [ctxMenuX, setCtxMenuX] = createSignal(0);
  const [ctxMenuY, setCtxMenuY] = createSignal(0);
  // Track where the right-click originated: 'cell' | 'row-header' | 'col-header'
  const [ctxMenuTarget, setCtxMenuTarget] = createSignal<'cell' | 'row-header' | 'col-header'>('cell');

  // Formula view: show raw formulas instead of computed values.
  // When the parent provides the showFormulas prop, use that; otherwise
  // fall back to a local signal.
  const [localShowFormulas, setLocalShowFormulas] = createSignal(false);
  const showFormulas = () => props.showFormulas !== undefined ? props.showFormulas : localShowFormulas();
  const setShowFormulas = (v: boolean) => {
    if (props.onToggleFormulas) {
      props.onToggleFormulas(v);
    } else {
      setLocalShowFormulas(v);
    }
  };

  // Validation dropdown state (for cells with List validation)
  const [validationDropdownVisible, setValidationDropdownVisible] = createSignal(false);
  const [validationDropdownItems, setValidationDropdownItems] = createSignal<string[]>([]);
  const [validationDropdownX, setValidationDropdownX] = createSignal(0);
  const [validationDropdownY, setValidationDropdownY] = createSignal(0);
  const [validationDropdownRow, setValidationDropdownRow] = createSignal(0);
  const [validationDropdownCol, setValidationDropdownCol] = createSignal(0);

  // Formula click-to-reference state.
  // When editing a formula (value starts with '='), clicking a cell inserts a
  // reference instead of moving the selection.  Dragging inserts a range ref.
  let formulaRefDragging = false;
  let formulaRefAnchor: { row: number; col: number } | null = null;
  // Track what was inserted so we can replace it during drag-extend.
  let formulaRefInsertStart = -1;
  let formulaRefInsertEnd = -1;

  /** Colors used to highlight referenced ranges in formulas. */
  const FORMULA_REF_COLORS = [
    '#1a73e8', // blue
    '#ea4335', // red
    '#34a853', // green
    '#9334e6', // purple
    '#e8710a', // orange
    '#00897b', // teal
  ];

  /** Whether the editor is in formula selection mode. */
  function isFormulaSelectionMode(): boolean {
    return editing() && editValue().trimStart().startsWith('=');
  }

  /** Parse cell references from a formula string and return their ranges with colors. */
  function parseFormulaRefs(formula: string): { minRow: number; minCol: number; maxRow: number; maxCol: number; color: string }[] {
    if (!formula.startsWith('=')) return [];
    const refs: { minRow: number; minCol: number; maxRow: number; maxCol: number; color: string }[] = [];
    // Match cell references and range references (e.g., A1, B2:D5, $A$1)
    const refPattern = /\$?[A-Za-z]{1,3}\$?\d+(?::\$?[A-Za-z]{1,3}\$?\d+)?/g;
    let match: RegExpExecArray | null;
    let colorIdx = 0;
    while ((match = refPattern.exec(formula)) !== null) {
      const refStr = match[0];
      const parts = refStr.split(':');
      const clean = (s: string) => s.replace(/\$/g, '');
      const startRef = parseRefCoords(clean(parts[0]));
      if (!startRef) continue;
      let endRef = startRef;
      if (parts.length === 2) {
        endRef = parseRefCoords(clean(parts[1])) ?? startRef;
      }
      refs.push({
        minRow: Math.min(startRef.row, endRef.row),
        minCol: Math.min(startRef.col, endRef.col),
        maxRow: Math.max(startRef.row, endRef.row),
        maxCol: Math.max(startRef.col, endRef.col),
        color: FORMULA_REF_COLORS[colorIdx % FORMULA_REF_COLORS.length],
      });
      colorIdx++;
    }
    return refs;
  }

  /** Parse "A1"-style reference to {row, col}. */
  function parseRefCoords(ref: string): { row: number; col: number } | null {
    const m = ref.match(/^([A-Za-z]+)(\d+)$/);
    if (!m) return null;
    const letters = m[1].toUpperCase();
    const rowNum = parseInt(m[2], 10);
    if (isNaN(rowNum) || rowNum < 1) return null;
    let col = 0;
    for (let i = 0; i < letters.length; i++) {
      col = col * 26 + (letters.charCodeAt(i) - 64);
    }
    return { row: rowNum - 1, col: col - 1 };
  }

  // Custom column widths / row heights (col/row index -> px).
  // Columns/rows not in the map use the default sizes.
  const colWidths = new Map<number, number>();
  const rowHeights = new Map<number, number>();

  // -----------------------------------------------------------------------
  // Variable-width/height helpers
  // -----------------------------------------------------------------------

  /** Return the width of a specific column. */
  function getColWidth(col: number): number {
    return colWidths.get(col) ?? DEFAULT_COL_WIDTH;
  }

  /** Return the height of a specific row. */
  function getRowHeight(row: number): number {
    return rowHeights.get(row) ?? DEFAULT_ROW_HEIGHT;
  }

  /** Return the x-offset (in content coordinates) of the left edge of `col`.
   *  Hidden columns are skipped (zero width), so they don't occupy space. */
  function getColX(col: number): number {
    // Start with all cols at default width, then adjust for custom widths
    // and subtract hidden columns.
    let x = col * DEFAULT_COL_WIDTH;
    colWidths.forEach((w, c) => {
      if (c < col) {
        x += w - DEFAULT_COL_WIDTH;
      }
    });
    // Subtract the width of hidden columns before this column.
    hiddenCols.forEach((c) => {
      if (c < col) {
        x -= colWidths.get(c) ?? DEFAULT_COL_WIDTH;
      }
    });
    return x;
  }

  /** Return the y-offset (in content coordinates) of the top edge of `row`.
   *  Hidden rows are skipped (zero height), so they don't occupy space. */
  function getRowY(row: number): number {
    let y = row * DEFAULT_ROW_HEIGHT;
    rowHeights.forEach((h, r) => {
      if (r < row) {
        y += h - DEFAULT_ROW_HEIGHT;
      }
    });
    // Subtract the height of hidden rows before this row.
    hiddenRows.forEach((r) => {
      if (r < row) {
        y -= rowHeights.get(r) ?? DEFAULT_ROW_HEIGHT;
      }
    });
    return y;
  }

  // -----------------------------------------------------------------------
  // Viewport helpers
  // -----------------------------------------------------------------------

  const totalContentWidth = () => ROW_NUMBER_WIDTH + getColX(TOTAL_COLS);
  const totalContentHeight = () => HEADER_HEIGHT + getRowY(TOTAL_ROWS);

  // Buffer: render extra rows/cols beyond viewport for smooth scrolling.
  const BUFFER_COLS = 4;
  const BUFFER_ROWS = 8;

  /** Find the first visible column by binary-searching getColX. */
  const firstVisibleCol = () => {
    const sx = scrollX();
    // Quick estimate with default widths, then adjust
    let col = Math.floor(sx / DEFAULT_COL_WIDTH);
    // Adjust if custom widths shift things
    while (col > 0 && getColX(col) > sx) col--;
    while (col < TOTAL_COLS - 1 && getColX(col + 1) <= sx) col++;
    return Math.max(0, col - BUFFER_COLS);
  };

  /** Find the first visible row by scanning getRowY. */
  const firstVisibleRow = () => {
    const sy = scrollY();
    let row = Math.floor(sy / DEFAULT_ROW_HEIGHT);
    while (row > 0 && getRowY(row) > sy) row--;
    while (row < TOTAL_ROWS - 1 && getRowY(row + 1) <= sy) row++;
    return Math.max(0, row - BUFFER_ROWS);
  };

  const visibleColCount = () => {
    const sx = scrollX();
    const viewW = canvasWidth() - ROW_NUMBER_WIDTH;
    const start = firstVisibleCol();
    let count = 0;
    let x = getColX(start) - sx;
    while (start + count < TOTAL_COLS && x < viewW + sx) {
      x += getColWidth(start + count);
      count++;
      // Include buffer beyond viewport
      if (x >= viewW && count > BUFFER_COLS * 2) break;
    }
    return Math.min(count + BUFFER_COLS, TOTAL_COLS - start);
  };

  const visibleRowCount = () => {
    const sy = scrollY();
    const viewH = canvasHeight() - HEADER_HEIGHT;
    const start = firstVisibleRow();
    let count = 0;
    let y = getRowY(start) - sy;
    while (start + count < TOTAL_ROWS && y < viewH + sy) {
      y += getRowHeight(start + count);
      count++;
      if (y >= viewH && count > BUFFER_ROWS * 2) break;
    }
    return Math.min(count + BUFFER_ROWS, TOTAL_ROWS - start);
  };

  // -----------------------------------------------------------------------
  // Data fetching
  // -----------------------------------------------------------------------

  async function fetchVisibleData() {
    const startRow = firstVisibleRow();
    const startCol = firstVisibleCol();
    const endRow = startRow + visibleRowCount() - 1;
    const endCol = startCol + visibleColCount() - 1;

    const fetchKey = `${props.activeSheet}:${startRow}:${startCol}:${endRow}:${endCol}`;
    if (fetchKey === lastFetchKey) return;
    lastFetchKey = fetchKey;

    try {
      const data = await getRange(props.activeSheet, startRow, startCol, endRow, endCol);
      for (let r = 0; r < data.length; r++) {
        for (let c = 0; c < data[r].length; c++) {
          const cell = data[r][c];
          const key = `${startRow + r}:${startCol + c}`;
          if (cell) {
            cellCache.set(key, cell);
          } else {
            cellCache.delete(key);
          }
        }
      }
      draw();
    } catch {
      // Tauri not available (browser dev mode) -- draw without data.
    }
  }

  /** Load persisted column widths and row heights from the backend. */
  async function loadPersistedSizes() {
    try {
      const widths = await getColWidths(props.activeSheet);
      colWidths.clear();
      for (const [col, w] of Object.entries(widths)) {
        const c = Number(col);
        if (w !== DEFAULT_COL_WIDTH) {
          colWidths.set(c, w);
        }
      }
    } catch {
      // Backend may not support this yet
    }
    try {
      const heights = await getRowHeights(props.activeSheet);
      rowHeights.clear();
      for (const [row, h] of Object.entries(heights)) {
        const r = Number(row);
        if (h !== DEFAULT_ROW_HEIGHT) {
          rowHeights.set(r, h);
        }
      }
    } catch {
      // Backend may not support this yet
    }
  }

  /** Load hidden rows from the backend (auto-filter and manual hide). */
  async function fetchHiddenRows() {
    try {
      const rows = await getHiddenRows(props.activeSheet);
      hiddenRows.clear();
      for (const r of rows) {
        hiddenRows.add(r);
      }
    } catch {
      // Backend may not support this yet
    }
  }

  /** Load hidden cols from the backend. */
  async function fetchHiddenCols() {
    try {
      const cols = await getHiddenCols(props.activeSheet);
      hiddenCols.clear();
      for (const c of cols) {
        hiddenCols.add(c);
      }
    } catch {
      // Backend may not support this yet
    }
  }

  /** Load validation rules for the active sheet. */
  async function fetchValidations() {
    try {
      const rules = await listValidations(props.activeSheet);
      validationCache.clear();
      for (const [r, c, data] of rules) {
        validationCache.set(`${r}:${c}`, data);
      }
    } catch {
      // Backend may not support this yet
    }
  }

  /** Load conditional format rules for the active sheet. */
  async function fetchConditionalFormats() {
    try {
      conditionalFormatRules = await listConditionalFormats(props.activeSheet);
    } catch {
      conditionalFormatRules = [];
    }
  }

  /** Load row groups for the active sheet. */
  async function fetchRowGroups() {
    try {
      rowGroups = await getRowGroups(props.activeSheet);
    } catch {
      rowGroups = [];
    }
  }

  // -----------------------------------------------------------------------
  // Selection change with formula bar sync
  // -----------------------------------------------------------------------

  /** Propagate the current selection range to the parent without re-fetching cell data. */
  function propagateSelectionRange() {
    const range = getSelectionRange();
    props.onSelectionChange(selectedRow(), selectedCol(), range.minRow, range.minCol, range.maxRow, range.maxCol);
  }

  function selectCell(row: number, col: number) {
    // Pass both the active cell and the full selection range to the parent.
    const range = getSelectionRange();
    props.onSelectionChange(row, col, range.minRow, range.minCol, range.maxRow, range.maxCol);
    // Fetch cell data for formula bar display
    getCell(props.activeSheet, row, col)
      .then((cell) => {
        const content = cell?.formula ? `=${cell.formula}` : cell?.value ?? '';
        props.onContentChange(content);
      })
      .catch(() => {
        props.onContentChange('');
      });
  }

  // -----------------------------------------------------------------------
  // Editing
  // -----------------------------------------------------------------------

  function startEditing(clearContent: boolean, clickX?: number, clickY?: number) {
    // Clear marching ants when editing starts
    stopMarchingAnts();
    // Reset formula ref tracking
    formulaRefInsertStart = -1;
    formulaRefInsertEnd = -1;
    formulaRefAnchor = null;
    formulaRefDragging = false;

    const row = selectedRow();
    const col = selectedCol();
    const cell = cellCache.get(`${row}:${col}`);
    const content = clearContent ? '' : (cell?.formula ? `=${cell.formula}` : cell?.value ?? '');
    setEditValue(content);
    setEditing(true);
    props.onModeChange('Edit');
    props.onContentChange(content);

    // Populate auto-complete suggestions from the column
    const suggestions = getColumnSuggestions(cellCache, col);
    setAcSuggestions(suggestions);
    setAcVisible(false); // don't show until user types
    setAcSelectedIdx(0);

    requestAnimationFrame(() => {
      if (editorRef) {
        editorRef.focus();
        if (!clearContent) {
          // If we have a click position (double-click), place cursor at click location
          if (clickX !== undefined && clickY !== undefined && content.length > 0) {
            const cursorPos = computeCursorPosFromClick(editorRef, content, clickX, clickY);
            editorRef.setSelectionRange(cursorPos, cursorPos);
          } else {
            editorRef.setSelectionRange(content.length, content.length);
          }
        }
        autoResizeEditor();
      }
    });
  }

  /** Compute the cursor position in text based on a click's screen coordinates. */
  function computeCursorPosFromClick(
    editor: HTMLTextAreaElement, text: string, clientX: number, _clientY: number,
  ): number {
    const rect = editor.getBoundingClientRect();
    // Relative X within the editor, accounting for padding
    const editorPadding = 4; // matches CSS padding
    const relX = clientX - rect.left - editorPadding;
    if (relX <= 0) return 0;

    // Use a temporary canvas to measure character widths
    const measureCanvas = document.createElement('canvas');
    const measureCtx = measureCanvas.getContext('2d');
    if (!measureCtx) return text.length;

    const style = window.getComputedStyle(editor);
    measureCtx.font = `${style.fontStyle} ${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;

    // Find character index where cumulative width exceeds relX
    for (let i = 0; i <= text.length; i++) {
      const w = measureCtx.measureText(text.slice(0, i)).width;
      if (w >= relX) {
        // Check if we're closer to character i or i-1
        if (i > 0) {
          const prevW = measureCtx.measureText(text.slice(0, i - 1)).width;
          return (relX - prevW < w - relX) ? i - 1 : i;
        }
        return i;
      }
    }
    return text.length;
  }

  async function commitEdit(moveRow: number, moveCol: number) {
    const row = selectedRow();
    const col = selectedCol();
    const value = editValue();

    setEditing(false);
    setAcVisible(false);
    setFormulaAcVisible(false);
    props.onModeChange('Ready');

    // Write to backend
    let formula: string | undefined;
    if (value.startsWith('=')) {
      formula = value.slice(1);
    }
    try {
      await setCell(props.activeSheet, row, col, value, formula);
    } catch {
      // Tauri not available in browser dev mode.
    }

    props.onCellCommit(row, col, value);
    props.onContentChange(value);

    // Invalidate cache and refetch
    lastFetchKey = '';
    fetchVisibleData();

    // Move selection
    const newRow = Math.max(0, Math.min(TOTAL_ROWS - 1, row + moveRow));
    const newCol = Math.max(0, Math.min(TOTAL_COLS - 1, col + moveCol));
    setSelectedRow(newRow);
    setSelectedCol(newCol);
    setRangeAnchor(null);
    setRangeEnd(null);
    selectCell(newRow, newCol);
    ensureCellVisible(newRow, newCol);
    draw();

    // Refocus the container so keyboard works again
    containerRef?.focus();
  }

  function cancelEdit() {
    setEditing(false);
    setAcVisible(false);
    setFormulaAcVisible(false);
    props.onModeChange('Ready');
    containerRef?.focus();
    draw();
  }

  /** Cmd+Enter: fill ALL cells in the current selection with the edited value. */
  async function commitEditFillSelection() {
    const value = editValue();
    setEditing(false);
    setAcVisible(false);
    setFormulaAcVisible(false);
    props.onModeChange('Ready');

    let formula: string | undefined;
    if (value.startsWith('=')) {
      formula = value.slice(1);
    }

    const range = getSelectionRange();
    const promises: Promise<void>[] = [];
    for (let r = range.minRow; r <= range.maxRow; r++) {
      for (let c = range.minCol; c <= range.maxCol; c++) {
        promises.push(
          setCell(props.activeSheet, r, c, value, formula).catch(() => {}),
        );
      }
    }
    await Promise.all(promises);

    props.onContentChange(value);
    lastFetchKey = '';
    fetchVisibleData();
    props.onStatusChange('Filled selection');
    containerRef?.focus();
  }

  function handleEditorInput(value: string) {
    setEditValue(value);
    props.onContentChange(value);
    autoResizeEditor();
    // Reset formula ref insertion tracking when user types manually
    formulaRefInsertStart = -1;
    formulaRefInsertEnd = -1;
    // Track cursor position for formula argument hints
    if (editorRef) {
      setEditorCursorPos(editorRef.selectionStart ?? value.length);
    }

    // Show/hide auto-complete based on input
    const trimmed = value.trim();
    if (trimmed.startsWith('=')) {
      // Formula mode: show formula function suggestions
      setAcVisible(false);
      const token = extractCurrentToken(trimmed);
      if (token.length > 0) {
        const matches = filterFormulaFunctions(token);
        setFormulaAcVisible(matches.length > 0);
        setFormulaAcSelectedIdx(0);
      } else {
        setFormulaAcVisible(false);
      }
    } else if (trimmed.length > 0) {
      // Non-formula mode: show cell value suggestions
      setFormulaAcVisible(false);
      const lower = trimmed.toLowerCase();
      const matches = acSuggestions().filter((s) => {
        const sl = s.toLowerCase();
        return sl.startsWith(lower) && sl !== lower;
      });
      setAcVisible(matches.length > 0);
      setAcSelectedIdx(0);
    } else {
      setAcVisible(false);
      setFormulaAcVisible(false);
    }
  }

  /** Compute the filtered auto-complete list (same logic as AutoComplete). */
  function acFiltered(): string[] {
    const input = editValue().toLowerCase().trim();
    if (!input) return [];
    return acSuggestions().filter((s) => {
      const lower = s.toLowerCase();
      return lower.startsWith(input) && lower !== input;
    });
  }

  /** Get filtered formula suggestions for current input. */
  function formulaAcFiltered() {
    const token = extractCurrentToken(editValue());
    return filterFormulaFunctions(token);
  }

  /**
   * F4: Toggle the cell reference nearest to the cursor through
   * absolute/relative modes: A1 -> $A$1 -> A$1 -> $A1 -> A1
   */
  function handleF4Toggle() {
    if (!editorRef) return;
    const text = editValue();
    const cursorPos = editorRef.selectionStart ?? 0;

    // Regex to match cell references (with optional $ before col and/or row)
    const refPattern = /\$?[A-Za-z]{1,3}\$?\d+/g;
    let match: RegExpExecArray | null;
    let bestMatch: RegExpExecArray | null = null;
    let bestDist = Infinity;

    // Find the reference closest to the cursor
    while ((match = refPattern.exec(text)) !== null) {
      const start = match.index;
      const end = start + match[0].length;
      // Check if cursor is inside or adjacent to this reference
      if (cursorPos >= start && cursorPos <= end) {
        bestMatch = match;
        bestDist = 0;
        break;
      }
      // Otherwise find the closest one
      const dist = Math.min(Math.abs(cursorPos - start), Math.abs(cursorPos - end));
      if (dist < bestDist) {
        bestDist = dist;
        bestMatch = match;
      }
    }

    // Only toggle if cursor is inside or immediately adjacent (dist <= 1)
    if (!bestMatch || bestDist > 1) return;

    const ref = bestMatch[0];
    const refStart = bestMatch.index;
    const refEnd = refStart + ref.length;

    // Parse the reference into components
    const refParts = /^(\$?)([A-Za-z]{1,3})(\$?)(\d+)$/.exec(ref);
    if (!refParts) return;

    const colDollar = refParts[1] === '$';
    const colLetters = refParts[2];
    const rowDollar = refParts[3] === '$';
    const rowNum = refParts[4];

    // Cycle: A1 -> $A$1 -> A$1 -> $A1 -> A1
    let newRef: string;
    if (!colDollar && !rowDollar) {
      // A1 -> $A$1
      newRef = `$${colLetters}$${rowNum}`;
    } else if (colDollar && rowDollar) {
      // $A$1 -> A$1
      newRef = `${colLetters}$${rowNum}`;
    } else if (!colDollar && rowDollar) {
      // A$1 -> $A1
      newRef = `$${colLetters}${rowNum}`;
    } else {
      // $A1 -> A1
      newRef = `${colLetters}${rowNum}`;
    }

    const newText = text.slice(0, refStart) + newRef + text.slice(refEnd);
    setEditValue(newText);
    props.onContentChange(newText);

    // Position cursor at the end of the new reference
    const newCursorPos = refStart + newRef.length;
    requestAnimationFrame(() => {
      if (editorRef) {
        editorRef.setSelectionRange(newCursorPos, newCursorPos);
        setEditorCursorPos(newCursorPos);
      }
    });
  }

  function handleEditorKeyDown(e: KeyboardEvent) {
    // F4: Toggle absolute/relative reference
    if (e.key === 'F4') {
      e.preventDefault();
      handleF4Toggle();
      return;
    }

    // When formula auto-complete is visible, handle navigation keys
    if (formulaAcVisible()) {
      const list = formulaAcFiltered();
      if (list.length > 0) {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          setFormulaAcSelectedIdx((i) => Math.min(i + 1, list.length - 1));
          return;
        }
        if (e.key === 'ArrowUp') {
          e.preventDefault();
          setFormulaAcSelectedIdx((i) => Math.max(i - 1, 0));
          return;
        }
        if (e.key === 'Tab') {
          const idx = formulaAcSelectedIdx();
          if (idx >= 0 && idx < list.length) {
            e.preventDefault();
            acceptFormulaAutoComplete(list[idx].name);
            return;
          }
        }
        if (e.key === 'Escape') {
          e.preventDefault();
          setFormulaAcVisible(false);
          return;
        }
      }
    }

    // When cell value auto-complete is visible, handle navigation keys
    if (acVisible()) {
      const list = acFiltered();
      if (list.length > 0) {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          setAcSelectedIdx((i) => Math.min(i + 1, list.length - 1));
          return;
        }
        if (e.key === 'ArrowUp') {
          e.preventDefault();
          setAcSelectedIdx((i) => Math.max(i - 1, 0));
          return;
        }
        if (e.key === 'Tab') {
          const idx = acSelectedIdx();
          if (idx >= 0 && idx < list.length) {
            e.preventDefault();
            acceptAutoComplete(list[idx]);
            return;
          }
        }
        if (e.key === 'Escape') {
          e.preventDefault();
          setAcVisible(false);
          return;
        }
      }
    }

    // Arrow keys: always commit and move to the next cell (matches Google Sheets).
    // The formula bar keeps its own cursor-at-edge behaviour; the cell editor
    // always commits immediately.
    if (e.key === 'ArrowUp' || e.key === 'ArrowDown' || e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      e.preventDefault();
      if (e.key === 'ArrowLeft') {
        commitEdit(0, -1);
      } else if (e.key === 'ArrowRight') {
        commitEdit(0, 1);
      } else if (e.key === 'ArrowUp') {
        commitEdit(-1, 0);
      } else {
        commitEdit(1, 0);
      }
      return;
    }

    // Alt+Enter (Option+Enter): insert newline in cell and enable text wrap
    if (e.key === 'Enter' && e.altKey) {
      e.preventDefault();
      if (editorRef) {
        const pos = editorRef.selectionStart ?? editValue().length;
        const val = editValue();
        const newVal = val.slice(0, pos) + '\n' + val.slice(pos);
        setEditValue(newVal);
        props.onContentChange(newVal);
        // Enable text wrap for this cell
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { text_wrap: 'Wrap' })
          .catch(() => {});
        // Restore cursor position after the inserted newline
        requestAnimationFrame(() => {
          if (editorRef) {
            editorRef.setSelectionRange(pos + 1, pos + 1);
          }
        });
      }
      return;
    }

    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      // Cmd+Enter: fill ALL cells in the current selection with edited value
      e.preventDefault();
      void commitEditFillSelection();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      commitEdit(e.shiftKey ? -1 : 1, 0);
    } else if (e.key === 'Tab') {
      e.preventDefault();
      commitEdit(0, e.shiftKey ? -1 : 1);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      cancelEdit();
    }
  }

  /** Accept a formula function suggestion: replace current token with function name + open paren. */
  function acceptFormulaAutoComplete(funcName: string) {
    const current = editValue();
    const token = extractCurrentToken(current);
    // Replace the last N chars (the token) with the function name + (
    const prefix = current.slice(0, current.length - token.length);
    const newValue = prefix + funcName + '(';
    setEditValue(newValue);
    props.onContentChange(newValue);
    setFormulaAcVisible(false);
    editorRef?.focus();
    // Position cursor at the end
    requestAnimationFrame(() => {
      if (editorRef) {
        editorRef.setSelectionRange(newValue.length, newValue.length);
      }
    });
  }

  function acceptAutoComplete(value: string) {
    setEditValue(value);
    props.onContentChange(value);
    setAcVisible(false);
    // Keep the editor focused and let user continue editing or commit
    editorRef?.focus();
  }

  /** Calculate editor position in CSS pixels relative to the container. */
  function editorStyle() {
    const col = selectedCol();
    const row = selectedRow();
    const fc = props.frozenCols ?? 0;
    const fr = props.frozenRows ?? 0;
    const sc = props.splitCol ?? 0;
    const sr = props.splitRow ?? 0;

    let sx: number;
    if (col < fc) sx = 0;
    else if (sc > 0 && col < sc) sx = splitScrollX();
    else sx = scrollX();

    let sy: number;
    if (row < fr) sy = 0;
    else if (sr > 0 && row < sr) sy = splitScrollY();
    else sy = scrollY();

    const x = ROW_NUMBER_WIDTH + getColX(col) - sx;
    const y = HEADER_HEIGHT + getRowY(row) - sy;
    const minH = getRowHeight(row);
    // Match canvas cell's font for visual consistency
    const cell = cellCache.get(`${row}:${col}`);
    const fontSize = cell?.font_size || 11;
    const fontFamily = cell?.font_family || '-apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';
    const fontWeight = cell?.bold ? 'bold' : 'normal';
    const fontStyleCss = cell?.italic ? 'italic' : 'normal';
    // Match cell alignment: auto-align numbers right when h_align is default "left"
    const isNumber = cell?.value ? (!isNaN(Number(cell.value)) && cell.value.trim() !== '') : false;
    const userSetAlign = cell?.h_align && cell.h_align !== 'left';
    const textAlign = userSetAlign ? cell!.h_align : (isNumber ? 'right' : 'left');
    return {
      position: 'absolute' as const,
      left: `${x}px`,
      top: `${y}px`,
      width: `${getColWidth(col)}px`,
      'min-height': `${minH}px`,
      'z-index': '10',
      'font-size': `${fontSize}px`,
      'font-family': fontFamily,
      'font-weight': fontWeight,
      'font-style': fontStyleCss,
      'text-align': textAlign as 'left' | 'right' | 'center',
      color: cell?.font_color || undefined,
    };
  }

  /** Auto-resize the editor textarea to fit its content. */
  function autoResizeEditor() {
    if (!editorRef) return;
    editorRef.style.height = 'auto';
    editorRef.style.height = `${Math.max(editorRef.scrollHeight, getRowHeight(selectedRow()))}px`;
  }

  // -----------------------------------------------------------------------
  // Drawing
  // -----------------------------------------------------------------------

  /** Width of the frozen columns region in pixels. */
  function frozenColsPx(): number {
    const fc = props.frozenCols ?? 0;
    if (fc <= 0) return 0;
    return getColX(fc);
  }

  /** Height of the frozen rows region in pixels. */
  function frozenRowsPx(): number {
    const fr = props.frozenRows ?? 0;
    if (fr <= 0) return 0;
    return getRowY(fr);
  }

  /** Whether split panes are active. */
  function isSplit(): boolean {
    return (props.splitRow ?? 0) > 0 || (props.splitCol ?? 0) > 0;
  }

  /** Width of the left split pane region in pixels. */
  function splitColsPx(): number {
    const sc = props.splitCol ?? 0;
    if (sc <= 0) return 0;
    return getColX(sc);
  }

  /** Height of the top split pane region in pixels. */
  function splitRowsPx(): number {
    const sr = props.splitRow ?? 0;
    if (sr <= 0) return 0;
    return getRowY(sr);
  }

  /** Find the first visible column for a given scroll offset. */
  function firstVisibleColAt(sx: number): number {
    let col = Math.floor(sx / DEFAULT_COL_WIDTH);
    while (col > 0 && getColX(col) > sx) col--;
    while (col < TOTAL_COLS - 1 && getColX(col + 1) <= sx) col++;
    return Math.max(0, col - BUFFER_COLS);
  }

  /** Find the first visible row for a given scroll offset. */
  function firstVisibleRowAt(sy: number): number {
    let row = Math.floor(sy / DEFAULT_ROW_HEIGHT);
    while (row > 0 && getRowY(row) > sy) row--;
    while (row < TOTAL_ROWS - 1 && getRowY(row + 1) <= sy) row++;
    return Math.max(0, row - BUFFER_ROWS);
  }

  /** Count visible columns for a given scroll offset and viewport width. */
  function visibleColCountAt(sx: number, viewW: number): number {
    const start = firstVisibleColAt(sx);
    let count = 0;
    let x = getColX(start) - sx;
    while (start + count < TOTAL_COLS && x < viewW + sx) {
      x += getColWidth(start + count);
      count++;
      if (x >= viewW && count > BUFFER_COLS * 2) break;
    }
    return Math.min(count + BUFFER_COLS, TOTAL_COLS - start);
  }

  /** Count visible rows for a given scroll offset and viewport height. */
  function visibleRowCountAt(sy: number, viewH: number): number {
    const start = firstVisibleRowAt(sy);
    let count = 0;
    let y = getRowY(start) - sy;
    while (start + count < TOTAL_ROWS && y < viewH + sy) {
      y += getRowHeight(start + count);
      count++;
      if (y >= viewH && count > BUFFER_ROWS * 2) break;
    }
    return Math.min(count + BUFFER_ROWS, TOTAL_ROWS - start);
  }

  /** Render a single pane region: grid lines, selection, cell data. */
  function drawPane(
    ctx: CanvasRenderingContext2D,
    clipX: number,
    clipY: number,
    clipW: number,
    clipH: number,
    sx: number,
    sy: number,
    startCol: number,
    startRow: number,
    colCount: number,
    rowCount: number,
  ) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(clipX, clipY, clipW, clipH);
    ctx.clip();
    drawGridLines(ctx, sx, sy, startCol, startRow, colCount, rowCount, clipX + clipW, clipY + clipH);
    drawSelection(ctx, sx, sy);
    drawCellData(ctx, sx, sy, startCol, startRow, colCount, rowCount);
    ctx.restore();
  }

  function draw() {
    const canvas = canvasRef;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const zoomLevel = props.zoom ?? 1.0;
    const w = canvasWidth();
    const h = canvasHeight();

    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr * zoomLevel, dpr * zoomLevel);
    ctx.clearRect(0, 0, w / zoomLevel, h / zoomLevel);

    // Fill background with theme-appropriate color for dark mode support.
    ctx.fillStyle = COLORS.cellBg;
    ctx.fillRect(0, 0, w / zoomLevel, h / zoomLevel);

    const sx = scrollX();
    const sy = scrollY();
    const fc = props.frozenCols ?? 0;
    const fr = props.frozenRows ?? 0;
    const fpx = frozenColsPx();
    const fpy = frozenRowsPx();

    if (isSplit() && fc <= 0 && fr <= 0) {
      // Split panes: each quadrant scrolls independently.
      const sc = props.splitCol ?? 0;
      const sr = props.splitRow ?? 0;
      const spx = splitColsPx();
      const spy = splitRowsPx();
      const ssx = splitScrollX();
      const ssy = splitScrollY();

      // Bottom-right pane (main scroll)
      {
        const paneW = sc > 0 ? w - ROW_NUMBER_WIDTH - spx : w - ROW_NUMBER_WIDTH;
        const paneH = sr > 0 ? h - HEADER_HEIGHT - spy : h - HEADER_HEIGHT;
        const paneLeft = ROW_NUMBER_WIDTH + (sc > 0 ? spx : 0);
        const paneTop = HEADER_HEIGHT + (sr > 0 ? spy : 0);
        const startCol = firstVisibleColAt(sx);
        const startRow = firstVisibleRowAt(sy);
        const colCount = visibleColCountAt(sx, paneW);
        const rowCount = visibleRowCountAt(sy, paneH);
        drawPane(ctx, paneLeft, paneTop, paneW, paneH, sx, sy, startCol, startRow, colCount, rowCount);
        drawColumnHeaders(ctx, sx, startCol, colCount, w);
        drawRowNumbers(ctx, sy, startRow, rowCount, h);
      }

      // Top pane (horizontal split) — uses splitScrollY
      if (sr > 0) {
        const paneW = sc > 0 ? w - ROW_NUMBER_WIDTH - spx : w - ROW_NUMBER_WIDTH;
        const paneH = spy;
        const paneLeft = ROW_NUMBER_WIDTH + (sc > 0 ? spx : 0);
        const startCol = firstVisibleColAt(sx);
        const startRow = firstVisibleRowAt(ssy);
        const colCount = visibleColCountAt(sx, paneW);
        const rowCount = visibleRowCountAt(ssy, paneH);
        drawPane(ctx, paneLeft, HEADER_HEIGHT, paneW, paneH, sx, ssy, startCol, startRow, colCount, rowCount);
        // Row numbers for top pane
        ctx.save();
        ctx.beginPath();
        ctx.rect(0, HEADER_HEIGHT, ROW_NUMBER_WIDTH, paneH);
        ctx.clip();
        drawRowNumbers(ctx, ssy, startRow, rowCount, HEADER_HEIGHT + paneH);
        ctx.restore();
      }

      // Left pane (vertical split) — uses splitScrollX
      if (sc > 0) {
        const paneW = spx;
        const paneH = sr > 0 ? h - HEADER_HEIGHT - spy : h - HEADER_HEIGHT;
        const paneTop = HEADER_HEIGHT + (sr > 0 ? spy : 0);
        const startCol = firstVisibleColAt(ssx);
        const startRow = firstVisibleRowAt(sy);
        const colCount = visibleColCountAt(ssx, paneW);
        const rowCount = visibleRowCountAt(sy, paneH);
        drawPane(ctx, ROW_NUMBER_WIDTH, paneTop, paneW, paneH, ssx, sy, startCol, startRow, colCount, rowCount);
        // Column headers for left pane
        ctx.save();
        ctx.beginPath();
        ctx.rect(ROW_NUMBER_WIDTH, 0, paneW, HEADER_HEIGHT);
        ctx.clip();
        drawColumnHeaders(ctx, ssx, startCol, colCount, ROW_NUMBER_WIDTH + paneW);
        ctx.restore();
      }

      // Top-left pane (both split) — uses splitScrollX + splitScrollY
      if (sc > 0 && sr > 0) {
        const startCol = firstVisibleColAt(ssx);
        const startRow = firstVisibleRowAt(ssy);
        const colCount = visibleColCountAt(ssx, spx);
        const rowCount = visibleRowCountAt(ssy, spy);
        drawPane(ctx, ROW_NUMBER_WIDTH, HEADER_HEIGHT, spx, spy, ssx, ssy, startCol, startRow, colCount, rowCount);
      }

      drawCorner(ctx);

      // Draw split divider lines (thicker, distinct from freeze)
      ctx.strokeStyle = COLORS.freezeBorder;
      ctx.lineWidth = 3;
      if (sc > 0) {
        const dx = ROW_NUMBER_WIDTH + spx;
        ctx.beginPath();
        ctx.moveTo(dx, 0);
        ctx.lineTo(dx, h);
        ctx.stroke();
      }
      if (sr > 0) {
        const dy = HEADER_HEIGHT + spy;
        ctx.beginPath();
        ctx.moveTo(0, dy);
        ctx.lineTo(w, dy);
        ctx.stroke();
      }
      ctx.lineWidth = 1;
    } else if (fc <= 0 && fr <= 0) {
      // No freeze panes: simple single-pass render.
      const startCol = firstVisibleCol();
      const startRow = firstVisibleRow();
      const colCount = visibleColCount();
      const rowCount = visibleRowCount();

      drawGridLines(ctx, sx, sy, startCol, startRow, colCount, rowCount, w, h);
      drawSelection(ctx, sx, sy);
      drawCellData(ctx, sx, sy, startCol, startRow, colCount, rowCount);
      drawColumnHeaders(ctx, sx, startCol, colCount, w);
      drawRowNumbers(ctx, sy, startRow, rowCount, h);
      drawCorner(ctx);
    } else {
      // Freeze panes: render 4 quadrants with clipping.
      const startCol = firstVisibleCol();
      const startRow = firstVisibleRow();
      const colCount = visibleColCount();
      const rowCount = visibleRowCount();

      // Q4: Bottom-right (scrollable rows + scrollable cols) — main area
      ctx.save();
      ctx.beginPath();
      ctx.rect(ROW_NUMBER_WIDTH + fpx, HEADER_HEIGHT + fpy, w - ROW_NUMBER_WIDTH - fpx, h - HEADER_HEIGHT - fpy);
      ctx.clip();
      drawGridLines(ctx, sx, sy, startCol, startRow, colCount, rowCount, w, h);
      drawSelection(ctx, sx, sy);
      drawCellData(ctx, sx, sy, startCol, startRow, colCount, rowCount);
      ctx.restore();

      // Q2: Top-right (frozen rows, scrollable cols) — scrolls horizontally
      if (fr > 0) {
        ctx.save();
        ctx.beginPath();
        ctx.rect(ROW_NUMBER_WIDTH + fpx, HEADER_HEIGHT, w - ROW_NUMBER_WIDTH - fpx, fpy);
        ctx.clip();
        drawGridLines(ctx, sx, 0, startCol, 0, colCount, fr, w, HEADER_HEIGHT + fpy);
        drawSelection(ctx, sx, 0);
        drawCellData(ctx, sx, 0, startCol, 0, colCount, fr);
        ctx.restore();
      }

      // Q3: Bottom-left (scrollable rows, frozen cols) — scrolls vertically
      if (fc > 0) {
        ctx.save();
        ctx.beginPath();
        ctx.rect(ROW_NUMBER_WIDTH, HEADER_HEIGHT + fpy, fpx, h - HEADER_HEIGHT - fpy);
        ctx.clip();
        drawGridLines(ctx, 0, sy, 0, startRow, fc, rowCount, ROW_NUMBER_WIDTH + fpx, h);
        drawSelection(ctx, 0, sy);
        drawCellData(ctx, 0, sy, 0, startRow, fc, rowCount);
        ctx.restore();
      }

      // Q1: Top-left (frozen rows + frozen cols) — always visible
      if (fc > 0 && fr > 0) {
        ctx.save();
        ctx.beginPath();
        ctx.rect(ROW_NUMBER_WIDTH, HEADER_HEIGHT, fpx, fpy);
        ctx.clip();
        drawGridLines(ctx, 0, 0, 0, 0, fc, fr, ROW_NUMBER_WIDTH + fpx, HEADER_HEIGHT + fpy);
        drawSelection(ctx, 0, 0);
        drawCellData(ctx, 0, 0, 0, 0, fc, fr);
        ctx.restore();
      }

      // Headers (drawn on top of quadrants)
      drawColumnHeaders(ctx, sx, startCol, colCount, w);
      // Frozen column headers (no scroll)
      if (fc > 0) {
        ctx.save();
        ctx.beginPath();
        ctx.rect(ROW_NUMBER_WIDTH, 0, fpx, HEADER_HEIGHT);
        ctx.clip();
        drawColumnHeaders(ctx, 0, 0, fc, ROW_NUMBER_WIDTH + fpx);
        ctx.restore();
      }

      drawRowNumbers(ctx, sy, startRow, rowCount, h);
      // Frozen row numbers (no scroll)
      if (fr > 0) {
        ctx.save();
        ctx.beginPath();
        ctx.rect(0, HEADER_HEIGHT, ROW_NUMBER_WIDTH, fpy);
        ctx.clip();
        drawRowNumbers(ctx, 0, 0, fr, HEADER_HEIGHT + fpy);
        ctx.restore();
      }

      drawCorner(ctx);

      // Draw freeze border lines.
      ctx.strokeStyle = COLORS.freezeBorder;
      ctx.lineWidth = 2;
      if (fc > 0) {
        const fx = ROW_NUMBER_WIDTH + fpx;
        ctx.beginPath();
        ctx.moveTo(fx, 0);
        ctx.lineTo(fx, h);
        ctx.stroke();
      }
      if (fr > 0) {
        const fy = HEADER_HEIGHT + fpy;
        ctx.beginPath();
        ctx.moveTo(0, fy);
        ctx.lineTo(w, fy);
        ctx.stroke();
      }
      ctx.lineWidth = 1;
    }

    // Draw page break indicators if enabled.
    drawPageBreaks(ctx, sx, sy, w / zoomLevel, h / zoomLevel);

    // Update status bar selection summary after each draw
    updateSelectionSummary();
  }

  // -----------------------------------------------------------------------
  // Selection helpers
  // -----------------------------------------------------------------------

  /** Check if a cell has content in the cache. */
  function cellHasContent(row: number, col: number): boolean {
    const cell = cellCache.get(`${row}:${col}`);
    return !!(cell && cell.value && cell.value.trim() !== '');
  }

  /**
   * Jump down from (row, col): if current cell is non-empty, go to the last
   * non-empty cell in a contiguous run below. If current cell is empty, go to
   * the next non-empty cell below. If nothing found, go to the bottom.
   */
  function jumpDown(row: number, col: number): number {
    const hasContent = cellHasContent(row, col);
    if (hasContent) {
      // Walk down through contiguous non-empty cells
      let r = row + 1;
      while (r < TOTAL_ROWS && cellHasContent(r, col)) r++;
      return Math.min(r - 1, TOTAL_ROWS - 1); // last non-empty, or same row if isolated
    }
    // Walk down to the next non-empty cell
    let r = row + 1;
    while (r < TOTAL_ROWS && !cellHasContent(r, col)) r++;
    return r < TOTAL_ROWS ? r : TOTAL_ROWS - 1;
  }

  /** Jump up: mirror of jumpDown. */
  function jumpUp(row: number, col: number): number {
    const hasContent = cellHasContent(row, col);
    if (hasContent) {
      let r = row - 1;
      while (r >= 0 && cellHasContent(r, col)) r--;
      return Math.max(r + 1, 0);
    }
    let r = row - 1;
    while (r >= 0 && !cellHasContent(r, col)) r--;
    return r >= 0 ? r : 0;
  }

  /** Jump right: like jumpDown but along the row axis. */
  function jumpRight(row: number, col: number): number {
    const hasContent = cellHasContent(row, col);
    if (hasContent) {
      let c = col + 1;
      while (c < TOTAL_COLS && cellHasContent(row, c)) c++;
      return Math.min(c - 1, TOTAL_COLS - 1);
    }
    let c = col + 1;
    while (c < TOTAL_COLS && !cellHasContent(row, c)) c++;
    return c < TOTAL_COLS ? c : TOTAL_COLS - 1;
  }

  /** Jump left: mirror of jumpRight. */
  function jumpLeft(row: number, col: number): number {
    const hasContent = cellHasContent(row, col);
    if (hasContent) {
      let c = col - 1;
      while (c >= 0 && cellHasContent(row, c)) c--;
      return Math.max(c + 1, 0);
    }
    let c = col - 1;
    while (c >= 0 && !cellHasContent(row, c)) c--;
    return c >= 0 ? c : 0;
  }

  function getSelectionRange() {
    const anchor = rangeAnchor();
    const end = rangeEnd();
    if (anchor && end) {
      return {
        minRow: Math.min(anchor[0], end[0]),
        maxRow: Math.max(anchor[0], end[0]),
        minCol: Math.min(anchor[1], end[1]),
        maxCol: Math.max(anchor[1], end[1]),
      };
    }
    return {
      minRow: selectedRow(),
      maxRow: selectedRow(),
      minCol: selectedCol(),
      maxCol: selectedCol(),
    };
  }

  /** Compute selection summary (Sum, Average, Count) and notify parent. */
  function updateSelectionSummary() {
    if (!props.onSelectionSummary) return;
    const range = getSelectionRange();
    const isMulti = range.minRow !== range.maxRow || range.minCol !== range.maxCol;
    if (!isMulti) {
      props.onSelectionSummary('');
      return;
    }

    let sum = 0;
    let count = 0;
    let numericCount = 0;
    let minVal = Infinity;
    let maxVal = -Infinity;

    for (let r = range.minRow; r <= range.maxRow; r++) {
      for (let c = range.minCol; c <= range.maxCol; c++) {
        const cell = cellCache.get(`${r}:${c}`);
        if (cell && cell.value && cell.value.trim() !== '') {
          count++;
          const num = Number(cell.value);
          if (!isNaN(num)) {
            sum += num;
            numericCount++;
            if (num < minVal) minVal = num;
            if (num > maxVal) maxVal = num;
          }
        }
      }
    }

    if (numericCount > 0) {
      const avg = sum / numericCount;
      const avgStr = Number.isInteger(avg) ? String(avg) : avg.toFixed(4).replace(/0+$/, '').replace(/\.$/, '');
      props.onSelectionSummary(`Sum: ${sum}  Average: ${avgStr}  Min: ${minVal}  Max: ${maxVal}  Count: ${count}`);
    } else if (count > 0) {
      props.onSelectionSummary(`Count: ${count}`);
    } else {
      props.onSelectionSummary('');
    }
  }

  function isColInSelection(col: number): boolean {
    const { minCol, maxCol } = getSelectionRange();
    return col >= minCol && col <= maxCol;
  }

  function isRowInSelection(row: number): boolean {
    const { minRow, maxRow } = getSelectionRange();
    return row >= minRow && row <= maxRow;
  }

  // Fill handle constants
  const FILL_HANDLE_SIZE = 8;
  const FILL_HANDLE_HIT_SIZE = 16; // larger hit area for easier clicking

  function drawSelection(ctx: CanvasRenderingContext2D, sx: number, sy: number) {
    const range = getSelectionRange();

    const isMulti = range.minRow !== range.maxRow || range.minCol !== range.maxCol;

    // Draw fill drag preview if active
    if (isFillDragging) {
      const fillRange = getFillPreviewRange();
      if (fillRange) {
        const fx = ROW_NUMBER_WIDTH + getColX(fillRange.minCol) - sx;
        const fy = HEADER_HEIGHT + getRowY(fillRange.minRow) - sy;
        const fw = getColX(fillRange.maxCol + 1) - getColX(fillRange.minCol);
        const fh = getRowY(fillRange.maxRow + 1) - getRowY(fillRange.minRow);
        ctx.strokeStyle = COLORS.selectionBorder;
        ctx.setLineDash([4, 4]);
        ctx.lineWidth = 2;
        ctx.strokeRect(fx, fy, fw, fh);
        ctx.setLineDash([]);
        ctx.lineWidth = 1;
      }
    }

    // Draw range fill if multi-cell selection
    if (isMulti) {
      const rx = ROW_NUMBER_WIDTH + getColX(range.minCol) - sx;
      const ry = HEADER_HEIGHT + getRowY(range.minRow) - sy;
      const rw = getColX(range.maxCol + 1) - getColX(range.minCol);
      const rh = getRowY(range.maxRow + 1) - getRowY(range.minRow);
      ctx.fillStyle = COLORS.selectionBg;
      ctx.fillRect(rx, ry, rw, rh);

      // Draw border around entire range
      ctx.strokeStyle = COLORS.selectionBorder;
      ctx.lineWidth = 2;
      ctx.strokeRect(rx, ry, rw, rh);
      ctx.lineWidth = 1;
    }

    // Draw active cell border (2px blue) — white fill cutout in range mode
    const cx = ROW_NUMBER_WIDTH + getColX(selectedCol()) - sx;
    const cy = HEADER_HEIGHT + getRowY(selectedRow()) - sy;
    if (isMulti) {
      // Clear the selection background on the active cell (white cutout)
      ctx.fillStyle = COLORS.cellBg;
      ctx.fillRect(cx, cy, getColWidth(selectedCol()), getRowHeight(selectedRow()));
    }
    ctx.strokeStyle = COLORS.selectionBorder;
    ctx.lineWidth = 2;
    ctx.strokeRect(cx, cy, getColWidth(selectedCol()), getRowHeight(selectedRow()));
    ctx.lineWidth = 1;

    // Draw fill handle (small blue square at bottom-right of selection)
    if (!editing()) {
      const handleX = ROW_NUMBER_WIDTH + getColX(range.maxCol) + getColWidth(range.maxCol) - sx - FILL_HANDLE_SIZE / 2;
      const handleY = HEADER_HEIGHT + getRowY(range.maxRow) + getRowHeight(range.maxRow) - sy - FILL_HANDLE_SIZE / 2;
      ctx.fillStyle = COLORS.selectionBorder;
      ctx.fillRect(handleX - 1, handleY - 1, FILL_HANDLE_SIZE + 2, FILL_HANDLE_SIZE + 2);
      ctx.fillStyle = '#ffffff';
      ctx.fillRect(handleX, handleY, FILL_HANDLE_SIZE, FILL_HANDLE_SIZE);
      ctx.fillStyle = COLORS.selectionBorder;
      ctx.fillRect(handleX + 1, handleY + 1, FILL_HANDLE_SIZE - 2, FILL_HANDLE_SIZE - 2);
    }

    // Draw marching ants around copied range
    if (copiedRange) {
      const cr = copiedRange;
      const mx = ROW_NUMBER_WIDTH + getColX(cr.minCol) - sx;
      const my = HEADER_HEIGHT + getRowY(cr.minRow) - sy;
      const mw = getColX(cr.maxCol + 1) - getColX(cr.minCol);
      const mh = getRowY(cr.maxRow + 1) - getRowY(cr.minRow);
      ctx.strokeStyle = COLORS.selectionBorder;
      ctx.lineWidth = 2;
      ctx.setLineDash([5, 3]);
      ctx.lineDashOffset = -marchingAntOffset;
      ctx.strokeRect(mx, my, mw, mh);
      ctx.setLineDash([]);
      ctx.lineDashOffset = 0;
      ctx.lineWidth = 1;
    }

    // Draw colored borders around formula-referenced ranges
    if (isFormulaSelectionMode()) {
      const refs = parseFormulaRefs(editValue());
      for (const ref of refs) {
        const rx = ROW_NUMBER_WIDTH + getColX(ref.minCol) - sx;
        const ry = HEADER_HEIGHT + getRowY(ref.minRow) - sy;
        const rw = getColX(ref.maxCol + 1) - getColX(ref.minCol);
        const rh = getRowY(ref.maxRow + 1) - getRowY(ref.minRow);
        // Light fill
        ctx.fillStyle = ref.color + '18'; // ~10% opacity
        ctx.fillRect(rx, ry, rw, rh);
        // Border
        ctx.strokeStyle = ref.color;
        ctx.lineWidth = 2;
        ctx.strokeRect(rx, ry, rw, rh);
        ctx.lineWidth = 1;
      }
    }
  }

  /**
   * Load an image from a data URL into the image cache.
   * Returns the cached image if already loaded, or null while loading.
   */
  function getCachedImage(dataUrl: string): HTMLImageElement | null {
    const cached = imageCache.get(dataUrl);
    if (cached !== undefined) return cached;

    // Mark as loading
    imageCache.set(dataUrl, null);
    const img = new Image();
    img.onload = () => {
      imageCache.set(dataUrl, img);
      scheduleDraw(); // redraw once the image is loaded
    };
    img.onerror = () => {
      // Remove from cache so it can be retried
      imageCache.delete(dataUrl);
    };
    img.src = dataUrl;
    return null;
  }

  /** Find the merged region that a cell belongs to, if any. */
  function findMergedRegion(row: number, col: number): MergedRegionData | undefined {
    const regions = props.mergedRegions;
    if (!regions || regions.length === 0) return undefined;
    return regions.find(
      (r) => row >= r.start_row && row <= r.end_row && col >= r.start_col && col <= r.end_col,
    );
  }

  /** Compute the total pixel width of a merged region. */
  function mergedWidth(region: MergedRegionData): number {
    let w = 0;
    for (let c = region.start_col; c <= region.end_col; c++) {
      if (!hiddenCols.has(c)) w += getColWidth(c);
    }
    return w;
  }

  /** Compute the total pixel height of a merged region. */
  function mergedHeight(region: MergedRegionData): number {
    let h = 0;
    for (let r = region.start_row; r <= region.end_row; r++) {
      if (!hiddenRows.has(r)) h += getRowHeight(r);
    }
    return h;
  }

  /**
   * Evaluate conditional format rules for a cell and return the first
   * matching style overrides (bg_color, font_color, bold, italic).
   * Rules are evaluated in array order (priority order from backend).
   */
  function evaluateConditionalFormat(
    row: number,
    col: number,
    cellValue: string | undefined,
  ): { bg_color?: string; font_color?: string; bold?: boolean; italic?: boolean } | null {
    if (conditionalFormatRules.length === 0) return null;

    for (const cfRange of conditionalFormatRules) {
      if (row < cfRange.start_row || row > cfRange.end_row) continue;
      if (col < cfRange.start_col || col > cfRange.end_col) continue;

      for (const rule of cfRange.rules) {
        let matches = false;
        const kind = rule.kind;

        if (kind === 'cell_value') {
          const numVal = cellValue != null ? Number(cellValue) : NaN;
          if (!isNaN(numVal) && rule.operator != null && rule.value1 != null) {
            const v1 = rule.value1;
            const v2 = rule.value2;
            switch (rule.operator) {
              case '>': matches = numVal > v1; break;
              case '<': matches = numVal < v1; break;
              case '>=': matches = numVal >= v1; break;
              case '<=': matches = numVal <= v1; break;
              case '=': matches = Math.abs(numVal - v1) < Number.EPSILON; break;
              case '!=': matches = Math.abs(numVal - v1) >= Number.EPSILON; break;
              case 'between': matches = numVal >= v1 && numVal <= (v2 ?? v1); break;
              case 'not between': matches = numVal < v1 || numVal > (v2 ?? v1); break;
            }
          }
        } else if (kind === 'text_contains') {
          const text = cellValue ?? '';
          const needle = rule.text ?? '';
          matches = text.toLowerCase().includes(needle.toLowerCase());
        } else if (kind === 'is_blank') {
          matches = !cellValue || cellValue === '';
        } else if (kind === 'is_not_blank') {
          matches = cellValue != null && cellValue !== '';
        } else if (kind === 'is_error') {
          matches = cellValue != null && (
            cellValue.startsWith('#DIV/0') ||
            cellValue.startsWith('#REF') ||
            cellValue.startsWith('#NAME') ||
            cellValue.startsWith('#VALUE') ||
            cellValue.startsWith('#N/A') ||
            cellValue.startsWith('#NULL') ||
            cellValue.startsWith('#NUM')
          );
        }

        if (matches) {
          return {
            bg_color: rule.bg_color ?? undefined,
            font_color: rule.font_color ?? undefined,
            bold: rule.bold ?? undefined,
            italic: rule.italic ?? undefined,
          };
        }
      }
    }
    return null;
  }

  function drawCellData(
    ctx: CanvasRenderingContext2D,
    sx: number,
    sy: number,
    startCol: number,
    startRow: number,
    colCount: number,
    rowCount: number,
  ) {
    const PADDING = 4;
    ctx.textBaseline = 'middle';

    // Banded row config
    const banded = props.bandedRows;

    for (let r = 0; r < rowCount; r++) {
      const row = startRow + r;
      // Skip hidden rows (e.g. from auto-filter)
      if (hiddenRows.has(row)) continue;
      const rh = getRowHeight(row);
      const y = HEADER_HEIGHT + getRowY(row) - sy;
      for (let c = 0; c < colCount; c++) {
        const col = startCol + c;
        // Skip hidden columns
        if (hiddenCols.has(col)) continue;

        // Merged region handling: skip non-anchor cells
        const mergedRegion = findMergedRegion(row, col);
        if (mergedRegion && (row !== mergedRegion.start_row || col !== mergedRegion.start_col)) {
          continue; // This cell is covered by a merge; skip it
        }

        const cell = cellCache.get(`${row}:${col}`);
        let cw = getColWidth(col);
        let cellHeight = rh;
        const x = ROW_NUMBER_WIDTH + getColX(col) - sx;

        // Evaluate conditional format rules for this cell
        const cfStyle = evaluateConditionalFormat(row, col, cell?.value);

        // If this is a merged anchor, expand dimensions
        if (mergedRegion) {
          cw = mergedWidth(mergedRegion);
          cellHeight = mergedHeight(mergedRegion);
          // Clear the merged area (including internal grid lines)
          ctx.fillStyle = COLORS.cellBg;
          ctx.fillRect(x, y, cw, cellHeight);
        }

        // Effective bg_color: conditional format overrides cell format
        const effectiveBgColor = cfStyle?.bg_color ?? cell?.bg_color;

        // Banded row background (only if no explicit bg_color on cell or CF)
        if (banded && banded.enabled && !effectiveBgColor) {
          if (banded.header_color && row === 0) {
            ctx.fillStyle = banded.header_color;
            ctx.fillRect(x + 1, y + 1, cw - 1, cellHeight - 1);
          } else {
            ctx.fillStyle = row % 2 === 0 ? banded.even_color : banded.odd_color;
            ctx.fillRect(x + 1, y + 1, cw - 1, cellHeight - 1);
          }
        }

        // Draw cell background color if set (even for empty cells).
        // Inset by 1px to preserve grid lines.
        if (effectiveBgColor) {
          ctx.fillStyle = effectiveBgColor;
          ctx.fillRect(x + 1, y + 1, cw - 1, cellHeight - 1);
        }

        // Draw cell borders if set.
        if (cell?.borders) {
          const b = cell.borders;
          const drawBorderLine = (
            edge: { style: string; color: string } | null | undefined,
            x1: number, y1: number, x2: number, y2: number,
          ) => {
            if (!edge || edge.style === 'none') return;
            ctx.strokeStyle = edge.color || '#000000';
            ctx.lineWidth = edge.style === 'thick' ? 3 : edge.style === 'medium' ? 2 : 1;
            if (edge.style === 'dashed') {
              ctx.setLineDash([4, 3]);
            } else if (edge.style === 'dotted') {
              ctx.setLineDash([1, 2]);
            } else if (edge.style === 'double') {
              ctx.setLineDash([]);
              ctx.lineWidth = 1;
              ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke();
              const off = x1 === x2 ? 2 : 0;
              const offY = y1 === y2 ? 2 : 0;
              ctx.beginPath(); ctx.moveTo(x1 + off, y1 + offY); ctx.lineTo(x2 + off, y2 + offY); ctx.stroke();
              return;
            } else {
              ctx.setLineDash([]);
            }
            ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke();
            ctx.setLineDash([]);
            ctx.lineWidth = 1;
          };
          drawBorderLine(b.top, x, y, x + cw, y);
          drawBorderLine(b.bottom, x, y + cellHeight, x + cw, y + cellHeight);
          drawBorderLine(b.left, x, y, x, y + cellHeight);
          drawBorderLine(b.right, x + cw, y, x + cw, y + cellHeight);
        }

        // Draw find match highlights
        if (findMatchSet && findMatchSet.has(`${row}:${col}`)) {
          const isActive = findActiveRow === row && findActiveCol === col;
          ctx.fillStyle = isActive ? 'rgba(0, 180, 80, 0.25)' : 'rgba(255, 235, 59, 0.35)';
          ctx.fillRect(x, y, cw, cellHeight);
        }

        // Draw validation indicators
        const validation = validationCache.get(`${row}:${col}`);
        if (validation) {
          // Small green triangle in top-right corner (like Google Sheets)
          ctx.fillStyle = '#34a853';
          ctx.beginPath();
          ctx.moveTo(x + cw - 7, y + 1);
          ctx.lineTo(x + cw - 1, y + 1);
          ctx.lineTo(x + cw - 1, y + 7);
          ctx.closePath();
          ctx.fill();

          // For list validation, draw a small dropdown arrow on the right
          if (validation.rule_type === 'list') {
            ctx.fillStyle = COLORS.headerText;
            const arrowX = x + cw - 14;
            const arrowY = y + cellHeight / 2 - 2;
            ctx.beginPath();
            ctx.moveTo(arrowX, arrowY);
            ctx.lineTo(arrowX + 8, arrowY);
            ctx.lineTo(arrowX + 4, arrowY + 5);
            ctx.closePath();
            ctx.fill();
          }
        }

        // Skip text rendering for cells with no value
        if (!cell || !cell.value) continue;

        // Image cell: value starts with data:image/
        if (cell.value.startsWith('data:image/')) {
          const img = getCachedImage(cell.value);
          if (img) {
            const maxW = cw - PADDING * 2;
            const maxH = cellHeight - PADDING * 2;
            if (maxW > 0 && maxH > 0) {
              const scale = Math.min(maxW / img.width, maxH / img.height, 1);
              const drawW = img.width * scale;
              const drawH = img.height * scale;
              const drawX = x + PADDING + (maxW - drawW) / 2;
              const drawY = y + PADDING + (maxH - drawH) / 2;
              ctx.drawImage(img, drawX, drawY, drawW, drawH);
            }
          }
          continue;
        }

        // Determine font style (conditional format can override bold/italic)
        const cfBold = cfStyle?.bold ?? undefined;
        const cfItalic = cfStyle?.italic ?? undefined;
        const fontWeight = (cfBold ?? cell.bold) ? 'bold' : 'normal';
        const fontStyle = (cfItalic ?? cell.italic) ? 'italic' : 'normal';
        const fontFamily = cell.font_family || '-apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';
        const fontSize = cell.font_size || 11;
        ctx.font = `${fontStyle} ${fontWeight} ${fontSize}px ${fontFamily}`;
        // Conditional format font_color overrides cell font_color
        const effectiveFontColor = cfStyle?.font_color ?? cell.font_color;
        ctx.fillStyle = effectiveFontColor ?? COLORS.cellText;

        // Auto-align: numbers right, text left — unless user explicitly set center/right.
        // Backend default is "left", so treat "left" as auto (allows number right-align).
        const isNumber = !isNaN(Number(cell.value)) && cell.value.trim() !== '';
        const userSetAlign = cell.h_align && cell.h_align !== 'left';
        const align: CanvasTextAlign = (userSetAlign ? cell.h_align : (isNumber ? 'right' : 'left')) as CanvasTextAlign;
        const maxTextW = cw - PADDING * 2;
        // In formula view, show raw formula instead of computed value
        let displayText = showFormulas() && cell.formula ? `=${cell.formula}` : cell.value;

        // Helper to draw underline and strikethrough decorations on a text segment
        const drawTextDecorations = (textX: number, textY: number, text: string, textAlign: CanvasTextAlign) => {
          if (!cell.underline && !cell.strikethrough) return;
          const tw = ctx.measureText(text).width;
          let lineStartX: number;
          if (textAlign === 'right') {
            lineStartX = textX - tw;
          } else if (textAlign === 'center') {
            lineStartX = textX - tw / 2;
          } else {
            lineStartX = textX;
          }
          ctx.strokeStyle = cell.font_color ?? COLORS.cellText;
          ctx.lineWidth = Math.max(1, fontSize / 12);
          if (cell.underline) {
            const underlineY = textY + fontSize * 0.15;
            ctx.beginPath();
            ctx.moveTo(lineStartX, underlineY);
            ctx.lineTo(lineStartX + tw, underlineY);
            ctx.stroke();
          }
          if (cell.strikethrough) {
            const strikeY = textY;
            ctx.beginPath();
            ctx.moveTo(lineStartX, strikeY);
            ctx.lineTo(lineStartX + tw, strikeY);
            ctx.stroke();
          }
          ctx.lineWidth = 1;
        };

        // Determine text rendering mode
        const textWrap = (cell as CellData).text_wrap ?? 'Overflow';

        if (textWrap === 'Wrap') {
          // Wrap mode: split text into lines that fit cell width
          ctx.textAlign = align;
          const words = displayText.split(' ');
          const lines: string[] = [];
          let currentLine = '';
          for (const word of words) {
            const testLine = currentLine ? `${currentLine} ${word}` : word;
            if (ctx.measureText(testLine).width > maxTextW && currentLine) {
              lines.push(currentLine);
              currentLine = word;
            } else {
              currentLine = testLine;
            }
          }
          if (currentLine) lines.push(currentLine);

          const lineHeight = 16;
          const totalTextH = lines.length * lineHeight;
          const startY = y + (cellHeight - totalTextH) / 2 + lineHeight / 2;
          for (let li = 0; li < lines.length; li++) {
            const lineY = startY + li * lineHeight;
            if (lineY > y + cellHeight) break; // clip vertically
            const textX = align === 'right' ? x + cw - PADDING
              : align === 'center' ? x + cw / 2
              : x + PADDING;
            ctx.fillText(lines[li], textX, lineY);
            drawTextDecorations(textX, lineY, lines[li], align as CanvasTextAlign);
          }
        } else {
          // Overflow or Clip mode
          const measured = ctx.measureText(displayText);
          if (measured.width > maxTextW && maxTextW > 0) {
            if (textWrap === 'Overflow' && align === 'left') {
              // Check if adjacent cells to the right are empty -- allow overflow
              let overflowW = cw;
              let nextCol = col + 1;
              while (nextCol < col + 10 && nextCol < TOTAL_COLS) {
                const nextCell = cellCache.get(`${row}:${nextCol}`);
                if (nextCell && nextCell.value && nextCell.value.trim() !== '') break;
                overflowW += getColWidth(nextCol);
                if (overflowW >= measured.width + PADDING * 2) break;
                nextCol++;
              }
              // Clip to overflow width
              ctx.save();
              ctx.beginPath();
              ctx.rect(x, y, overflowW, cellHeight);
              ctx.clip();
              ctx.textAlign = 'left';
              ctx.fillText(displayText, x + PADDING, y + cellHeight / 2);
              drawTextDecorations(x + PADDING, y + cellHeight / 2, displayText, 'left');
              ctx.restore();
            } else {
              // Clip with ellipsis
              const ellipsis = '\u2026';
              const ellipsisW = ctx.measureText(ellipsis).width;
              let truncLen = displayText.length;
              while (truncLen > 0) {
                truncLen--;
                if (ctx.measureText(displayText.slice(0, truncLen)).width + ellipsisW <= maxTextW) {
                  break;
                }
              }
              displayText = displayText.slice(0, truncLen) + ellipsis;
              ctx.textAlign = align;
              const textX = align === 'right' ? x + cw - PADDING
                : align === 'center' ? x + cw / 2
                : x + PADDING;
              ctx.fillText(displayText, textX, y + cellHeight / 2);
              drawTextDecorations(textX, y + cellHeight / 2, displayText, align as CanvasTextAlign);
            }
          } else {
            ctx.textAlign = align;
            const textX = align === 'right' ? x + cw - PADDING
              : align === 'center' ? x + cw / 2
              : x + PADDING;
            ctx.fillText(displayText, textX, y + cellHeight / 2);
            drawTextDecorations(textX, y + cellHeight / 2, displayText, align as CanvasTextAlign);
          }
        }
      }
    }
  }

  /**
   * Draw page break indicators (blue dashed lines) based on paper size.
   * Only drawn when `pageBreakPreview` is true.
   */
  function drawPageBreaks(
    ctx: CanvasRenderingContext2D,
    sx: number,
    sy: number,
    w: number,
    h: number,
  ) {
    if (!props.pageBreakPreview) return;

    // Convert paper size to pixels at 96 DPI.
    // Paper sizes in inches: letter=8.5x11, a4~8.27x11.69, legal=8.5x14, tabloid=11x17
    let paperW = 8.5;
    let paperH = 11;
    switch (props.pageBreakPaperSize) {
      case 'a4':     paperW = 8.27; paperH = 11.69; break;
      case 'legal':  paperW = 8.5;  paperH = 14;    break;
      case 'tabloid': paperW = 11;  paperH = 17;    break;
      default:       paperW = 8.5;  paperH = 11;    break; // letter
    }

    if (props.pageBreakOrientation === 'landscape') {
      [paperW, paperH] = [paperH, paperW];
    }

    // Subtract 1.5cm margins on each side (~0.59in) and convert to pixels at 96 DPI.
    const marginIn = 0.59;
    const printableW = (paperW - 2 * marginIn) * 96;
    const printableH = (paperH - 2 * marginIn) * 96;

    if (printableW <= 0 || printableH <= 0) return;

    ctx.save();
    ctx.strokeStyle = '#1a73e8';
    ctx.lineWidth = 2;
    ctx.setLineDash([8, 4]);

    // Draw vertical page break lines.
    let accumX = 0;
    let col = 0;
    while (accumX < getColX(TOTAL_COLS)) {
      accumX += printableW;
      // Find the column that crosses this threshold.
      while (col < TOTAL_COLS && getColX(col) < accumX) col++;
      const lineX = ROW_NUMBER_WIDTH + getColX(col) - sx;
      if (lineX > ROW_NUMBER_WIDTH && lineX < w) {
        ctx.beginPath();
        ctx.moveTo(Math.round(lineX), HEADER_HEIGHT);
        ctx.lineTo(Math.round(lineX), h);
        ctx.stroke();
      }
      if (lineX > w) break;
    }

    // Draw horizontal page break lines.
    let accumY = 0;
    let row = 0;
    while (accumY < getRowY(TOTAL_ROWS)) {
      accumY += printableH;
      while (row < TOTAL_ROWS && getRowY(row) < accumY) row++;
      const lineY = HEADER_HEIGHT + getRowY(row) - sy;
      if (lineY > HEADER_HEIGHT && lineY < h) {
        ctx.beginPath();
        ctx.moveTo(ROW_NUMBER_WIDTH, Math.round(lineY));
        ctx.lineTo(w, Math.round(lineY));
        ctx.stroke();
      }
      if (lineY > h) break;
    }

    ctx.restore();
  }

  function drawGridLines(
    ctx: CanvasRenderingContext2D,
    sx: number,
    sy: number,
    startCol: number,
    startRow: number,
    colCount: number,
    rowCount: number,
    w: number,
    h: number,
  ) {
    // Skip grid lines if toggled off via View > Show Gridlines.
    if (props.showGridlines === false) return;

    ctx.strokeStyle = COLORS.gridBorder;
    ctx.lineWidth = 1;

    // Vertical grid lines (column borders)
    for (let c = 0; c <= colCount; c++) {
      const col = startCol + c;
      const x = ROW_NUMBER_WIDTH + getColX(col) - sx;
      if (x < ROW_NUMBER_WIDTH || x > w) continue;
      ctx.beginPath();
      ctx.moveTo(Math.round(x) + 0.5, HEADER_HEIGHT);
      ctx.lineTo(Math.round(x) + 0.5, h);
      ctx.stroke();
    }

    // Horizontal grid lines (row borders)
    for (let r = 0; r <= rowCount; r++) {
      const row = startRow + r;
      const y = HEADER_HEIGHT + getRowY(row) - sy;
      if (y < HEADER_HEIGHT || y > h) continue;
      ctx.beginPath();
      ctx.moveTo(ROW_NUMBER_WIDTH, Math.round(y) + 0.5);
      ctx.lineTo(w, Math.round(y) + 0.5);
      ctx.stroke();
    }
  }

  function drawColumnHeaders(
    ctx: CanvasRenderingContext2D,
    sx: number,
    startCol: number,
    colCount: number,
    w: number,
  ) {
    ctx.fillStyle = COLORS.headerBg;
    ctx.fillRect(0, 0, w, HEADER_HEIGHT);

    ctx.strokeStyle = COLORS.gridBorder;
    ctx.beginPath();
    ctx.moveTo(0, HEADER_HEIGHT + 0.5);
    ctx.lineTo(w, HEADER_HEIGHT + 0.5);
    ctx.stroke();

    ctx.font = '11px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';

    for (let c = 0; c < colCount; c++) {
      const col = startCol + c;
      // Skip hidden columns
      if (hiddenCols.has(col)) continue;
      const cw = getColWidth(col);
      const x = ROW_NUMBER_WIDTH + getColX(col) - sx;
      const cellRight = x + cw;
      if (cellRight < ROW_NUMBER_WIDTH || x > w) continue;

      ctx.strokeStyle = COLORS.gridBorder;
      ctx.beginPath();
      ctx.moveTo(Math.round(cellRight) + 0.5, 0);
      ctx.lineTo(Math.round(cellRight) + 0.5, HEADER_HEIGHT);
      ctx.stroke();

      // Highlight selected column header
      if (isColInSelection(col)) {
        ctx.fillStyle = COLORS.selectionBg;
        ctx.fillRect(x, 0, cw, HEADER_HEIGHT);
        ctx.fillStyle = COLORS.selectionBorder;
      } else {
        ctx.fillStyle = COLORS.headerText;
      }
      ctx.fillText(col_to_letter(col), x + cw / 2, HEADER_HEIGHT / 2);

      // Draw filter dropdown arrow on header row columns in the filter range
      if (
        props.filterActive &&
        props.filterStartCol !== undefined &&
        props.filterEndCol !== undefined &&
        col >= props.filterStartCol &&
        col <= props.filterEndCol
      ) {
        ctx.fillStyle = COLORS.headerText;
        const arrowX = x + cw - 12;
        const arrowY = HEADER_HEIGHT / 2 - 2;
        ctx.beginPath();
        ctx.moveTo(arrowX, arrowY);
        ctx.lineTo(arrowX + 7, arrowY);
        ctx.lineTo(arrowX + 3.5, arrowY + 4);
        ctx.closePath();
        ctx.fill();
      }

      // Draw hidden-column indicator: double line at left edge if previous col is hidden
      if (col > 0 && hiddenCols.has(col - 1) && !hiddenCols.has(col)) {
        ctx.strokeStyle = COLORS.selectionBorder;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(Math.round(x) - 1, 0);
        ctx.lineTo(Math.round(x) - 1, HEADER_HEIGHT);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(Math.round(x) + 2, 0);
        ctx.lineTo(Math.round(x) + 2, HEADER_HEIGHT);
        ctx.stroke();
        ctx.lineWidth = 1;
      }
    }
  }

  function drawRowNumbers(
    ctx: CanvasRenderingContext2D,
    sy: number,
    startRow: number,
    rowCount: number,
    h: number,
  ) {
    ctx.fillStyle = COLORS.headerBg;
    ctx.fillRect(0, HEADER_HEIGHT, ROW_NUMBER_WIDTH, h - HEADER_HEIGHT);

    ctx.strokeStyle = COLORS.gridBorder;
    ctx.beginPath();
    ctx.moveTo(ROW_NUMBER_WIDTH + 0.5, HEADER_HEIGHT);
    ctx.lineTo(ROW_NUMBER_WIDTH + 0.5, h);
    ctx.stroke();

    ctx.font = '11px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';

    for (let r = 0; r < rowCount; r++) {
      const row = startRow + r;
      // Skip hidden rows (e.g. from auto-filter)
      if (hiddenRows.has(row)) continue;
      const rh = getRowHeight(row);
      const y = HEADER_HEIGHT + getRowY(row) - sy;
      const cellBottom = y + rh;
      if (cellBottom < HEADER_HEIGHT || y > h) continue;

      ctx.strokeStyle = COLORS.gridBorder;
      ctx.beginPath();
      ctx.moveTo(0, Math.round(cellBottom) + 0.5);
      ctx.lineTo(ROW_NUMBER_WIDTH, Math.round(cellBottom) + 0.5);
      ctx.stroke();

      // Highlight selected row number
      if (isRowInSelection(row)) {
        ctx.fillStyle = COLORS.selectionBg;
        ctx.fillRect(0, y, ROW_NUMBER_WIDTH, rh);
        ctx.fillStyle = COLORS.selectionBorder;
      } else {
        ctx.fillStyle = COLORS.headerText;
      }
      ctx.fillText(String(row + 1), ROW_NUMBER_WIDTH / 2, y + rh / 2);

      // Draw hidden-row indicator: double line at top edge if previous row is hidden
      if (row > 0 && hiddenRows.has(row - 1) && !hiddenRows.has(row)) {
        ctx.strokeStyle = COLORS.selectionBorder;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(0, Math.round(y) - 1);
        ctx.lineTo(ROW_NUMBER_WIDTH, Math.round(y) - 1);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(0, Math.round(y) + 2);
        ctx.lineTo(ROW_NUMBER_WIDTH, Math.round(y) + 2);
        ctx.stroke();
        ctx.lineWidth = 1;
      }

      // Draw row group +/- indicator in left gutter (indented by level)
      for (let gi = 0; gi < rowGroups.length; gi++) {
        const g = rowGroups[gi];
        const indent = (g.level ?? 0) * 10;
        if (g.collapsed) {
          // Show + button on the row just before the collapsed group (or at group start if start=0)
          const indicatorRow = g.start > 0 ? g.start - 1 : g.start;
          if (row === indicatorRow) {
            const btnSize = 9;
            const btnX = 2 + indent;
            const btnY = y + rh / 2 - btnSize / 2;
            ctx.strokeStyle = COLORS.headerText;
            ctx.lineWidth = 1;
            ctx.strokeRect(btnX, btnY, btnSize, btnSize);
            // Draw + sign
            ctx.beginPath();
            ctx.moveTo(btnX + 2, btnY + btnSize / 2);
            ctx.lineTo(btnX + btnSize - 2, btnY + btnSize / 2);
            ctx.moveTo(btnX + btnSize / 2, btnY + 2);
            ctx.lineTo(btnX + btnSize / 2, btnY + btnSize - 2);
            ctx.stroke();
          }
        } else {
          // Show - button on the first row of the expanded group
          if (row === g.start) {
            const btnSize = 9;
            const btnX = 2 + indent;
            const btnY = y + rh / 2 - btnSize / 2;
            ctx.strokeStyle = COLORS.headerText;
            ctx.lineWidth = 1;
            ctx.strokeRect(btnX, btnY, btnSize, btnSize);
            // Draw - sign
            ctx.beginPath();
            ctx.moveTo(btnX + 2, btnY + btnSize / 2);
            ctx.lineTo(btnX + btnSize - 2, btnY + btnSize / 2);
            ctx.stroke();
          }
          // Draw a bracket line along the gutter for grouped rows
          if (row >= g.start && row <= g.end) {
            const lineX = 6 + indent;
            if (row === g.start) {
              ctx.strokeStyle = COLORS.headerText;
              ctx.lineWidth = 1;
              ctx.beginPath();
              ctx.moveTo(lineX, y + rh / 2 + 6);
              ctx.lineTo(lineX, y + rh);
              ctx.stroke();
            } else if (row === g.end) {
              ctx.strokeStyle = COLORS.headerText;
              ctx.lineWidth = 1;
              ctx.beginPath();
              ctx.moveTo(lineX, y);
              ctx.lineTo(lineX, y + rh / 2);
              ctx.lineTo(lineX + 4, y + rh / 2);
              ctx.stroke();
            } else {
              ctx.strokeStyle = COLORS.headerText;
              ctx.lineWidth = 1;
              ctx.beginPath();
              ctx.moveTo(lineX, y);
              ctx.lineTo(lineX, y + rh);
              ctx.stroke();
            }
          }
        }
      }
    }
  }

  function drawCorner(ctx: CanvasRenderingContext2D) {
    ctx.fillStyle = COLORS.cornerBg;
    ctx.fillRect(0, 0, ROW_NUMBER_WIDTH, HEADER_HEIGHT);
    ctx.strokeStyle = COLORS.gridBorder;
    ctx.strokeRect(0, 0, ROW_NUMBER_WIDTH, HEADER_HEIGHT);

    // Draw row group level indicator buttons (1, 2, 3...) when groups exist.
    if (rowGroups.length > 0) {
      const maxLevel = Math.max(...rowGroups.map((g) => g.level ?? 0));
      const btnSize = 10;
      const btnGap = 2;
      const startX = 1;
      const btnY = (HEADER_HEIGHT - btnSize) / 2;
      ctx.font = '8px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      for (let lvl = 0; lvl <= maxLevel; lvl++) {
        const bx = startX + lvl * (btnSize + btnGap);
        ctx.strokeStyle = COLORS.headerText;
        ctx.lineWidth = 1;
        ctx.strokeRect(bx, btnY, btnSize, btnSize);
        ctx.fillStyle = COLORS.headerText;
        ctx.fillText(String(lvl + 1), bx + btnSize / 2, btnY + btnSize / 2);
      }
    }
  }

  // -----------------------------------------------------------------------
  // Resize state
  // -----------------------------------------------------------------------

  // Pixel tolerance for detecting header border hover.
  const RESIZE_HANDLE_PX = 5;

  // Active resize drag state (null when not resizing).
  let resizeDrag: {
    kind: 'col' | 'row';
    index: number;
    startMouse: number; // clientX for col, clientY for row
    startSize: number;  // original width/height
  } | null = null;

  // Double-click tracking for auto-fit.
  let lastResizeBorderClickTime = 0;
  let lastResizeBorderCol = -1;
  let lastResizeBorderRow = -1;

  /**
   * Check if the mouse is over a column header border (right edge).
   * Returns the column index whose right edge is near the mouse, or -1.
   */
  /** Get the effective scroll-X for a given screen local-X, accounting for freeze/split. */
  function effectiveScrollX(localX: number): number {
    const fc = props.frozenCols ?? 0;
    if (fc > 0 && localX < ROW_NUMBER_WIDTH + frozenColsPx()) return 0;
    const sc = props.splitCol ?? 0;
    if (sc > 0 && localX < ROW_NUMBER_WIDTH + splitColsPx()) return splitScrollX();
    return scrollX();
  }

  /** Get the effective scroll-Y for a given screen local-Y, accounting for freeze/split. */
  function effectiveScrollY(localY: number): number {
    const fr = props.frozenRows ?? 0;
    if (fr > 0 && localY < HEADER_HEIGHT + frozenRowsPx()) return 0;
    const sr = props.splitRow ?? 0;
    if (sr > 0 && localY < HEADER_HEIGHT + splitRowsPx()) return splitScrollY();
    return scrollY();
  }

  function hitColHeaderBorder(localX: number, localY: number): number {
    if (localY >= HEADER_HEIGHT || localY < 0) return -1;
    if (localX < ROW_NUMBER_WIDTH) return -1;
    const effSx = effectiveScrollX(localX);
    const contentX = localX - ROW_NUMBER_WIDTH + effSx;
    // Check nearby columns
    const approxCol = Math.floor(contentX / DEFAULT_COL_WIDTH);
    const start = Math.max(0, approxCol - 2);
    const end = Math.min(TOTAL_COLS, approxCol + 3);
    for (let c = start; c < end; c++) {
      const rightEdge = getColX(c + 1);
      const screenRight = ROW_NUMBER_WIDTH + rightEdge - effSx;
      if (Math.abs(localX - screenRight) <= RESIZE_HANDLE_PX) {
        return c;
      }
    }
    return -1;
  }

  /**
   * Check if the mouse is over a row header border (bottom edge).
   * Returns the row index whose bottom edge is near the mouse, or -1.
   */
  function hitRowHeaderBorder(localX: number, localY: number): number {
    if (localX >= ROW_NUMBER_WIDTH || localX < 0) return -1;
    if (localY < HEADER_HEIGHT) return -1;
    const effSy = effectiveScrollY(localY);
    const contentY = localY - HEADER_HEIGHT + effSy;
    const approxRow = Math.floor(contentY / DEFAULT_ROW_HEIGHT);
    const start = Math.max(0, approxRow - 2);
    const end = Math.min(TOTAL_ROWS, approxRow + 3);
    for (let r = start; r < end; r++) {
      const bottomEdge = getRowY(r + 1);
      const screenBottom = HEADER_HEIGHT + bottomEdge - effSy;
      if (Math.abs(localY - screenBottom) <= RESIZE_HANDLE_PX) {
        return r;
      }
    }
    return -1;
  }

  /** Auto-fit a column width based on measuring visible cell text widths. */
  function autoFitColumn(col: number) {
    const canvas = canvasRef;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const PADDING = 4;
    let maxW = MIN_COL_WIDTH;

    // Measure header text
    ctx.font = '11px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';
    const headerW = ctx.measureText(col_to_letter(col)).width + PADDING * 2;
    maxW = Math.max(maxW, headerW);

    // Measure visible cell text
    const startRow = firstVisibleRow();
    const rowCount = visibleRowCount();
    for (let r = 0; r < rowCount; r++) {
      const row = startRow + r;
      const cell = cellCache.get(`${row}:${col}`);
      if (!cell || !cell.value) continue;
      const fontWeight = cell.bold ? 'bold' : 'normal';
      const fontStyle = cell.italic ? 'italic' : 'normal';
      const fontSize = cell.font_size || 11;
      const fontFamily = cell.font_family || '-apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif';
      ctx.font = `${fontStyle} ${fontWeight} ${fontSize}px ${fontFamily}`;
      const tw = ctx.measureText(cell.value).width + PADDING * 2 + 4;
      maxW = Math.max(maxW, tw);
    }

    maxW = Math.ceil(maxW);
    if (maxW === DEFAULT_COL_WIDTH) {
      colWidths.delete(col);
    } else {
      colWidths.set(col, maxW);
    }
    tauriSetColWidth(props.activeSheet, col, maxW).catch(() => {});
    draw();
  }

  /** Auto-fit a row height based on the default (reset to default). */
  function autoFitRow(row: number) {
    rowHeights.delete(row);
    tauriSetRowHeight(props.activeSheet, row, DEFAULT_ROW_HEIGHT).catch(() => {});
    draw();
  }

  // Drag-to-select state
  let isDragging = false;
  // Track header drag kind: 'col' for column header drag, 'row' for row header drag, null for cell drag
  let headerDragKind: 'col' | 'row' | null = null;
  let headerDragStartIndex = 0;

  function handleMouseMove(e: MouseEvent) {
    if (!containerRef) return;
    const rect = containerRef.getBoundingClientRect();
    const localX = e.clientX - rect.left;
    const localY = e.clientY - rect.top;

    // If actively resizing, update size.
    if (resizeDrag) {
      if (resizeDrag.kind === 'col') {
        const delta = e.clientX - resizeDrag.startMouse;
        const newW = Math.max(MIN_COL_WIDTH, resizeDrag.startSize + delta);
        colWidths.set(resizeDrag.index, newW);
      } else {
        const delta = e.clientY - resizeDrag.startMouse;
        const newH = Math.max(MIN_ROW_HEIGHT, resizeDrag.startSize + delta);
        rowHeights.set(resizeDrag.index, newH);
      }
      scheduleDraw();
      return;
    }

    // If dragging across column headers, extend column selection.
    if (isDragging && headerDragKind === 'col') {
      const effSx = effectiveScrollX(localX);
      const contentX = Math.max(0, localX - ROW_NUMBER_WIDTH + effSx);
      const col = colAtX(contentX);
      const curEnd = rangeEnd();
      if (!curEnd || curEnd[1] !== col) {
        setRangeAnchor([0, headerDragStartIndex]);
        setRangeEnd([TOTAL_ROWS - 1, col]);
        scheduleDraw();
      }
      return;
    }

    // If dragging across row headers, extend row selection.
    if (isDragging && headerDragKind === 'row') {
      const effSy = effectiveScrollY(localY);
      const contentY = Math.max(0, localY - HEADER_HEIGHT + effSy);
      const row = rowAtY(contentY);
      const curEnd = rangeEnd();
      if (!curEnd || curEnd[0] !== row) {
        setRangeAnchor([headerDragStartIndex, 0]);
        setRangeEnd([row, TOTAL_COLS - 1]);
        scheduleDraw();
      }
      return;
    }

    // If dragging to select cells, update the range end.
    if (isDragging) {
      const hit = hitTest(e.clientX, e.clientY);
      if (hit) {
        const curEnd = rangeEnd();
        // Only update and redraw if the cell actually changed
        if (!curEnd || curEnd[0] !== hit.row || curEnd[1] !== hit.col) {
          setRangeEnd([hit.row, hit.col]);
          scheduleDraw();
        }
      }
      return;
    }

    // Update cursor based on hover position.
    if (fillHandleHit(localX, localY)) {
      containerRef.style.cursor = 'crosshair';
    } else if (hitColHeaderBorder(localX, localY) >= 0) {
      containerRef.style.cursor = 'col-resize';
    } else if (hitRowHeaderBorder(localX, localY) >= 0) {
      containerRef.style.cursor = 'row-resize';
    } else {
      containerRef.style.cursor = '';
    }
  }

  function handleDragMouseUp() {
    if (!isDragging) return;
    isDragging = false;
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleDragMouseUp);
    // Propagate the final selection range to the parent so toolbar operations
    // (bold, italic, etc.) use the correct range.
    propagateSelectionRange();
  }

  function handleResizeMouseUp() {
    if (!resizeDrag) return;
    // If the width/height matches the default, remove the override.
    if (resizeDrag.kind === 'col') {
      const w = colWidths.get(resizeDrag.index);
      if (w !== undefined && w === DEFAULT_COL_WIDTH) {
        colWidths.delete(resizeDrag.index);
      }
      // Persist to backend
      const finalW = colWidths.get(resizeDrag.index) ?? DEFAULT_COL_WIDTH;
      tauriSetColWidth(props.activeSheet, resizeDrag.index, finalW).catch(() => {});
    } else {
      const h = rowHeights.get(resizeDrag.index);
      if (h !== undefined && h === DEFAULT_ROW_HEIGHT) {
        rowHeights.delete(resizeDrag.index);
      }
      // Persist to backend
      const finalH = rowHeights.get(resizeDrag.index) ?? DEFAULT_ROW_HEIGHT;
      tauriSetRowHeight(props.activeSheet, resizeDrag.index, finalH).catch(() => {});
    }
    resizeDrag = null;
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleResizeMouseUp);
    if (containerRef) {
      containerRef.style.cursor = '';
    }
    draw();
  }

  // -----------------------------------------------------------------------
  // Fill handle (auto-fill) state
  // -----------------------------------------------------------------------

  let isFillDragging = false;
  const [fillDragRow, setFillDragRow] = createSignal(-1);
  const [fillDragCol, setFillDragCol] = createSignal(-1);

  /** Check if a local (container-relative) coordinate hits the fill handle. */
  function fillHandleHit(localX: number, localY: number): boolean {
    const range = getSelectionRange();
    const sx = effectiveScrollX(localX);
    const sy = effectiveScrollY(localY);
    // Handle is drawn at the bottom-right corner of the selection, offset by half size
    const handleCenterX = ROW_NUMBER_WIDTH + getColX(range.maxCol) + getColWidth(range.maxCol) - sx - FILL_HANDLE_SIZE / 2;
    const handleCenterY = HEADER_HEIGHT + getRowY(range.maxRow) + getRowHeight(range.maxRow) - sy - FILL_HANDLE_SIZE / 2;
    return (
      Math.abs(localX - handleCenterX) <= FILL_HANDLE_HIT_SIZE / 2 &&
      Math.abs(localY - handleCenterY) <= FILL_HANDLE_HIT_SIZE / 2
    );
  }

  /** Get the fill preview range (the cells that will be filled). */
  function getFillPreviewRange(): { minRow: number; maxRow: number; minCol: number; maxCol: number } | null {
    if (!isFillDragging) return null;
    const range = getSelectionRange();
    const dragR = fillDragRow();
    const dragC = fillDragCol();
    if (dragR < 0 || dragC < 0) return null;

    // Determine fill direction by which axis has more displacement
    const dRow = dragR - range.maxRow;
    const dRowUp = range.minRow - dragR;
    const dCol = dragC - range.maxCol;
    const dColLeft = range.minCol - dragC;

    const maxDisp = Math.max(dRow, dRowUp, dCol, dColLeft);
    if (maxDisp <= 0) return null;

    if (dRow === maxDisp) {
      // Fill down
      return { minRow: range.maxRow + 1, maxRow: dragR, minCol: range.minCol, maxCol: range.maxCol };
    } else if (dRowUp === maxDisp) {
      // Fill up
      return { minRow: dragR, maxRow: range.minRow - 1, minCol: range.minCol, maxCol: range.maxCol };
    } else if (dCol === maxDisp) {
      // Fill right
      return { minRow: range.minRow, maxRow: range.maxRow, minCol: range.maxCol + 1, maxCol: dragC };
    } else {
      // Fill left
      return { minRow: range.minRow, maxRow: range.maxRow, minCol: dragC, maxCol: range.minCol - 1 };
    }
  }

  /** Execute auto-fill: detect pattern in source cells and fill target range. */
  async function executeFill() {
    const range = getSelectionRange();
    const fillRange = getFillPreviewRange();
    if (!fillRange) {
      return;
    }

    // Determine fill direction
    const isVertical = fillRange.minCol === range.minCol && fillRange.maxCol === range.maxCol;
    const isDown = isVertical && fillRange.minRow > range.maxRow;
    const isUp = isVertical && fillRange.maxRow < range.minRow;
    const isRight = !isVertical && fillRange.minRow === range.minRow;
    const isLeft = !isVertical && fillRange.maxCol < range.minCol;

    const promises: Promise<void>[] = [];

    if (isVertical) {
      // Fill each column independently
      for (let c = range.minCol; c <= range.maxCol; c++) {
        // Collect source values for this column
        const sourceVals: string[] = [];
        for (let r = range.minRow; r <= range.maxRow; r++) {
          const cached = cellCache.get(`${r}:${c}`);
          sourceVals.push(cached?.value ?? '');
        }

        // Detect pattern and fill
        const fillCount = isDown
          ? fillRange.maxRow - fillRange.minRow + 1
          : fillRange.maxRow - fillRange.minRow + 1;
        const filledValues = detectAndFill(sourceVals, fillCount, isUp);

        for (let i = 0; i < filledValues.length; i++) {
          const targetRow = isDown ? fillRange.minRow + i : fillRange.maxRow - i;
          promises.push(
            setCell(props.activeSheet, targetRow, c, filledValues[i]).catch(() => {}),
          );
        }
      }
    } else {
      // Fill each row independently
      for (let r = range.minRow; r <= range.maxRow; r++) {
        const sourceVals: string[] = [];
        for (let c = range.minCol; c <= range.maxCol; c++) {
          const cached = cellCache.get(`${r}:${c}`);
          sourceVals.push(cached?.value ?? '');
        }

        const fillCount = isRight
          ? fillRange.maxCol - fillRange.minCol + 1
          : fillRange.maxCol - fillRange.minCol + 1;
        const filledValues = detectAndFill(sourceVals, fillCount, isLeft);

        for (let i = 0; i < filledValues.length; i++) {
          const targetCol = isRight ? fillRange.minCol + i : fillRange.maxCol - i;
          promises.push(
            setCell(props.activeSheet, r, targetCol, filledValues[i]).catch(() => {}),
          );
        }
      }
    }

    await Promise.all(promises);
    lastFetchKey = '';
    fetchVisibleData();
    props.onStatusChange('Auto-filled cells');
  }

  /** Simple frontend pattern detection and fill value generation. */
  function detectAndFill(sourceVals: string[], count: number, _reverse: boolean): string[] {
    const result: string[] = [];
    const len = sourceVals.length;

    // Try numeric linear pattern
    const nums = sourceVals.map(Number);
    const allNumeric = sourceVals.every((v) => v.trim() !== '' && !isNaN(Number(v)));

    if (allNumeric && len >= 2) {
      const step = nums[len - 1] - nums[len - 2];
      const isInteger = nums.every((n) => Number.isInteger(n)) && Number.isInteger(step);
      for (let i = 0; i < count; i++) {
        const val = nums[len - 1] + step * (i + 1);
        result.push(isInteger ? String(Math.round(val)) : String(val));
      }
      return result;
    }

    // Single numeric value: repeat (constant fill)
    if (allNumeric && len === 1) {
      for (let i = 0; i < count; i++) {
        result.push(sourceVals[0]);
      }
      return result;
    }

    // Default: repeat the source values cyclically
    for (let i = 0; i < count; i++) {
      result.push(sourceVals[i % len]);
    }
    return result;
  }

  function handleFillMouseUp() {
    if (!isFillDragging) return;
    // Execute fill BEFORE clearing the drag state, since getFillPreviewRange()
    // checks isFillDragging.
    document.removeEventListener('mousemove', handleFillMouseMove);
    document.removeEventListener('mouseup', handleFillMouseUp);
    if (containerRef) {
      containerRef.style.cursor = '';
    }
    executeFill();
    isFillDragging = false;
    setFillDragRow(-1);
    setFillDragCol(-1);
    draw();
  }

  function handleFillMouseMove(e: MouseEvent) {
    if (!isFillDragging || !containerRef) return;
    const hit = hitTest(e.clientX, e.clientY);
    if (hit) {
      setFillDragRow(hit.row);
      setFillDragCol(hit.col);
      scheduleDraw();
    }
  }

  // -----------------------------------------------------------------------
  // Hit testing & event handlers
  // -----------------------------------------------------------------------

  /** Find which column a content-x coordinate falls in. */
  function colAtX(contentX: number): number {
    // Quick estimate, then adjust
    let col = Math.floor(contentX / DEFAULT_COL_WIDTH);
    col = Math.max(0, Math.min(col, TOTAL_COLS - 1));
    // Adjust forward/backward
    while (col > 0 && getColX(col) > contentX) col--;
    while (col < TOTAL_COLS - 1 && getColX(col + 1) <= contentX) col++;
    // Skip hidden columns forward to nearest visible column
    while (col < TOTAL_COLS - 1 && hiddenCols.has(col)) col++;
    return col;
  }

  /** Find which row a content-y coordinate falls in. */
  function rowAtY(contentY: number): number {
    let row = Math.floor(contentY / DEFAULT_ROW_HEIGHT);
    row = Math.max(0, Math.min(row, TOTAL_ROWS - 1));
    while (row > 0 && getRowY(row) > contentY) row--;
    while (row < TOTAL_ROWS - 1 && getRowY(row + 1) <= contentY) row++;
    // Skip hidden rows forward to nearest visible row
    while (row < TOTAL_ROWS - 1 && hiddenRows.has(row)) row++;
    return row;
  }

  function hitTest(
    clientX: number,
    clientY: number,
  ): { row: number; col: number } | null {
    if (!containerRef) return null;
    const rect = containerRef.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    if (x < ROW_NUMBER_WIDTH || y < HEADER_HEIGHT) return null;
    const contentX = x - ROW_NUMBER_WIDTH + effectiveScrollX(x);
    const contentY = y - HEADER_HEIGHT + effectiveScrollY(y);
    const col = colAtX(contentX);
    const row = rowAtY(contentY);
    if (col < 0 || col >= TOTAL_COLS || row < 0 || row >= TOTAL_ROWS) return null;
    return { row, col };
  }

  // -----------------------------------------------------------------------
  // Formula click-to-reference helpers
  // -----------------------------------------------------------------------

  /** Build an A1-style reference string from row/col. */
  function buildCellRef(row: number, col: number): string {
    return `${col_to_letter(col)}${row + 1}`;
  }

  /** Build a range reference string from two corners. */
  function buildRangeRef(r1: number, c1: number, r2: number, c2: number): string {
    const minR = Math.min(r1, r2);
    const minC = Math.min(c1, c2);
    const maxR = Math.max(r1, r2);
    const maxC = Math.max(c1, c2);
    if (minR === maxR && minC === maxC) return buildCellRef(minR, minC);
    return `${buildCellRef(minR, minC)}:${buildCellRef(maxR, maxC)}`;
  }

  /**
   * Insert a cell reference at the cursor position in the formula editor.
   * If there's already a formula-ref insertion in progress (from drag), replace it.
   */
  function insertFormulaRef(row: number, col: number, endRow?: number, endCol?: number) {
    if (!editorRef) return;
    const value = editValue();
    const ref = (endRow !== undefined && endCol !== undefined)
      ? buildRangeRef(row, col, endRow, endCol)
      : buildCellRef(row, col);

    let newValue: string;
    let newCursorPos: number;

    if (formulaRefInsertStart >= 0 && formulaRefInsertEnd >= 0) {
      // Replace the previously inserted reference (drag-extend)
      newValue = value.slice(0, formulaRefInsertStart) + ref + value.slice(formulaRefInsertEnd);
      formulaRefInsertEnd = formulaRefInsertStart + ref.length;
      newCursorPos = formulaRefInsertEnd;
    } else {
      // Fresh insertion at the cursor position
      const cursorPos = editorRef.selectionStart ?? value.length;
      newValue = value.slice(0, cursorPos) + ref + value.slice(cursorPos);
      formulaRefInsertStart = cursorPos;
      formulaRefInsertEnd = cursorPos + ref.length;
      newCursorPos = formulaRefInsertEnd;
    }

    setEditValue(newValue);
    props.onContentChange(newValue);
    // Update cursor position after DOM updates
    requestAnimationFrame(() => {
      if (editorRef) {
        editorRef.setSelectionRange(newCursorPos, newCursorPos);
        editorRef.focus();
      }
    });
    scheduleDraw();
  }

  function handleFormulaRefMouseMove(e: MouseEvent) {
    if (!formulaRefDragging || !formulaRefAnchor) return;
    const hit = hitTest(e.clientX, e.clientY);
    if (!hit) return;
    // Update the inserted reference to a range
    insertFormulaRef(formulaRefAnchor.row, formulaRefAnchor.col, hit.row, hit.col);
  }

  function handleFormulaRefMouseUp() {
    formulaRefDragging = false;
    formulaRefAnchor = null;
    // Reset insert tracking so the next click creates a new insertion
    formulaRefInsertStart = -1;
    formulaRefInsertEnd = -1;
    document.removeEventListener('mousemove', handleFormulaRefMouseMove);
    document.removeEventListener('mouseup', handleFormulaRefMouseUp);
    editorRef?.focus();
  }

  let lastClickTime = 0;
  let lastClickRow = -1;
  let lastClickCol = -1;

  function handleMouseDown(e: MouseEvent) {
    // Dismiss context menu on any click
    if (ctxMenuVisible()) {
      dismissContextMenu();
    }
    // Dismiss validation dropdown on any click outside it
    if (validationDropdownVisible()) {
      setValidationDropdownVisible(false);
    }

    if (editing() && !isFormulaSelectionMode()) return; // let the editor handle clicks
    if (!containerRef) return;

    // Formula selection mode: clicking inserts a cell reference into the editor.
    // Skip if the click is on the editor textarea itself (cursor positioning).
    if (isFormulaSelectionMode() && e.target !== editorRef) {
      const hit = hitTest(e.clientX, e.clientY);
      if (hit) {
        e.preventDefault();
        e.stopPropagation();
        insertFormulaRef(hit.row, hit.col);
        // Start formula-ref drag to allow range selection
        formulaRefDragging = true;
        formulaRefAnchor = { row: hit.row, col: hit.col };
        document.addEventListener('mousemove', handleFormulaRefMouseMove);
        document.addEventListener('mouseup', handleFormulaRefMouseUp);
      }
      return;
    }

    const rect = containerRef.getBoundingClientRect();
    const localX = e.clientX - rect.left;
    const localY = e.clientY - rect.top;

    // Check if click is on a row group level button in the corner area
    if (localX < ROW_NUMBER_WIDTH && localY < HEADER_HEIGHT && rowGroups.length > 0) {
      const maxLevel = Math.max(...rowGroups.map((g) => g.level ?? 0));
      const btnSize = 10;
      const btnGap = 2;
      const startX = 1;
      const btnY = (HEADER_HEIGHT - btnSize) / 2;
      for (let lvl = 0; lvl <= maxLevel; lvl++) {
        const bx = startX + lvl * (btnSize + btnGap);
        if (localX >= bx && localX <= bx + btnSize && localY >= btnY && localY <= btnY + btnSize) {
          e.preventDefault();
          // Toggle all groups at this level: collapse expanded ones, or expand all if all are collapsed
          const groupsAtLevel = rowGroups.filter((g) => (g.level ?? 0) === lvl);
          const allCollapsed = groupsAtLevel.every((g) => g.collapsed);
          const togglePromises = rowGroups.map((g, gi) => {
            if ((g.level ?? 0) !== lvl) return Promise.resolve();
            if (allCollapsed && g.collapsed) return toggleRowGroup(props.activeSheet, gi);
            if (!allCollapsed && !g.collapsed) return toggleRowGroup(props.activeSheet, gi);
            return Promise.resolve();
          });
          void Promise.all(togglePromises).then(() => {
            fetchRowGroups();
            fetchHiddenRows();
            lastFetchKey = '';
            fetchVisibleData();
            draw();
          }).catch(() => props.onStatusChange('Toggle level failed'));
          return;
        }
      }
    }

    // Check if click is on a row group +/- button in the gutter
    const gutterWidth = rowGroups.length > 0
      ? 14 + Math.max(...rowGroups.map((g) => (g.level ?? 0))) * 10
      : 14;
    if (localX < gutterWidth && localY >= HEADER_HEIGHT && rowGroups.length > 0) {
      const contentY = localY - HEADER_HEIGHT + effectiveScrollY(localY);
      const clickRow = rowAtY(contentY);
      for (let gi = 0; gi < rowGroups.length; gi++) {
        const g = rowGroups[gi];
        const indent = (g.level ?? 0) * 10;
        const indicatorRow = g.collapsed
          ? (g.start > 0 ? g.start - 1 : g.start)
          : g.start;
        if (clickRow === indicatorRow && localX >= indent && localX < indent + 14) {
          e.preventDefault();
          void toggleRowGroup(props.activeSheet, gi).then(() => {
            fetchRowGroups();
            fetchHiddenRows();
            lastFetchKey = '';
            fetchVisibleData();
            draw();
          }).catch(() => props.onStatusChange('Toggle row group failed'));
          return;
        }
      }
    }

    // Check if click is on a list validation dropdown arrow
    if (localX >= ROW_NUMBER_WIDTH && localY >= HEADER_HEIGHT) {
      const contentX = localX - ROW_NUMBER_WIDTH + effectiveScrollX(localX);
      const contentY = localY - HEADER_HEIGHT + effectiveScrollY(localY);
      const col = colAtX(contentX);
      const row = rowAtY(contentY);
      const validation = validationCache.get(`${row}:${col}`);
      if (validation && validation.rule_type === 'list') {
        // Check if click is near the right edge (dropdown arrow area)
        const cellRight = getColX(col) + getColWidth(col);
        const clickOffsetFromRight = cellRight - contentX;
        if (clickOffsetFromRight <= 16) {
          // Show the validation dropdown
          const items = (validation.list_items ?? '').split(',').map(s => s.trim()).filter(s => s.length > 0);
          if (items.length > 0) {
            const cellScreenX = ROW_NUMBER_WIDTH + getColX(col) - effectiveScrollX(localX);
            const cellScreenY = HEADER_HEIGHT + getRowY(row) - effectiveScrollY(localY) + getRowHeight(row);
            setValidationDropdownItems(items);
            setValidationDropdownX(cellScreenX);
            setValidationDropdownY(cellScreenY);
            setValidationDropdownRow(row);
            setValidationDropdownCol(col);
            setValidationDropdownVisible(true);
            e.preventDefault();
            return;
          }
        }
      }
    }

    // Check for column header border resize drag.
    const resizeCol = hitColHeaderBorder(localX, localY);
    if (resizeCol >= 0) {
      const now = Date.now();
      if (
        now - lastResizeBorderClickTime < 400 &&
        resizeCol === lastResizeBorderCol
      ) {
        // Double-click: auto-fit column width.
        autoFitColumn(resizeCol);
        lastResizeBorderClickTime = 0;
        lastResizeBorderCol = -1;
        return;
      }
      lastResizeBorderClickTime = now;
      lastResizeBorderCol = resizeCol;

      resizeDrag = {
        kind: 'col',
        index: resizeCol,
        startMouse: e.clientX,
        startSize: getColWidth(resizeCol),
      };
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleResizeMouseUp);
      e.preventDefault();
      return;
    }

    // Check for row header border resize drag.
    const resizeRow = hitRowHeaderBorder(localX, localY);
    if (resizeRow >= 0) {
      const now = Date.now();
      if (
        now - lastResizeBorderClickTime < 400 &&
        resizeRow === lastResizeBorderRow
      ) {
        // Double-click: auto-fit row height.
        autoFitRow(resizeRow);
        lastResizeBorderClickTime = 0;
        lastResizeBorderRow = -1;
        return;
      }
      lastResizeBorderClickTime = now;
      lastResizeBorderRow = resizeRow;

      resizeDrag = {
        kind: 'row',
        index: resizeRow,
        startMouse: e.clientY,
        startSize: getRowHeight(resizeRow),
      };
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleResizeMouseUp);
      e.preventDefault();
      return;
    }

    // Check for fill handle drag.
    if (fillHandleHit(localX, localY)) {
      isFillDragging = true;
      const range = getSelectionRange();
      setFillDragRow(range.maxRow);
      setFillDragCol(range.maxCol);
      document.addEventListener('mousemove', handleFillMouseMove);
      document.addEventListener('mouseup', handleFillMouseUp);
      e.preventDefault();
      return;
    }

    // -------------------------------------------------------------------
    // Header click handling: column headers, row numbers, corner
    // -------------------------------------------------------------------

    // Click on corner (top-left intersection): select all cells
    if (localX < ROW_NUMBER_WIDTH && localY < HEADER_HEIGHT) {
      setSelectedRow(0);
      setSelectedCol(0);
      setRangeAnchor([0, 0]);
      setRangeEnd([TOTAL_ROWS - 1, TOTAL_COLS - 1]);
      selectCell(0, 0);
      draw();
      return;
    }

    // Click on column header: check for filter arrow click first
    if (localY < HEADER_HEIGHT && localX >= ROW_NUMBER_WIDTH) {
      const effSx = effectiveScrollX(localX);
      const contentX = localX - ROW_NUMBER_WIDTH + effSx;
      const col = colAtX(contentX);

      // Check if clicking on filter arrow area
      if (
        props.filterActive &&
        props.filterStartCol !== undefined &&
        props.filterEndCol !== undefined &&
        col >= props.filterStartCol &&
        col <= props.filterEndCol &&
        props.onFilterColumnClick
      ) {
        const cellRight = ROW_NUMBER_WIDTH + getColX(col) + getColWidth(col) - effSx;
        if (localX >= cellRight - 16) {
          // Clicked on the filter arrow -- get screen coordinates
          const rect = containerRef!.getBoundingClientRect();
          props.onFilterColumnClick(col, rect.left + cellRight - getColWidth(col), rect.top + HEADER_HEIGHT);
          e.preventDefault();
          return;
        }
      }
      if (e.shiftKey) {
        // Extend selection from current column to clicked column
        const anchor = rangeAnchor();
        const anchorCol = anchor ? anchor[1] : selectedCol();
        setRangeAnchor([0, anchorCol]);
        setRangeEnd([TOTAL_ROWS - 1, col]);
        propagateSelectionRange();
      } else {
        setSelectedRow(0);
        setSelectedCol(col);
        setRangeAnchor([0, col]);
        setRangeEnd([TOTAL_ROWS - 1, col]);
        selectCell(0, col);
      }
      // Start drag across column headers
      headerDragKind = 'col';
      headerDragStartIndex = col;
      isDragging = true;
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleDragMouseUp);
      draw();
      return;
    }

    // Click on row number: select entire row
    if (localX < ROW_NUMBER_WIDTH && localY >= HEADER_HEIGHT) {
      const effSy = effectiveScrollY(localY);
      const contentY = localY - HEADER_HEIGHT + effSy;
      const row = rowAtY(contentY);
      if (e.shiftKey) {
        // Extend selection from current row to clicked row
        const anchor = rangeAnchor();
        const anchorRow = anchor ? anchor[0] : selectedRow();
        setRangeAnchor([anchorRow, 0]);
        setRangeEnd([row, TOTAL_COLS - 1]);
        propagateSelectionRange();
      } else {
        setSelectedRow(row);
        setSelectedCol(0);
        setRangeAnchor([row, 0]);
        setRangeEnd([row, TOTAL_COLS - 1]);
        selectCell(row, 0);
      }
      // Start drag across row headers
      headerDragKind = 'row';
      headerDragStartIndex = row;
      isDragging = true;
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleDragMouseUp);
      draw();
      return;
    }

    // -------------------------------------------------------------------
    // Normal cell click handling.
    // -------------------------------------------------------------------
    const hit = hitTest(e.clientX, e.clientY);
    if (!hit) return;

    const now = Date.now();
    const isDoubleClick =
      now - lastClickTime < 400 &&
      hit.row === lastClickRow &&
      hit.col === lastClickCol;
    lastClickTime = now;
    lastClickRow = hit.row;
    lastClickCol = hit.col;

    if (e.shiftKey) {
      // Shift+Click: extend range from anchor
      if (!rangeAnchor()) {
        setRangeAnchor([selectedRow(), selectedCol()]);
      }
      setRangeEnd([hit.row, hit.col]);
      propagateSelectionRange();
    } else {
      setSelectedRow(hit.row);
      setSelectedCol(hit.col);
      setRangeAnchor(null);
      setRangeEnd(null);
      selectCell(hit.row, hit.col);

      if (isDoubleClick) {
        startEditing(false, e.clientX, e.clientY);
        return;
      }

      // Start drag-to-select: set anchor and listen for drag
      setRangeAnchor([hit.row, hit.col]);
      setRangeEnd([hit.row, hit.col]);
      headerDragKind = null;
      isDragging = true;
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleDragMouseUp);
    }
    draw();
  }

  function ensureCellVisible(row: number, col: number) {
    const sx = scrollX();
    const sy = scrollY();
    const viewW = canvasWidth() - ROW_NUMBER_WIDTH;
    const viewH = canvasHeight() - HEADER_HEIGHT;

    const cellLeft = getColX(col);
    const cellRight = cellLeft + getColWidth(col);
    if (cellLeft < sx) {
      setScrollX(cellLeft);
    } else if (cellRight > sx + viewW) {
      setScrollX(cellRight - viewW);
    }

    const cellTop = getRowY(row);
    const cellBottom = cellTop + getRowHeight(row);
    if (cellTop < sy) {
      setScrollY(cellTop);
    } else if (cellBottom > sy + viewH) {
      setScrollY(cellBottom - viewH);
    }
  }

  // -----------------------------------------------------------------------
  // Clipboard
  // -----------------------------------------------------------------------

  /** Build a TSV string from the selected range. */
  async function getSelectionTSV(): Promise<string> {
    const range = getSelectionRange();
    const rows: string[] = [];
    for (let r = range.minRow; r <= range.maxRow; r++) {
      const cols: string[] = [];
      for (let c = range.minCol; c <= range.maxCol; c++) {
        const cached = cellCache.get(`${r}:${c}`);
        cols.push(cached?.value ?? '');
      }
      rows.push(cols.join('\t'));
    }
    return rows.join('\n');
  }

  /** Copy selected cells to the clipboard as TSV. */
  async function handleCopy() {
    const tsv = await getSelectionTSV();
    try {
      await navigator.clipboard.writeText(tsv);
      copiedRange = { ...getSelectionRange() };
      startMarchingAnts();
      props.onStatusChange('Copied to clipboard');
    } catch {
      props.onStatusChange('Copy failed');
    }
  }

  /** Cut: copy to clipboard then clear selected cells. */
  async function handleCut() {
    const tsv = await getSelectionTSV();
    try {
      await navigator.clipboard.writeText(tsv);
    } catch {
      props.onStatusChange('Cut failed');
      return;
    }
    await clearSelectedCells();
    props.onStatusChange('Cut to clipboard');
  }

  /** Paste from clipboard: parse TSV and write cells starting at selection. */
  async function handlePaste() {
    let text: string;
    try {
      text = await navigator.clipboard.readText();
    } catch {
      props.onStatusChange('Paste failed — clipboard access denied');
      return;
    }
    if (!text) return;

    const rows = text.split('\n');
    const startRow = selectedRow();
    const startCol = selectedCol();
    const promises: Promise<void>[] = [];

    for (let r = 0; r < rows.length; r++) {
      const cols = rows[r].split('\t');
      for (let c = 0; c < cols.length; c++) {
        const cellRow = startRow + r;
        const cellCol = startCol + c;
        if (cellRow >= TOTAL_ROWS || cellCol >= TOTAL_COLS) continue;
        const value = cols[c];
        let formula: string | undefined;
        if (value.startsWith('=')) {
          formula = value.slice(1);
        }
        promises.push(
          setCell(props.activeSheet, cellRow, cellCol, value, formula).catch(() => {}),
        );
      }
    }

    await Promise.all(promises);
    lastFetchKey = '';
    fetchVisibleData();
    props.onStatusChange('Pasted from clipboard');
  }

  /** Paste values only: strip formulas, treat '=' prefix as text. */
  async function handlePasteValuesOnly() {
    let text: string;
    try {
      text = await navigator.clipboard.readText();
    } catch {
      props.onStatusChange('Paste failed — clipboard access denied');
      return;
    }
    if (!text) return;

    const rows = text.split('\n');
    const startRow = selectedRow();
    const startCol = selectedCol();
    const promises: Promise<void>[] = [];

    for (let r = 0; r < rows.length; r++) {
      const cols = rows[r].split('\t');
      for (let c = 0; c < cols.length; c++) {
        const cellRow = startRow + r;
        const cellCol = startCol + c;
        if (cellRow >= TOTAL_ROWS || cellCol >= TOTAL_COLS) continue;
        const value = cols[c];
        // Never treat as formula — always paste as plain text value
        promises.push(
          setCell(props.activeSheet, cellRow, cellCol, value, undefined).catch(() => {}),
        );
      }
    }

    await Promise.all(promises);
    lastFetchKey = '';
    fetchVisibleData();
    props.onStatusChange('Pasted values only');
  }

  /** Paste from clipboard with rows and columns transposed. */
  async function handlePasteTransposed() {
    let text: string;
    try {
      text = await navigator.clipboard.readText();
    } catch {
      props.onStatusChange('Paste failed — clipboard access denied');
      return;
    }
    if (!text) return;

    const rows = text.split('\n').filter((r) => r.length > 0);
    const startRow = selectedRow();
    const startCol = selectedCol();
    const promises: Promise<void>[] = [];

    for (let r = 0; r < rows.length; r++) {
      const cols = rows[r].split('\t');
      for (let c = 0; c < cols.length; c++) {
        // Transpose: source (r, c) -> dest (startRow + c, startCol + r)
        const cellRow = startRow + c;
        const cellCol = startCol + r;
        if (cellRow >= TOTAL_ROWS || cellCol >= TOTAL_COLS) continue;
        const value = cols[c];
        let formula: string | undefined;
        if (value.startsWith('=')) {
          formula = value.slice(1);
        }
        promises.push(
          setCell(props.activeSheet, cellRow, cellCol, value, formula).catch(() => {}),
        );
      }
    }

    await Promise.all(promises);
    lastFetchKey = '';
    fetchVisibleData();
    props.onStatusChange('Pasted transposed');
  }

  /** Paste from clipboard: only write cells that start with '=' (formulas). */
  async function handlePasteFormulasOnly() {
    let text: string;
    try {
      text = await navigator.clipboard.readText();
    } catch {
      props.onStatusChange('Paste failed — clipboard access denied');
      return;
    }
    if (!text) return;

    const rows = text.split('\n');
    const startRow = selectedRow();
    const startCol = selectedCol();
    const promises: Promise<void>[] = [];

    for (let r = 0; r < rows.length; r++) {
      const cols = rows[r].split('\t');
      for (let c = 0; c < cols.length; c++) {
        const cellRow = startRow + r;
        const cellCol = startCol + c;
        if (cellRow >= TOTAL_ROWS || cellCol >= TOTAL_COLS) continue;
        const value = cols[c];
        // Only write cells that are formulas (start with '=')
        if (value.startsWith('=')) {
          const formula = value.slice(1);
          promises.push(
            setCell(props.activeSheet, cellRow, cellCol, value, formula).catch(() => {}),
          );
        }
      }
    }

    await Promise.all(promises);
    lastFetchKey = '';
    fetchVisibleData();
    props.onStatusChange('Pasted formulas only');
  }

  /** Paste formatting only: read source cells' format from cache and apply via formatCells. */
  async function handlePasteFormattingOnly() {
    // For formatting-only paste, we read the cached cell data from the
    // *source* range (the cells that were last copied). The source range
    // is inferred from the clipboard text dimensions, and we look up the
    // formatting from the cell cache (which contains the data from the
    // cells that were copied, since they were visible on screen).
    //
    // We apply each source cell's bold/italic format to the target range.
    let text: string;
    try {
      text = await navigator.clipboard.readText();
    } catch {
      props.onStatusChange('Paste failed — clipboard access denied');
      return;
    }
    if (!text) return;

    const rows = text.split('\n');
    const startRow = selectedRow();
    const startCol = selectedCol();
    const promises: Promise<void>[] = [];

    for (let r = 0; r < rows.length; r++) {
      const cols = rows[r].split('\t');
      for (let c = 0; c < cols.length; c++) {
        const cellRow = startRow + r;
        const cellCol = startCol + c;
        if (cellRow >= TOTAL_ROWS || cellCol >= TOTAL_COLS) continue;
        // Look up the source cell's format from cache.
        // The source cell is at the same offset from the original selection
        // origin. We check if there's cached data for the source cell.
        const srcKey = `${cellRow}:${cellCol}`;
        const cached = cellCache.get(srcKey);
        // Build format options from source cell
        const format: Record<string, unknown> = {};
        if (cached) {
          format.bold = cached.bold ?? false;
          format.italic = cached.italic ?? false;
        }
        // Apply formatting to the target cell without changing values
        promises.push(
          formatCells(props.activeSheet, cellRow, cellCol, cellRow, cellCol, format).catch(() => {}),
        );
      }
    }

    await Promise.all(promises);
    lastFetchKey = '';
    fetchVisibleData();
    props.onStatusChange('Pasted formatting only');
  }

  /** Clear all cells in the current selection. */
  async function clearSelectedCells() {
    const range = getSelectionRange();
    const promises: Promise<void>[] = [];
    for (let r = range.minRow; r <= range.maxRow; r++) {
      for (let c = range.minCol; c <= range.maxCol; c++) {
        promises.push(setCell(props.activeSheet, r, c, '').catch(() => {}));
      }
    }
    await Promise.all(promises);
    lastFetchKey = '';
    fetchVisibleData();
  }

  // -----------------------------------------------------------------------
  // Context menu
  // -----------------------------------------------------------------------

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    if (!containerRef) return;
    const rect = containerRef.getBoundingClientRect();
    const localX = e.clientX - rect.left;
    const localY = e.clientY - rect.top;

    // Determine click target: row header, column header, or cell area
    const onRowHeader = localX < ROW_NUMBER_WIDTH && localY >= HEADER_HEIGHT;
    const onColHeader = localY < HEADER_HEIGHT && localX >= ROW_NUMBER_WIDTH;

    if (onRowHeader) {
      // Clicked on row number area — identify which row
      const contentY = localY - HEADER_HEIGHT + effectiveScrollY(localY);
      const row = rowAtY(contentY);
      if (row >= 0 && row < TOTAL_ROWS) {
        setSelectedRow(row);
        setRangeAnchor([row, 0]);
        setRangeEnd([row, TOTAL_COLS - 1]);
        propagateSelectionRange();
        draw();
      }
      setCtxMenuTarget('row-header');
    } else if (onColHeader) {
      // Clicked on column header area — identify which column
      const contentX = localX - ROW_NUMBER_WIDTH + effectiveScrollX(localX);
      const col = colAtX(contentX);
      if (col >= 0 && col < TOTAL_COLS) {
        setSelectedCol(col);
        setRangeAnchor([0, col]);
        setRangeEnd([TOTAL_ROWS - 1, col]);
        propagateSelectionRange();
        draw();
      }
      setCtxMenuTarget('col-header');
    } else {
      // Cell area
      const hit = hitTest(e.clientX, e.clientY);
      if (hit) {
        const range = getSelectionRange();
        const inRange =
          hit.row >= range.minRow &&
          hit.row <= range.maxRow &&
          hit.col >= range.minCol &&
          hit.col <= range.maxCol;
        if (!inRange) {
          setSelectedRow(hit.row);
          setSelectedCol(hit.col);
          setRangeAnchor(null);
          setRangeEnd(null);
          selectCell(hit.row, hit.col);
          draw();
        }
      }
      setCtxMenuTarget('cell');
    }
    setCtxMenuX(e.clientX);
    setCtxMenuY(e.clientY);
    setCtxMenuVisible(true);
  }

  function dismissContextMenu() {
    setCtxMenuVisible(false);
  }

  /** Handle selecting a value from the validation dropdown list. */
  async function handleValidationDropdownSelect(value: string) {
    setValidationDropdownVisible(false);
    const row = validationDropdownRow();
    const col = validationDropdownCol();
    try {
      await setCell(props.activeSheet, row, col, value, undefined);
      lastFetchKey = '';
      fetchVisibleData();
      props.onContentChange(value);
      props.onStatusChange(`Set ${value}`);
    } catch {
      props.onStatusChange('Failed to set value');
    }
  }

  async function ctxInsertRowAbove() {
    dismissContextMenu();
    const row = selectedRow();
    try {
      await insertRows(props.activeSheet, row, 1);
      lastFetchKey = '';
      fetchVisibleData();
      props.onStatusChange(`Inserted row above ${row + 1}`);
    } catch {
      props.onStatusChange('Insert row failed');
    }
  }

  async function ctxInsertRowBelow() {
    dismissContextMenu();
    const row = selectedRow();
    try {
      await insertRows(props.activeSheet, row + 1, 1);
      lastFetchKey = '';
      fetchVisibleData();
      props.onStatusChange(`Inserted row below ${row + 1}`);
    } catch {
      props.onStatusChange('Insert row failed');
    }
  }

  async function ctxInsertColLeft() {
    dismissContextMenu();
    const col = selectedCol();
    try {
      await insertCols(props.activeSheet, col, 1);
      lastFetchKey = '';
      fetchVisibleData();
      props.onStatusChange(`Inserted column left of ${col_to_letter(col)}`);
    } catch {
      props.onStatusChange('Insert column failed');
    }
  }

  async function ctxInsertColRight() {
    dismissContextMenu();
    const col = selectedCol();
    try {
      await insertCols(props.activeSheet, col + 1, 1);
      lastFetchKey = '';
      fetchVisibleData();
      props.onStatusChange(`Inserted column right of ${col_to_letter(col)}`);
    } catch {
      props.onStatusChange('Insert column failed');
    }
  }

  async function ctxDeleteRow() {
    dismissContextMenu();
    const row = selectedRow();
    try {
      await deleteRows(props.activeSheet, row, 1);
      lastFetchKey = '';
      fetchVisibleData();
      props.onStatusChange(`Deleted row ${row + 1}`);
    } catch {
      props.onStatusChange('Delete row failed');
    }
  }

  async function ctxDeleteCol() {
    dismissContextMenu();
    const col = selectedCol();
    try {
      await deleteCols(props.activeSheet, col, 1);
      lastFetchKey = '';
      fetchVisibleData();
      props.onStatusChange(`Deleted column ${col_to_letter(col)}`);
    } catch {
      props.onStatusChange('Delete column failed');
    }
  }

  async function ctxClearContents() {
    dismissContextMenu();
    await clearSelectedCells();
    props.onStatusChange('Cleared contents');
  }

  async function ctxHideRow() {
    dismissContextMenu();
    const range = getSelectionRange();
    const count = range.maxRow - range.minRow + 1;
    try {
      await hideRows(props.activeSheet, range.minRow, count);
      await fetchHiddenRows();
      lastFetchKey = '';
      fetchVisibleData();
      draw();
      props.onStatusChange(`Hid ${count} row(s)`);
    } catch {
      props.onStatusChange('Hide row failed');
    }
  }

  async function ctxUnhideRow() {
    dismissContextMenu();
    const row = selectedRow();
    // Unhide rows adjacent to the selected row (one above and one below)
    const start = Math.max(0, row - 1);
    const count = 3;
    try {
      await unhideRows(props.activeSheet, start, count);
      await fetchHiddenRows();
      lastFetchKey = '';
      fetchVisibleData();
      draw();
      props.onStatusChange('Unhid row(s)');
    } catch {
      props.onStatusChange('Unhide row failed');
    }
  }

  async function ctxGroupRows() {
    dismissContextMenu();
    const range = getSelectionRange();
    try {
      await addRowGroup(props.activeSheet, range.minRow, range.maxRow);
      await fetchRowGroups();
      draw();
      props.onStatusChange(`Grouped rows ${range.minRow + 1}-${range.maxRow + 1}`);
    } catch (err) {
      props.onStatusChange(`Group rows failed: ${err}`);
    }
  }

  async function ctxUngroupRows() {
    dismissContextMenu();
    const range = getSelectionRange();
    // Find the row group that contains the selected rows and remove it
    const gi = rowGroups.findIndex(
      (g) => g.start === range.minRow && g.end === range.maxRow,
    );
    if (gi >= 0) {
      try {
        await removeRowGroup(props.activeSheet, gi);
        await fetchRowGroups();
        await fetchHiddenRows();
        lastFetchKey = '';
        fetchVisibleData();
        draw();
        props.onStatusChange(`Ungrouped rows ${range.minRow + 1}-${range.maxRow + 1}`);
      } catch (err) {
        props.onStatusChange(`Ungroup rows failed: ${err}`);
      }
    } else {
      // Try to find any group that overlaps and remove the first match
      const overlapIdx = rowGroups.findIndex(
        (g) => range.minRow <= g.end && range.maxRow >= g.start,
      );
      if (overlapIdx >= 0) {
        try {
          await removeRowGroup(props.activeSheet, overlapIdx);
          await fetchRowGroups();
          await fetchHiddenRows();
          lastFetchKey = '';
          fetchVisibleData();
          draw();
          props.onStatusChange('Ungrouped rows');
        } catch (err) {
          props.onStatusChange(`Ungroup rows failed: ${err}`);
        }
      } else {
        props.onStatusChange('No row group found for selection');
      }
    }
  }

  async function ctxHideCol() {
    dismissContextMenu();
    const range = getSelectionRange();
    const count = range.maxCol - range.minCol + 1;
    try {
      await hideCols(props.activeSheet, range.minCol, count);
      await fetchHiddenCols();
      lastFetchKey = '';
      fetchVisibleData();
      draw();
      props.onStatusChange(`Hid ${count} column(s)`);
    } catch {
      props.onStatusChange('Hide column failed');
    }
  }

  async function ctxUnhideCol() {
    dismissContextMenu();
    const col = selectedCol();
    // Unhide cols adjacent to the selected col
    const start = Math.max(0, col - 1);
    const count = 3;
    try {
      await unhideCols(props.activeSheet, start, count);
      await fetchHiddenCols();
      lastFetchKey = '';
      fetchVisibleData();
      draw();
      props.onStatusChange('Unhid column(s)');
    } catch {
      props.onStatusChange('Unhide column failed');
    }
  }

  async function ctxSortAsc() {
    dismissContextMenu();
    const col = selectedCol();
    try {
      await sortRange(props.activeSheet, null, [{ col, direction: 'asc' }]);
      lastFetchKey = '';
      fetchVisibleData();
      props.onStatusChange(`Sorted by column ${col_to_letter(col)} A\u2192Z`);
    } catch {
      props.onStatusChange('Sort failed');
    }
  }

  async function ctxSortDesc() {
    dismissContextMenu();
    const col = selectedCol();
    try {
      await sortRange(props.activeSheet, null, [{ col, direction: 'desc' }]);
      lastFetchKey = '';
      fetchVisibleData();
      props.onStatusChange(`Sorted by column ${col_to_letter(col)} Z\u2192A`);
    } catch {
      props.onStatusChange('Sort failed');
    }
  }

  // -----------------------------------------------------------------------
  // Undo / Redo
  // -----------------------------------------------------------------------

  /** Undo the last operation via Tauri backend. */
  async function handleUndo() {
    try {
      await undo();
      lastFetchKey = '';
      fetchVisibleData();
      props.onStatusChange('Undo');
    } catch {
      props.onStatusChange('Nothing to undo');
    }
  }

  /** Redo the last undone operation via Tauri backend. */
  async function handleRedo() {
    try {
      await redo();
      lastFetchKey = '';
      fetchVisibleData();
      props.onStatusChange('Redo');
    } catch {
      props.onStatusChange('Nothing to redo');
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    // Dismiss context menu on any key
    if (ctxMenuVisible()) {
      dismissContextMenu();
      if (e.key === 'Escape') {
        e.preventDefault();
        return;
      }
    }

    if (editing()) return; // editor handles its own keys

    // Escape: clear marching ants (copy indicator)
    if (e.key === 'Escape') {
      if (copiedRange) {
        e.preventDefault();
        stopMarchingAnts();
        draw();
        return;
      }
    }

    // F2 enters edit mode without clearing
    if (e.key === 'F2') {
      e.preventDefault();
      startEditing(false);
      return;
    }

    // Ctrl+F3: open named ranges manager
    if (e.key === 'F3' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      props.onNamedRangesOpen?.();
      return;
    }

    // Delete/Backspace clears selected cell(s) (but NOT Cmd+Backspace which scrolls)
    if ((e.key === 'Delete' || e.key === 'Backspace') && !e.metaKey && !e.ctrlKey) {
      e.preventDefault();
      clearSelectedCells();
      return;
    }

    // Cmd+Backspace: scroll to active cell (center if possible) without changing selection
    if (e.key === 'Backspace' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      const row = selectedRow();
      const col = selectedCol();
      const viewW = canvasWidth() - ROW_NUMBER_WIDTH;
      const viewH = canvasHeight() - HEADER_HEIGHT;
      const cellLeft = getColX(col);
      const cellTop = getRowY(row);
      const cellW = getColWidth(col);
      const cellH = getRowHeight(row);
      // Center the active cell in the viewport
      setScrollX(Math.max(0, cellLeft + cellW / 2 - viewW / 2));
      setScrollY(Math.max(0, cellTop + cellH / 2 - viewH / 2));
      draw();
      props.onStatusChange('Scrolled to active cell');
      return;
    }

    // Option+Down / Option+Up: switch sheet tabs
    if (e.altKey && !e.metaKey && !e.ctrlKey && !e.shiftKey) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        props.onNextSheet?.();
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        props.onPrevSheet?.();
        return;
      }
    }

    // Option+Shift+1/2/3/4/6/7: border shortcuts
    if (e.altKey && e.shiftKey && !e.metaKey && !e.ctrlKey) {
      const borderCode = e.code;
      const range = getSelectionRange();
      let borderFormat: FormatOptions | null = null;
      let statusMsg = '';

      if (borderCode === 'Digit1') {
        borderFormat = { borders: { top: { style: 'thin', color: '#000000' } } };
        statusMsg = 'Top border applied';
      } else if (borderCode === 'Digit2') {
        borderFormat = { borders: { right: { style: 'thin', color: '#000000' } } };
        statusMsg = 'Right border applied';
      } else if (borderCode === 'Digit3') {
        borderFormat = { borders: { bottom: { style: 'thin', color: '#000000' } } };
        statusMsg = 'Bottom border applied';
      } else if (borderCode === 'Digit4') {
        borderFormat = { borders: { left: { style: 'thin', color: '#000000' } } };
        statusMsg = 'Left border applied';
      } else if (borderCode === 'Digit6') {
        borderFormat = { borders: {
          top: { style: 'none' },
          right: { style: 'none' },
          bottom: { style: 'none' },
          left: { style: 'none' },
        } };
        statusMsg = 'All borders removed';
      } else if (borderCode === 'Digit7') {
        borderFormat = { borders: {
          top: { style: 'thin', color: '#000000' },
          right: { style: 'thin', color: '#000000' },
          bottom: { style: 'thin', color: '#000000' },
          left: { style: 'thin', color: '#000000' },
        } };
        statusMsg = 'Outer border applied';
      }

      if (borderFormat) {
        e.preventDefault();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol,
          borderFormat,
        ).catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange(statusMsg);
        return;
      }
    }

    // Typing a printable character starts edit mode (clearing old content)
    if (e.key.length === 1 && !e.metaKey && !e.ctrlKey && !e.altKey) {
      startEditing(true);
      // Let the character be typed into the editor
      setEditValue(e.key);
      props.onContentChange(e.key);
      return;
    }

    // Ctrl+Space: select entire column
    if (e.key === ' ' && e.ctrlKey && !e.metaKey && !e.shiftKey) {
      e.preventDefault();
      const col = selectedCol();
      setRangeAnchor([0, col]);
      setRangeEnd([TOTAL_ROWS - 1, col]);
      propagateSelectionRange();
      props.onStatusChange(`Column ${col_to_letter(col)} selected`);
      draw();
      return;
    }

    // Shift+Space: select entire row
    if (e.key === ' ' && e.shiftKey && !e.metaKey && !e.ctrlKey) {
      e.preventDefault();
      const row = selectedRow();
      setRangeAnchor([row, 0]);
      setRangeEnd([row, TOTAL_COLS - 1]);
      propagateSelectionRange();
      props.onStatusChange(`Row ${row + 1} selected`);
      draw();
      return;
    }

    // Keyboard shortcuts (Cmd/Ctrl + key)
    if (e.metaKey || e.ctrlKey) {
      // Cmd+Option+9: hide selected rows
      if (e.key === '9' && e.altKey && !e.shiftKey) {
        e.preventDefault();
        const range = getSelectionRange();
        const count = range.maxRow - range.minRow + 1;
        void hideRows(props.activeSheet, range.minRow, count).then(() => {
          fetchHiddenRows();
          lastFetchKey = '';
          fetchVisibleData();
          draw();
          props.onStatusChange(`Hid ${count} row(s)`);
        }).catch(() => props.onStatusChange('Hide row failed'));
        return;
      }
      // Cmd+Shift+9: unhide rows
      if (e.key === '(' || (e.key === '9' && e.shiftKey && !e.altKey)) {
        e.preventDefault();
        const row = selectedRow();
        const start = Math.max(0, row - 1);
        void unhideRows(props.activeSheet, start, 3).then(() => {
          fetchHiddenRows();
          lastFetchKey = '';
          fetchVisibleData();
          draw();
          props.onStatusChange('Unhid row(s)');
        }).catch(() => props.onStatusChange('Unhide row failed'));
        return;
      }
      // Cmd+Option+0: hide selected columns
      if (e.key === '0' && e.altKey && !e.shiftKey) {
        e.preventDefault();
        const range = getSelectionRange();
        const count = range.maxCol - range.minCol + 1;
        void hideCols(props.activeSheet, range.minCol, count).then(() => {
          fetchHiddenCols();
          lastFetchKey = '';
          fetchVisibleData();
          draw();
          props.onStatusChange(`Hid ${count} column(s)`);
        }).catch(() => props.onStatusChange('Hide column failed'));
        return;
      }
      // Cmd+Shift+0: unhide columns
      if (e.key === ')' || (e.key === '0' && e.shiftKey && !e.altKey)) {
        e.preventDefault();
        const col = selectedCol();
        const start = Math.max(0, col - 1);
        void unhideCols(props.activeSheet, start, 3).then(() => {
          fetchHiddenCols();
          lastFetchKey = '';
          fetchVisibleData();
          draw();
          props.onStatusChange('Unhid column(s)');
        }).catch(() => props.onStatusChange('Unhide column failed'));
        return;
      }
      // Cmd+1: open Format Cells dialog
      if (e.key === '1' && !e.shiftKey && !e.altKey) {
        e.preventDefault();
        props.onFormatCellsOpen?.();
        return;
      }
      if (e.key === 'b') {
        e.preventDefault();
        props.onBoldToggle();
        return;
      }
      if (e.key === 'i') {
        e.preventDefault();
        props.onItalicToggle();
        return;
      }
      if (e.key === 'u') {
        e.preventDefault();
        props.onUnderlineToggle();
        return;
      }
      // Cmd+\: clear formatting — reset all format fields to defaults
      if (e.key === '\\') {
        e.preventDefault();
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, {
          bold: false,
          italic: false,
          underline: false,
          strikethrough: false,
          font_size: 11,
          font_family: 'Arial',
          font_color: '#000000',
          bg_color: '',
          h_align: 'left',
          number_format: 'General',
          borders: {
            top: { style: 'none' },
            bottom: { style: 'none' },
            left: { style: 'none' },
            right: { style: 'none' },
          },
        }).catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange('Formatting cleared');
        return;
      }
      // Copy
      if (e.key === 'c') {
        e.preventDefault();
        handleCopy();
        return;
      }
      // Cut
      if (e.key === 'x') {
        e.preventDefault();
        handleCut();
        return;
      }
      // Cmd+P: open print preview dialog
      if (e.key === 'p' && !e.shiftKey) {
        e.preventDefault();
        props.onPrintPreviewOpen?.();
        return;
      }
      // Cmd+Shift+P: open paste special dialog
      if ((e.key === 'p' || e.key === 'P') && e.shiftKey) {
        e.preventDefault();
        props.onPasteSpecialOpen?.();
        return;
      }
      // Cmd+Shift+V: paste values only (strip formulas)
      if (e.key === 'v' && e.shiftKey) {
        e.preventDefault();
        handlePasteValuesOnly();
        return;
      }
      // Paste
      if (e.key === 'v') {
        e.preventDefault();
        handlePaste();
        return;
      }
      // Undo: Cmd+Z (without Shift)
      if (e.key === 'z' && !e.shiftKey) {
        e.preventDefault();
        handleUndo();
        return;
      }
      // Redo: Cmd+Shift+Z or Cmd+Y
      if ((e.key === 'z' && e.shiftKey) || (e.key === 'Z' && e.shiftKey) || e.key === 'y') {
        e.preventDefault();
        handleRedo();
        return;
      }
      // Cmd+A: select all cells
      if (e.key === 'a') {
        e.preventDefault();
        setRangeAnchor([0, 0]);
        setRangeEnd([TOTAL_ROWS - 1, TOTAL_COLS - 1]);
        propagateSelectionRange();
        props.onStatusChange('All cells selected');
        draw();
        return;
      }
      // Cmd+F: open find dialog
      if (e.key === 'f' && !e.shiftKey) {
        e.preventDefault();
        props.onFindOpen?.();
        return;
      }
      // Cmd+H: open find & replace dialog
      if (e.key === 'h') {
        e.preventDefault();
        props.onFindReplaceOpen?.();
        return;
      }
      // Cmd+;: insert current date
      if (e.key === ';' && !e.shiftKey) {
        e.preventDefault();
        const today = new Date();
        const mm = String(today.getMonth() + 1).padStart(2, '0');
        const dd = String(today.getDate()).padStart(2, '0');
        const yyyy = today.getFullYear();
        const dateStr = `${mm}/${dd}/${yyyy}`;
        setCell(props.activeSheet, selectedRow(), selectedCol(), dateStr, undefined)
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onContentChange(dateStr);
        props.onStatusChange(`Inserted date: ${dateStr}`);
        return;
      }
      // Cmd+Shift+;: insert current time
      if (e.key === ':' || (e.key === ';' && e.shiftKey)) {
        e.preventDefault();
        const now = new Date();
        const hh = String(now.getHours()).padStart(2, '0');
        const mm = String(now.getMinutes()).padStart(2, '0');
        const ss = String(now.getSeconds()).padStart(2, '0');
        const timeStr = `${hh}:${mm}:${ss}`;
        setCell(props.activeSheet, selectedRow(), selectedCol(), timeStr, undefined)
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onContentChange(timeStr);
        props.onStatusChange(`Inserted time: ${timeStr}`);
        return;
      }
      // Cmd+D: fill down — copy first row of selection into all rows below
      if (e.key === 'd' && !e.shiftKey) {
        e.preventDefault();
        const range = getSelectionRange();
        const promises: Promise<void>[] = [];
        for (let c = range.minCol; c <= range.maxCol; c++) {
          const source = cellCache.get(`${range.minRow}:${c}`);
          const value = source?.formula ? `=${source.formula}` : source?.value ?? '';
          const formula = source?.formula ?? undefined;
          for (let r = range.minRow + 1; r <= range.maxRow; r++) {
            promises.push(
              setCell(props.activeSheet, r, c, value, formula).catch(() => {}),
            );
          }
          // If single row selected and there's a row above, fill from above
          if (range.minRow === range.maxRow && range.minRow > 0) {
            const above = cellCache.get(`${range.minRow - 1}:${c}`);
            const aboveVal = above?.formula ? `=${above.formula}` : above?.value ?? '';
            const aboveFormula = above?.formula ?? undefined;
            promises.push(
              setCell(props.activeSheet, range.minRow, c, aboveVal, aboveFormula).catch(() => {}),
            );
          }
        }
        void Promise.all(promises).then(() => {
          lastFetchKey = '';
          fetchVisibleData();
        });
        props.onStatusChange('Filled down');
        return;
      }
      // Cmd+R: fill right — copy first column of selection into all columns right
      if (e.key === 'r' && !e.shiftKey) {
        e.preventDefault();
        const range = getSelectionRange();
        const promises: Promise<void>[] = [];
        for (let r = range.minRow; r <= range.maxRow; r++) {
          const source = cellCache.get(`${r}:${range.minCol}`);
          const value = source?.formula ? `=${source.formula}` : source?.value ?? '';
          const formula = source?.formula ?? undefined;
          for (let c = range.minCol + 1; c <= range.maxCol; c++) {
            promises.push(
              setCell(props.activeSheet, r, c, value, formula).catch(() => {}),
            );
          }
          // If single column selected and there's a column to the left, fill from left
          if (range.minCol === range.maxCol && range.minCol > 0) {
            const left = cellCache.get(`${r}:${range.minCol - 1}`);
            const leftVal = left?.formula ? `=${left.formula}` : left?.value ?? '';
            const leftFormula = left?.formula ?? undefined;
            promises.push(
              setCell(props.activeSheet, r, range.minCol, leftVal, leftFormula).catch(() => {}),
            );
          }
        }
        void Promise.all(promises).then(() => {
          lastFetchKey = '';
          fetchVisibleData();
        });
        props.onStatusChange('Filled right');
        return;
      }
      // Cmd+Shift+K: strikethrough toggle
      if (e.key === 'K' || (e.key === 'k' && e.shiftKey)) {
        e.preventDefault();
        const row = selectedRow();
        const col = selectedCol();
        const cell = cellCache.get(`${row}:${col}`);
        const newStrike = !(cell?.strikethrough ?? false);
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { strikethrough: newStrike })
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange(newStrike ? 'Strikethrough on' : 'Strikethrough off');
        return;
      }
      // Cmd+Shift+T: paste transposed
      if (e.key === 'T' || (e.key === 't' && e.shiftKey)) {
        e.preventDefault();
        handlePasteTransposed();
        return;
      }
      // Cmd+': copy formula from cell above (without evaluating)
      if (e.key === "'") {
        e.preventDefault();
        const row = selectedRow();
        const col = selectedCol();
        if (row > 0) {
          const above = cellCache.get(`${row - 1}:${col}`);
          if (above?.formula) {
            const formulaText = `=${above.formula}`;
            setCell(props.activeSheet, row, col, formulaText, above.formula).catch(() => {});
            lastFetchKey = '';
            fetchVisibleData();
            props.onContentChange(formulaText);
            props.onStatusChange('Copied formula from above');
          }
        }
        return;
      }
      // Cmd+Shift+E: center align
      if (e.key === 'E' || (e.key === 'e' && e.shiftKey)) {
        e.preventDefault();
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { h_align: 'center' })
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange('Align: center');
        return;
      }
      // Cmd+Shift+L: left align
      if (e.key === 'L' || (e.key === 'l' && e.shiftKey)) {
        e.preventDefault();
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { h_align: 'left' })
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange('Align: left');
        return;
      }
      // Cmd+Shift+R: right align (must come after Cmd+R which is not shifted)
      if (e.key === 'R' || (e.key === 'r' && e.shiftKey)) {
        e.preventDefault();
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { h_align: 'right' })
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange('Align: right');
        return;
      }
      // Cmd+K: insert hyperlink
      if (e.key === 'k' && !e.shiftKey) {
        e.preventDefault();
        const url = window.prompt('Enter URL:');
        if (url) {
          const row = selectedRow();
          const col = selectedCol();
          setCell(props.activeSheet, row, col, url, undefined)
            .catch(() => {});
          lastFetchKey = '';
          fetchVisibleData();
          props.onContentChange(url);
          props.onStatusChange(`Link: ${url}`);
        }
        return;
      }
      // Cmd+Shift+.: increase font size
      if (e.key === '>' || (e.key === '.' && e.shiftKey)) {
        e.preventDefault();
        const row = selectedRow();
        const col = selectedCol();
        const cell = cellCache.get(`${row}:${col}`);
        const currentSize = cell?.font_size ?? 11;
        const newSize = Math.min(72, currentSize + 1);
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { font_size: newSize })
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange(`Font size: ${newSize}`);
        return;
      }
      // Cmd+Shift+,: decrease font size
      if (e.key === '<' || (e.key === ',' && e.shiftKey)) {
        e.preventDefault();
        const row = selectedRow();
        const col = selectedCol();
        const cell = cellCache.get(`${row}:${col}`);
        const currentSize = cell?.font_size ?? 11;
        const newSize = Math.max(1, currentSize - 1);
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { font_size: newSize })
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange(`Font size: ${newSize}`);
        return;
      }
      // Cmd+/: open keyboard shortcuts help dialog
      if (e.key === '/') {
        e.preventDefault();
        props.onKeyboardShortcutsOpen?.();
        return;
      }
      // Cmd+Shift+Home: extend selection from anchor to A1
      if (e.key === 'Home' && e.shiftKey) {
        e.preventDefault();
        if (!rangeAnchor()) {
          setRangeAnchor([selectedRow(), selectedCol()]);
        }
        setRangeEnd([0, 0]);
        propagateSelectionRange();
        ensureCellVisible(0, 0);
        draw();
        return;
      }
      // Cmd+Shift+End: extend selection from anchor to last used cell
      if (e.key === 'End' && e.shiftKey) {
        e.preventDefault();
        let maxR = 0;
        let maxC = 0;
        cellCache.forEach((_v, key) => {
          const [r, c] = key.split(':').map(Number);
          if (r > maxR) maxR = r;
          if (c > maxC) maxC = c;
        });
        if (!rangeAnchor()) {
          setRangeAnchor([selectedRow(), selectedCol()]);
        }
        setRangeEnd([maxR, maxC]);
        propagateSelectionRange();
        ensureCellVisible(maxR, maxC);
        draw();
        return;
      }
      // Cmd+Home: go to cell A1
      if (e.key === 'Home') {
        e.preventDefault();
        setSelectedRow(0);
        setSelectedCol(0);
        setRangeAnchor(null);
        setRangeEnd(null);
        selectCell(0, 0);
        ensureCellVisible(0, 0);
        draw();
        return;
      }
      // Cmd+End: go to last used cell
      if (e.key === 'End') {
        e.preventDefault();
        let maxR = 0;
        let maxC = 0;
        cellCache.forEach((_v, key) => {
          const [r, c] = key.split(':').map(Number);
          if (r > maxR) maxR = r;
          if (c > maxC) maxC = c;
        });
        setSelectedRow(maxR);
        setSelectedCol(maxC);
        setRangeAnchor(null);
        setRangeEnd(null);
        selectCell(maxR, maxC);
        ensureCellVisible(maxR, maxC);
        draw();
        return;
      }
      // Cmd+Enter: commit edit but stay in cell (handled in editor, noop here)
      if (e.key === 'Enter') {
        e.preventDefault();
        return;
      }
      // Zoom: Cmd+= to zoom in, Cmd+- to zoom out, Cmd+0 to reset
      if (e.key === '=' || e.key === '+') {
        e.preventDefault();
        props.onZoomIn?.();
        return;
      }
      if (e.key === '-') {
        e.preventDefault();
        props.onZoomOut?.();
        return;
      }
      if (e.key === '0') {
        e.preventDefault();
        props.onZoomReset?.();
        return;
      }

      // Cmd+Arrow / Cmd+Shift+Arrow: jump to edge of data region
      if (
        e.key === 'ArrowUp' ||
        e.key === 'ArrowDown' ||
        e.key === 'ArrowLeft' ||
        e.key === 'ArrowRight'
      ) {
        e.preventDefault();
        const curEnd = rangeEnd();
        const curRow = curEnd ? curEnd[0] : selectedRow();
        const curCol = curEnd ? curEnd[1] : selectedCol();
        let targetRow = curRow;
        let targetCol = curCol;

        if (e.key === 'ArrowDown') {
          targetRow = jumpDown(curRow, curCol);
        } else if (e.key === 'ArrowUp') {
          targetRow = jumpUp(curRow, curCol);
        } else if (e.key === 'ArrowRight') {
          targetCol = jumpRight(curRow, curCol);
        } else if (e.key === 'ArrowLeft') {
          targetCol = jumpLeft(curRow, curCol);
        }

        if (e.shiftKey) {
          // Cmd+Shift+Arrow: extend selection to edge
          if (!rangeAnchor()) {
            setRangeAnchor([selectedRow(), selectedCol()]);
          }
          setRangeEnd([targetRow, targetCol]);
          propagateSelectionRange();
        } else {
          // Cmd+Arrow: jump without selection
          setSelectedRow(targetRow);
          setSelectedCol(targetCol);
          setRangeAnchor(null);
          setRangeEnd(null);
          selectCell(targetRow, targetCol);
        }
        ensureCellVisible(targetRow, targetCol);
        draw();
        return;
      }
    }

    // Ctrl-only shortcuts (Ctrl key, not Cmd on macOS)
    if (e.ctrlKey && !e.metaKey) {
      // Ctrl+Shift+1: apply number format
      if (e.key === '!' || (e.key === '1' && e.shiftKey)) {
        e.preventDefault();
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { number_format: '#,##0.00' })
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange('Number format applied');
        return;
      }
      // Ctrl+Shift+3: apply date format
      if (e.key === '#' || (e.key === '3' && e.shiftKey)) {
        e.preventDefault();
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { number_format: 'MM/DD/YYYY' })
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange('Date format applied');
        return;
      }
      // Ctrl+Shift+4: apply currency format
      if (e.key === '$' || (e.key === '4' && e.shiftKey)) {
        e.preventDefault();
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { number_format: '$#,##0.00' })
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange('Currency format applied');
        return;
      }
      // Ctrl+Shift+5: apply percentage format
      if (e.key === '%' || (e.key === '5' && e.shiftKey)) {
        e.preventDefault();
        const range = getSelectionRange();
        formatCells(props.activeSheet, range.minRow, range.minCol, range.maxRow, range.maxCol, { number_format: '0%' })
          .catch(() => {});
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange('Percentage format applied');
        return;
      }
      // Ctrl+`: toggle formula view
      if (e.key === '`') {
        e.preventDefault();
        setShowFormulas(!showFormulas());
        lastFetchKey = '';
        fetchVisibleData();
        props.onStatusChange(showFormulas() ? 'Formula view on' : 'Formula view off');
        return;
      }
    }

    // Page Down / Page Up: scroll by one viewport height and move selection
    if (e.key === 'PageDown' || e.key === 'PageUp') {
      e.preventDefault();
      // Compute the number of rows that fit in the viewport (without buffer)
      const viewH = canvasHeight() - HEADER_HEIGHT;
      let pageRows = Math.max(1, Math.floor(viewH / DEFAULT_ROW_HEIGHT));

      const curEnd = rangeEnd();
      let row = curEnd ? curEnd[0] : selectedRow();
      const col = curEnd ? curEnd[1] : selectedCol();

      if (e.key === 'PageDown') {
        row = Math.min(TOTAL_ROWS - 1, row + pageRows);
      } else {
        row = Math.max(0, row - pageRows);
      }

      if (e.shiftKey) {
        // Shift+PageDown/PageUp: extend selection
        if (!rangeAnchor()) {
          setRangeAnchor([selectedRow(), selectedCol()]);
        }
        setRangeEnd([row, col]);
        propagateSelectionRange();
      } else {
        setSelectedRow(row);
        setSelectedCol(col);
        setRangeAnchor(null);
        setRangeEnd(null);
        selectCell(row, col);
      }
      ensureCellVisible(row, col);
      draw();
      return;
    }

    // Arrow keys (with optional Shift for range extension)
    if (
      e.key === 'ArrowUp' ||
      e.key === 'ArrowDown' ||
      e.key === 'ArrowLeft' ||
      e.key === 'ArrowRight'
    ) {
      e.preventDefault();
      // When extending, start from the current cursor (rangeEnd) not the anchor
      const curEnd = rangeEnd();
      let row = curEnd ? curEnd[0] : selectedRow();
      let col = curEnd ? curEnd[1] : selectedCol();

      if (e.key === 'ArrowUp') row = Math.max(0, row - 1);
      else if (e.key === 'ArrowDown') row = Math.min(TOTAL_ROWS - 1, row + 1);
      else if (e.key === 'ArrowLeft') col = Math.max(0, col - 1);
      else if (e.key === 'ArrowRight') col = Math.min(TOTAL_COLS - 1, col + 1);

      if (e.shiftKey) {
        // Shift+Arrow: extend selection from anchor
        if (!rangeAnchor()) {
          setRangeAnchor([selectedRow(), selectedCol()]);
        }
        setRangeEnd([row, col]);
        propagateSelectionRange();
      } else {
        // Plain arrow: move active cell, clear selection
        setSelectedRow(row);
        setSelectedCol(col);
        setRangeAnchor(null);
        setRangeEnd(null);
        selectCell(row, col);
      }
      ensureCellVisible(row, col);
      draw();
      return;
    }

    let row = selectedRow();
    let col = selectedCol();
    let handled = false;

    switch (e.key) {
      case 'Tab':
        e.preventDefault();
        col = Math.max(0, Math.min(TOTAL_COLS - 1, col + (e.shiftKey ? -1 : 1)));
        handled = true;
        break;
      case 'Enter':
        row = Math.max(0, Math.min(TOTAL_ROWS - 1, row + (e.shiftKey ? -1 : 1)));
        handled = true;
        break;
      case 'Home':
        // Home (without Cmd): go to column A in current row
        col = 0;
        handled = true;
        break;
      case 'End': {
        // End (without Cmd): go to last used column in current row
        let maxCol = 0;
        cellCache.forEach((_v, key) => {
          const parts = key.split(':');
          const r = Number(parts[0]);
          const c = Number(parts[1]);
          if (r === row && c > maxCol) maxCol = c;
        });
        col = maxCol;
        handled = true;
        break;
      }
    }

    if (handled) {
      e.preventDefault();
      setSelectedRow(row);
      setSelectedCol(col);
      setRangeAnchor(null);
      setRangeEnd(null);
      selectCell(row, col);
      ensureCellVisible(row, col);
      draw();
    }
  }

  // -----------------------------------------------------------------------
  // Scroll
  // -----------------------------------------------------------------------

  let rafPending = false;
  function scheduleDraw() {
    if (rafPending) return;
    rafPending = true;
    requestAnimationFrame(() => {
      rafPending = false;
      draw();
    });
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    const maxX = Math.max(0, totalContentWidth() - canvasWidth());
    const maxY = Math.max(0, totalContentHeight() - canvasHeight());

    // When split panes are active, route scroll to the correct pane
    if (isSplit() && containerRef) {
      const rect = containerRef.getBoundingClientRect();
      const localX = e.clientX - rect.left;
      const localY = e.clientY - rect.top;
      const sc = props.splitCol ?? 0;
      const sr = props.splitRow ?? 0;
      const inLeftPane = sc > 0 && localX < ROW_NUMBER_WIDTH + splitColsPx();
      const inTopPane = sr > 0 && localY < HEADER_HEIGHT + splitRowsPx();

      if (inLeftPane) {
        setSplitScrollX(Math.max(0, Math.min(maxX, splitScrollX() + e.deltaX)));
      } else {
        setScrollX(Math.max(0, Math.min(maxX, scrollX() + e.deltaX)));
      }
      if (inTopPane) {
        setSplitScrollY(Math.max(0, Math.min(maxY, splitScrollY() + e.deltaY)));
      } else {
        setScrollY(Math.max(0, Math.min(maxY, scrollY() + e.deltaY)));
      }
    } else {
      setScrollX(Math.max(0, Math.min(maxX, scrollX() + e.deltaX)));
      setScrollY(Math.max(0, Math.min(maxY, scrollY() + e.deltaY)));
    }

    scheduleDraw();
    fetchVisibleData();
  }

  // -----------------------------------------------------------------------
  // Lifecycle
  // -----------------------------------------------------------------------

  onMount(() => {
    if (!containerRef) return;

    const updateSize = () => {
      if (!containerRef) return;
      const rect = containerRef.getBoundingClientRect();
      setCanvasWidth(rect.width);
      setCanvasHeight(rect.height);
      draw();
    };

    const observer = new ResizeObserver(updateSize);
    observer.observe(containerRef);
    onCleanup(() => observer.disconnect());

    // Listen for system dark/light mode changes and re-render the canvas.
    const darkModeQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleColorSchemeChange = () => {
      COLORS = getColors();
      draw();
    };
    darkModeQuery.addEventListener('change', handleColorSchemeChange);
    onCleanup(() => darkModeQuery.removeEventListener('change', handleColorSchemeChange));

    // Clean up drag listeners and marching ants on unmount
    onCleanup(() => {
      stopMarchingAnts();
      if (isDragging) {
        isDragging = false;
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleDragMouseUp);
      }
      if (resizeDrag) {
        resizeDrag = null;
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleResizeMouseUp);
      }
      if (isFillDragging) {
        isFillDragging = false;
        document.removeEventListener('mousemove', handleFillMouseMove);
        document.removeEventListener('mouseup', handleFillMouseUp);
      }
    });

    // Close context menu on window blur
    const handleWindowBlur = () => {
      if (ctxMenuVisible()) dismissContextMenu();
    };
    window.addEventListener('blur', handleWindowBlur);
    onCleanup(() => window.removeEventListener('blur', handleWindowBlur));

    updateSize();
    containerRef.focus();

    // Load persisted col/row sizes from backend
    loadPersistedSizes();
    // Load validation rules, hidden rows, hidden cols, conditional formats, and row groups
    fetchValidations();
    fetchHiddenRows();
    fetchHiddenCols();
    fetchConditionalFormats();
    fetchRowGroups();

    // Initial data fetch + formula bar sync
    fetchVisibleData();
    selectCell(0, 0);
  });

  // Re-fetch data when active sheet changes or external edits occur
  createEffect(() => {
    // Access props to subscribe to reactive changes
    void props.activeSheet;
    void props.refreshTrigger;
    // Invalidate and refetch
    lastFetchKey = '';
    fetchVisibleData();
    // Reload persisted sizes and validations for new sheet
    loadPersistedSizes();
    fetchValidations();
    fetchHiddenRows();
    fetchHiddenCols();
    fetchConditionalFormats();
    fetchRowGroups();
  });

  // Handle external navigation requests (name box, find bar)
  createEffect(() => {
    const nav = props.navigateTo;
    if (!nav) return;
    const [row, col, anchorRow, anchorCol, endRow, endCol] = nav;
    setSelectedRow(row);
    setSelectedCol(col);
    setRangeAnchor([anchorRow, anchorCol]);
    setRangeEnd([endRow, endCol]);
    ensureCellVisible(row, col);
    fetchVisibleData();
    draw();
  });

  // Sync find match highlights from props to canvas-accessible state
  createEffect(() => {
    const matches = props.findMatches;
    const activeIdx = props.findActiveIndex ?? -1;
    if (matches && matches.length > 0) {
      findMatchSet = new Set(matches.map((m) => `${m.row}:${m.col}`));
      if (activeIdx >= 0 && activeIdx < matches.length) {
        findActiveRow = matches[activeIdx].row;
        findActiveCol = matches[activeIdx].col;
      } else {
        findActiveRow = -1;
        findActiveCol = -1;
      }
    } else {
      findMatchSet = null;
      findActiveRow = -1;
      findActiveCol = -1;
    }
    scheduleDraw();
  });

  // Execute paste special when mode is set from the dialog
  createEffect(() => {
    const mode = props.pasteSpecialMode;
    if (!mode) return;
    const run = async () => {
      switch (mode) {
        case 'All':
          await handlePaste();
          break;
        case 'ValuesOnly':
          await handlePasteValuesOnly();
          break;
        case 'FormulasOnly':
          await handlePasteFormulasOnly();
          break;
        case 'FormattingOnly':
          await handlePasteFormattingOnly();
          break;
        case 'Transposed':
          await handlePasteTransposed();
          break;
      }
      props.onPasteSpecialDone?.();
    };
    void run();
  });

  return (
    <div
      ref={containerRef}
      class="virtual-grid-container"
      tabIndex={0}
      role="grid"
      aria-label="Spreadsheet grid"
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onKeyDown={handleKeyDown}
      onWheel={handleWheel}
      onContextMenu={handleContextMenu}
      style={{ outline: 'none' }}
    >
      <canvas ref={canvasRef} class="virtual-grid-canvas" aria-hidden="true" />
      <Show when={editing()}>
        <textarea
          ref={editorRef}
          class="cell-editor-textarea"
          value={editValue()}
          onInput={(e) => handleEditorInput(e.currentTarget.value)}
          onKeyDown={handleEditorKeyDown}
          onKeyUp={() => {
            if (editorRef) setEditorCursorPos(editorRef.selectionStart ?? 0);
          }}
          onClick={() => {
            if (editorRef) setEditorCursorPos(editorRef.selectionStart ?? 0);
          }}
          onBlur={() => {
            // Don't commit when clicking the canvas in formula selection mode
            if (editing() && isFormulaSelectionMode()) {
              // Re-focus the editor after a short delay so the user can continue editing
              requestAnimationFrame(() => editorRef?.focus());
              return;
            }
            if (editing()) {
              commitEdit(0, 0);
            }
          }}
          style={editorStyle()}
          rows={1}
        />
        <AutoComplete
          inputValue={editValue()}
          suggestions={acSuggestions()}
          left={(() => {
            const col = selectedCol();
            const fc = props.frozenCols ?? 0;
            const sx = col < fc ? 0 : scrollX();
            return ROW_NUMBER_WIDTH + getColX(col) - sx;
          })()}
          top={(() => {
            const row = selectedRow();
            const fr = props.frozenRows ?? 0;
            const sy = row < fr ? 0 : scrollY();
            return HEADER_HEIGHT + getRowY(row) - sy + getRowHeight(row);
          })()}
          width={getColWidth(selectedCol())}
          visible={acVisible()}
          onAccept={acceptAutoComplete}
          onDismiss={() => setAcVisible(false)}
        />
        <FormulaAutoComplete
          inputValue={editValue()}
          left={(() => {
            const col = selectedCol();
            const fc = props.frozenCols ?? 0;
            const sx = col < fc ? 0 : scrollX();
            return ROW_NUMBER_WIDTH + getColX(col) - sx;
          })()}
          top={(() => {
            const row = selectedRow();
            const fr = props.frozenRows ?? 0;
            const sy = row < fr ? 0 : scrollY();
            return HEADER_HEIGHT + getRowY(row) - sy + getRowHeight(row);
          })()}
          width={getColWidth(selectedCol())}
          visible={formulaAcVisible()}
          selectedIndex={formulaAcSelectedIdx()}
          onAccept={acceptFormulaAutoComplete}
          onDismiss={() => setFormulaAcVisible(false)}
        />
        <FormulaHint
          inputValue={editValue()}
          cursorPos={editorCursorPos()}
          left={(() => {
            const col = selectedCol();
            const fc = props.frozenCols ?? 0;
            const sx = col < fc ? 0 : scrollX();
            return ROW_NUMBER_WIDTH + getColX(col) - sx;
          })()}
          top={(() => {
            const row = selectedRow();
            const fr = props.frozenRows ?? 0;
            const sy = row < fr ? 0 : scrollY();
            return HEADER_HEIGHT + getRowY(row) - sy + getRowHeight(row) + 20;
          })()}
          visible={editValue().startsWith('=') && !formulaAcVisible()}
        />
      </Show>
      <Show when={ctxMenuVisible()}>
        <div
          class="sheet-tab-context-menu"
          style={{
            left: `${ctxMenuX()}px`,
            top: `${ctxMenuY()}px`,
          }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <div
            class="context-menu-item"
            onClick={() => { dismissContextMenu(); handleCut(); }}
          >
            <span>Cut</span><span class="context-menu-shortcut">{'\u2318'}X</span>
          </div>
          <div
            class="context-menu-item"
            onClick={() => { dismissContextMenu(); handleCopy(); }}
          >
            <span>Copy</span><span class="context-menu-shortcut">{'\u2318'}C</span>
          </div>
          <div
            class="context-menu-item"
            onClick={() => { dismissContextMenu(); handlePaste(); }}
          >
            <span>Paste</span><span class="context-menu-shortcut">{'\u2318'}V</span>
          </div>
          <div
            class="context-menu-item"
            onClick={() => { dismissContextMenu(); props.onPasteSpecialOpen?.(); }}
          >
            Paste special...
          </div>
          <div class="context-menu-separator" />
          <div
            class="context-menu-item"
            onClick={() => {
              dismissContextMenu();
              const url = window.prompt('Enter URL:');
              if (url) {
                const row = selectedRow();
                const col = selectedCol();
                setCell(props.activeSheet, row, col, url, undefined).catch(() => {});
                lastFetchKey = '';
                fetchVisibleData();
                props.onContentChange(url);
                props.onStatusChange(`Link: ${url}`);
              }
            }}
          >
            <span>Insert link</span><span class="context-menu-shortcut">{'\u2318'}K</span>
          </div>
          <div
            class="context-menu-item"
            onClick={() => {
              dismissContextMenu();
              const note = window.prompt('Enter note:');
              if (note !== null) {
                const row = selectedRow();
                const col = selectedCol();
                const cell = cellCache.get(`${row}:${col}`);
                const currentVal = cell?.value ?? '';
                // Store note as a cell comment (append to value if needed, or just set)
                // For now, set the note text as the cell value if cell is empty, otherwise just status
                if (!currentVal) {
                  setCell(props.activeSheet, row, col, note, undefined).catch(() => {});
                  lastFetchKey = '';
                  fetchVisibleData();
                  props.onContentChange(note);
                }
                props.onStatusChange(`Note: ${note}`);
              }
            }}
          >
            Insert note
          </div>
          <div class="context-menu-separator" />
          <div class="context-menu-item" onClick={ctxInsertRowAbove}>
            Insert row above
          </div>
          <div class="context-menu-item" onClick={ctxInsertRowBelow}>
            Insert row below
          </div>
          <div class="context-menu-item" onClick={ctxInsertColLeft}>
            Insert column left
          </div>
          <div class="context-menu-item" onClick={ctxInsertColRight}>
            Insert column right
          </div>
          <div class="context-menu-separator" />
          <div class="context-menu-item destructive" onClick={ctxDeleteRow}>
            Delete row
          </div>
          <div class="context-menu-item destructive" onClick={ctxDeleteCol}>
            Delete column
          </div>
          <div class="context-menu-separator" />
          <Show when={ctxMenuTarget() === 'row-header'}>
            <div class="context-menu-item" onClick={ctxHideRow}>
              Hide row
            </div>
            <div class="context-menu-item" onClick={ctxUnhideRow}>
              Unhide row
            </div>
            <div class="context-menu-separator" />
            <div class="context-menu-item" onClick={ctxGroupRows}>
              Group rows
            </div>
            <div class="context-menu-item" onClick={ctxUngroupRows}>
              Ungroup rows
            </div>
            <div class="context-menu-separator" />
          </Show>
          <Show when={ctxMenuTarget() === 'col-header'}>
            <div class="context-menu-item" onClick={ctxHideCol}>
              Hide column
            </div>
            <div class="context-menu-item" onClick={ctxUnhideCol}>
              Unhide column
            </div>
            <div class="context-menu-separator" />
          </Show>
          <div class="context-menu-item" onClick={ctxSortAsc}>
            Sort sheet A &rarr; Z
          </div>
          <div class="context-menu-item" onClick={ctxSortDesc}>
            Sort sheet Z &rarr; A
          </div>
          <Show when={props.onSortDialogOpen}>
            <div class="context-menu-item" onClick={() => { dismissContextMenu(); props.onSortDialogOpen?.(); }}>
              Custom sort...
            </div>
          </Show>
          <div class="context-menu-separator" />
          <div class="context-menu-item" onClick={ctxClearContents}>
            Clear contents
          </div>
          <div class="context-menu-separator" />
          <div class="context-menu-item" onClick={() => { dismissContextMenu(); props.onFormatCellsOpen?.(); }}>
            Format cells...
          </div>
          <div class="context-menu-item" onClick={() => { dismissContextMenu(); props.onDataValidationOpen?.(); }}>
            Data validation...
          </div>
        </div>
      </Show>
      <Show when={validationDropdownVisible()}>
        <div
          class="validation-dropdown"
          style={{
            left: `${validationDropdownX()}px`,
            top: `${validationDropdownY()}px`,
            "min-width": `${getColWidth(validationDropdownCol())}px`,
          }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          {validationDropdownItems().map((item) => (
            <div
              class="validation-dropdown-item"
              onClick={() => handleValidationDropdownSelect(item)}
            >
              {item}
            </div>
          ))}
        </div>
      </Show>
    </div>
  );
};

export default VirtualGrid;
