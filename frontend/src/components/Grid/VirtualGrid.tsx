import type { Component } from 'solid-js';
import { createSignal, createEffect, onMount, onCleanup, Show } from 'solid-js';
import { col_to_letter } from '../../bridge/tauri_helpers';
import type { CellData } from '../../bridge/tauri';
import { getCell, getRange, setCell, undo, redo } from '../../bridge/tauri';
import AutoComplete, { getColumnSuggestions } from './AutoComplete';
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

// Colors — kept in sync with grid.css CSS variables (light mode defaults).
const COLORS = {
  headerBg: '#f8f9fa',
  headerText: '#5f6368',
  gridBorder: '#e0e0e0',
  selectionBorder: '#1a73e8',
  selectionBg: 'rgba(26, 115, 232, 0.08)',
  cornerBg: '#f8f9fa',
  cellText: '#202124',
  freezeBorder: '#9e9e9e',
};

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
  onSelectionChange: (row: number, col: number) => void;
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

  // Split pane divider drag state
  let splitDrag: {
    kind: 'row' | 'col';
    startMouse: number;
    startSplitRow: number;
    startSplitCol: number;
  } | null = null;

  // Selection state
  const [selectedRow, setSelectedRow] = createSignal(0);
  const [selectedCol, setSelectedCol] = createSignal(0);
  const [rangeAnchor, setRangeAnchor] = createSignal<[number, number] | null>(null);
  const [rangeEnd, setRangeEnd] = createSignal<[number, number] | null>(null);

  // Cell data cache: maps "row:col" to CellData
  const cellCache = new Map<string, CellData>();
  let lastFetchKey = ''; // tracks last fetched range to avoid duplicate calls

  // Image cache: maps data URL to loaded HTMLImageElement (or null if loading).
  const imageCache = new Map<string, HTMLImageElement | null>();

  // Editing state
  const [editing, setEditing] = createSignal(false);
  const [editValue, setEditValue] = createSignal('');
  let editorRef: HTMLInputElement | undefined;

  // Auto-complete state
  const [acVisible, setAcVisible] = createSignal(false);
  const [acSuggestions, setAcSuggestions] = createSignal<string[]>([]);
  const [acSelectedIdx, setAcSelectedIdx] = createSignal(0);

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

  /** Return the x-offset (in content coordinates) of the left edge of `col`. */
  function getColX(col: number): number {
    // For efficiency, sum only the columns that have custom widths up to `col`.
    // All others use the default width.
    let x = col * DEFAULT_COL_WIDTH;
    colWidths.forEach((w, c) => {
      if (c < col) {
        x += w - DEFAULT_COL_WIDTH;
      }
    });
    return x;
  }

  /** Return the y-offset (in content coordinates) of the top edge of `row`. */
  function getRowY(row: number): number {
    let y = row * DEFAULT_ROW_HEIGHT;
    rowHeights.forEach((h, r) => {
      if (r < row) {
        y += h - DEFAULT_ROW_HEIGHT;
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

  // -----------------------------------------------------------------------
  // Selection change with formula bar sync
  // -----------------------------------------------------------------------

  function selectCell(row: number, col: number) {
    props.onSelectionChange(row, col);
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

  function startEditing(clearContent: boolean) {
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
          editorRef.setSelectionRange(content.length, content.length);
        }
      }
    });
  }

  async function commitEdit(moveRow: number, moveCol: number) {
    const row = selectedRow();
    const col = selectedCol();
    const value = editValue();

    setEditing(false);
    setAcVisible(false);
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
    props.onModeChange('Ready');
    containerRef?.focus();
    draw();
  }

  function handleEditorInput(value: string) {
    setEditValue(value);
    props.onContentChange(value);

    // Show/hide auto-complete based on input
    const trimmed = value.trim();
    if (trimmed.length > 0 && !trimmed.startsWith('=')) {
      // Filter suggestions by prefix
      const lower = trimmed.toLowerCase();
      const matches = acSuggestions().filter((s) => {
        const sl = s.toLowerCase();
        return sl.startsWith(lower) && sl !== lower;
      });
      setAcVisible(matches.length > 0);
      setAcSelectedIdx(0);
    } else {
      setAcVisible(false);
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

  function handleEditorKeyDown(e: KeyboardEvent) {
    // When auto-complete is visible, handle navigation keys
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

    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      // Cmd+Enter: commit edit but stay in cell (don't move selection)
      e.preventDefault();
      commitEdit(0, 0);
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
    return {
      position: 'absolute' as const,
      left: `${x}px`,
      top: `${y}px`,
      width: `${getColWidth(col)}px`,
      height: `${getRowHeight(row)}px`,
      'z-index': '10',
    };
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
  }

  // -----------------------------------------------------------------------
  // Selection helpers
  // -----------------------------------------------------------------------

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

  function isColInSelection(col: number): boolean {
    const { minCol, maxCol } = getSelectionRange();
    return col >= minCol && col <= maxCol;
  }

  function isRowInSelection(row: number): boolean {
    const { minRow, maxRow } = getSelectionRange();
    return row >= minRow && row <= maxRow;
  }

  function drawSelection(ctx: CanvasRenderingContext2D, sx: number, sy: number) {
    const range = getSelectionRange();

    // Draw range fill if multi-cell selection
    if (range.minRow !== range.maxRow || range.minCol !== range.maxCol) {
      const rx = ROW_NUMBER_WIDTH + getColX(range.minCol) - sx;
      const ry = HEADER_HEIGHT + getRowY(range.minRow) - sy;
      const rw = getColX(range.maxCol + 1) - getColX(range.minCol);
      const rh = getRowY(range.maxRow + 1) - getRowY(range.minRow);
      ctx.fillStyle = COLORS.selectionBg;
      ctx.fillRect(rx, ry, rw, rh);
    }

    // Draw active cell border (2px blue)
    const cx = ROW_NUMBER_WIDTH + getColX(selectedCol()) - sx;
    const cy = HEADER_HEIGHT + getRowY(selectedRow()) - sy;
    ctx.strokeStyle = COLORS.selectionBorder;
    ctx.lineWidth = 2;
    ctx.strokeRect(cx, cy, getColWidth(selectedCol()), getRowHeight(selectedRow()));
    ctx.lineWidth = 1;
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

    for (let r = 0; r < rowCount; r++) {
      const row = startRow + r;
      const rh = getRowHeight(row);
      const y = HEADER_HEIGHT + getRowY(row) - sy;
      for (let c = 0; c < colCount; c++) {
        const col = startCol + c;
        const cell = cellCache.get(`${row}:${col}`);
        if (!cell || !cell.value) continue;

        const cw = getColWidth(col);
        const x = ROW_NUMBER_WIDTH + getColX(col) - sx;

        // Image cell: value starts with data:image/
        if (cell.value.startsWith('data:image/')) {
          const img = getCachedImage(cell.value);
          if (img) {
            // Scale image to fit within the cell with padding, preserving aspect ratio
            const maxW = cw - PADDING * 2;
            const maxH = rh - PADDING * 2;
            if (maxW > 0 && maxH > 0) {
              const scale = Math.min(maxW / img.width, maxH / img.height, 1);
              const drawW = img.width * scale;
              const drawH = img.height * scale;
              // Center the image in the cell
              const drawX = x + PADDING + (maxW - drawW) / 2;
              const drawY = y + PADDING + (maxH - drawH) / 2;
              ctx.drawImage(img, drawX, drawY, drawW, drawH);
            }
          }
          continue;
        }

        // Determine font style
        const fontWeight = cell.bold ? 'bold' : 'normal';
        const fontStyle = cell.italic ? 'italic' : 'normal';
        ctx.font = `${fontStyle} ${fontWeight} 13px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif`;
        ctx.fillStyle = COLORS.cellText;

        // Right-align numbers, left-align strings
        const isNumber = !isNaN(Number(cell.value)) && cell.value.trim() !== '';
        if (isNumber) {
          ctx.textAlign = 'right';
          ctx.fillText(cell.value, x + cw - PADDING, y + rh / 2, cw - PADDING * 2);
        } else {
          ctx.textAlign = 'left';
          ctx.fillText(cell.value, x + PADDING, y + rh / 2, cw - PADDING * 2);
        }
      }
    }
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
    }
  }

  function drawCorner(ctx: CanvasRenderingContext2D) {
    ctx.fillStyle = COLORS.cornerBg;
    ctx.fillRect(0, 0, ROW_NUMBER_WIDTH, HEADER_HEIGHT);
    ctx.strokeStyle = COLORS.gridBorder;
    ctx.strokeRect(0, 0, ROW_NUMBER_WIDTH, HEADER_HEIGHT);
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
      ctx.font = `${fontStyle} ${fontWeight} 13px -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif`;
      const tw = ctx.measureText(cell.value).width + PADDING * 2 + 4;
      maxW = Math.max(maxW, tw);
    }

    maxW = Math.ceil(maxW);
    if (maxW === DEFAULT_COL_WIDTH) {
      colWidths.delete(col);
    } else {
      colWidths.set(col, maxW);
    }
    draw();
  }

  /** Auto-fit a row height based on the default (reset to default). */
  function autoFitRow(row: number) {
    rowHeights.delete(row);
    draw();
  }

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

    // Update cursor based on hover position.
    if (hitColHeaderBorder(localX, localY) >= 0) {
      containerRef.style.cursor = 'col-resize';
    } else if (hitRowHeaderBorder(localX, localY) >= 0) {
      containerRef.style.cursor = 'row-resize';
    } else {
      containerRef.style.cursor = '';
    }
  }

  function handleResizeMouseUp() {
    if (!resizeDrag) return;
    // If the width/height matches the default, remove the override.
    if (resizeDrag.kind === 'col') {
      const w = colWidths.get(resizeDrag.index);
      if (w !== undefined && w === DEFAULT_COL_WIDTH) {
        colWidths.delete(resizeDrag.index);
      }
    } else {
      const h = rowHeights.get(resizeDrag.index);
      if (h !== undefined && h === DEFAULT_ROW_HEIGHT) {
        rowHeights.delete(resizeDrag.index);
      }
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
    return col;
  }

  /** Find which row a content-y coordinate falls in. */
  function rowAtY(contentY: number): number {
    let row = Math.floor(contentY / DEFAULT_ROW_HEIGHT);
    row = Math.max(0, Math.min(row, TOTAL_ROWS - 1));
    while (row > 0 && getRowY(row) > contentY) row--;
    while (row < TOTAL_ROWS - 1 && getRowY(row + 1) <= contentY) row++;
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

  let lastClickTime = 0;
  let lastClickRow = -1;
  let lastClickCol = -1;

  function handleMouseDown(e: MouseEvent) {
    if (editing()) return; // let the editor handle clicks
    if (!containerRef) return;

    const rect = containerRef.getBoundingClientRect();
    const localX = e.clientX - rect.left;
    const localY = e.clientY - rect.top;

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

    // Normal cell click handling.
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
      if (!rangeAnchor()) {
        setRangeAnchor([selectedRow(), selectedCol()]);
      }
      setRangeEnd([hit.row, hit.col]);
    } else {
      setSelectedRow(hit.row);
      setSelectedCol(hit.col);
      setRangeAnchor(null);
      setRangeEnd(null);
      selectCell(hit.row, hit.col);

      if (isDoubleClick) {
        startEditing(false);
        return;
      }
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
    if (editing()) return; // editor handles its own keys

    // F2 enters edit mode without clearing
    if (e.key === 'F2') {
      e.preventDefault();
      startEditing(false);
      return;
    }

    // Delete/Backspace clears selected cell(s)
    if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      clearSelectedCells();
      return;
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
      props.onStatusChange(`Row ${row + 1} selected`);
      draw();
      return;
    }

    // Keyboard shortcuts (Cmd/Ctrl + key)
    if (e.metaKey || e.ctrlKey) {
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
      if (e.key === ';') {
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
    }

    let row = selectedRow();
    let col = selectedCol();
    let handled = false;

    switch (e.key) {
      case 'ArrowUp':
        row = Math.max(0, row - 1);
        handled = true;
        break;
      case 'ArrowDown':
        row = Math.min(TOTAL_ROWS - 1, row + 1);
        handled = true;
        break;
      case 'ArrowLeft':
        col = Math.max(0, col - 1);
        handled = true;
        break;
      case 'ArrowRight':
        col = Math.min(TOTAL_COLS - 1, col + 1);
        handled = true;
        break;
      case 'Tab':
        e.preventDefault();
        col = Math.max(0, Math.min(TOTAL_COLS - 1, col + (e.shiftKey ? -1 : 1)));
        handled = true;
        break;
      case 'Enter':
        row = Math.max(0, Math.min(TOTAL_ROWS - 1, row + (e.shiftKey ? -1 : 1)));
        handled = true;
        break;
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

    updateSize();
    containerRef.focus();

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
  });

  return (
    <div
      ref={containerRef}
      class="virtual-grid-container"
      tabIndex={0}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onKeyDown={handleKeyDown}
      onWheel={handleWheel}
      style={{ outline: 'none' }}
    >
      <canvas ref={canvasRef} class="virtual-grid-canvas" />
      <Show when={editing()}>
        <input
          ref={editorRef}
          class="cell-editor-textarea"
          value={editValue()}
          onInput={(e) => handleEditorInput(e.currentTarget.value)}
          onKeyDown={handleEditorKeyDown}
          onBlur={() => {
            if (editing()) {
              commitEdit(0, 0);
            }
          }}
          style={editorStyle()}
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
      </Show>
    </div>
  );
};

export default VirtualGrid;
