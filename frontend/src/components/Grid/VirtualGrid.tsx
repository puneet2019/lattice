import type { Component } from 'solid-js';
import { createSignal, onMount, onCleanup } from 'solid-js';
import { col_to_letter } from '../../bridge/tauri_helpers';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

export const DEFAULT_COL_WIDTH = 80;
export const ROW_HEIGHT = 21;
export const HEADER_HEIGHT = 24;
export const ROW_NUMBER_WIDTH = 50;
export const TOTAL_COLS = 702; // A..ZZ
export const TOTAL_ROWS = 10_000;

// Colors — kept in sync with grid.css CSS variables (light mode defaults).
const COLORS = {
  headerBg: '#f8f9fa',
  headerText: '#5f6368',
  gridBorder: '#e0e0e0',
  selectionBorder: '#1a73e8',
  selectionBg: 'rgba(26, 115, 232, 0.08)',
  cornerBg: '#f8f9fa',
};

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface VirtualGridProps {
  activeSheet: string;
  onSelectionChange: (row: number, col: number) => void;
  onContentChange: (content: string) => void;
  onCellCommit: (row: number, col: number, value: string) => void;
  onStatusChange: (message: string) => void;
  onModeChange: (mode: 'Ready' | 'Edit') => void;
  onBoldToggle: () => void;
  onItalicToggle: () => void;
  onUnderlineToggle: () => void;
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

  // Selection state
  const [selectedRow, setSelectedRow] = createSignal(0);
  const [selectedCol, setSelectedCol] = createSignal(0);
  const [rangeAnchor, setRangeAnchor] = createSignal<[number, number] | null>(null);
  const [rangeEnd, setRangeEnd] = createSignal<[number, number] | null>(null);

  // -----------------------------------------------------------------------
  // Viewport helpers
  // -----------------------------------------------------------------------

  const totalContentWidth = () => ROW_NUMBER_WIDTH + TOTAL_COLS * DEFAULT_COL_WIDTH;
  const totalContentHeight = () => HEADER_HEIGHT + TOTAL_ROWS * ROW_HEIGHT;

  // Buffer: render extra rows/cols beyond viewport for smooth scrolling.
  const BUFFER_COLS = 4;
  const BUFFER_ROWS = 8;

  const firstVisibleCol = () => {
    const col = Math.floor(scrollX() / DEFAULT_COL_WIDTH);
    return Math.max(0, col - BUFFER_COLS);
  };
  const firstVisibleRow = () => {
    const row = Math.floor(scrollY() / ROW_HEIGHT);
    return Math.max(0, row - BUFFER_ROWS);
  };

  const visibleColCount = () => {
    const viewportCols = Math.ceil((canvasWidth() - ROW_NUMBER_WIDTH) / DEFAULT_COL_WIDTH);
    const total = viewportCols + BUFFER_COLS * 2;
    return Math.min(total, TOTAL_COLS - firstVisibleCol());
  };

  const visibleRowCount = () => {
    const viewportRows = Math.ceil((canvasHeight() - HEADER_HEIGHT) / ROW_HEIGHT);
    const total = viewportRows + BUFFER_ROWS * 2;
    return Math.min(total, TOTAL_ROWS - firstVisibleRow());
  };

  // -----------------------------------------------------------------------
  // Drawing
  // -----------------------------------------------------------------------

  function draw() {
    const canvas = canvasRef;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const w = canvasWidth();
    const h = canvasHeight();

    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    const sx = scrollX();
    const sy = scrollY();
    const startCol = firstVisibleCol();
    const startRow = firstVisibleRow();
    const colCount = visibleColCount();
    const rowCount = visibleRowCount();

    drawGridLines(ctx, sx, sy, startCol, startRow, colCount, rowCount, w, h);
    drawSelection(ctx, sx, sy);
    drawColumnHeaders(ctx, sx, startCol, colCount, w);
    drawRowNumbers(ctx, sy, startRow, rowCount, h);
    drawCorner(ctx);
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
      const rx = ROW_NUMBER_WIDTH + range.minCol * DEFAULT_COL_WIDTH - sx;
      const ry = HEADER_HEIGHT + range.minRow * ROW_HEIGHT - sy;
      const rw = (range.maxCol - range.minCol + 1) * DEFAULT_COL_WIDTH;
      const rh = (range.maxRow - range.minRow + 1) * ROW_HEIGHT;
      ctx.fillStyle = COLORS.selectionBg;
      ctx.fillRect(rx, ry, rw, rh);
    }

    // Draw active cell border (2px blue)
    const cx = ROW_NUMBER_WIDTH + selectedCol() * DEFAULT_COL_WIDTH - sx;
    const cy = HEADER_HEIGHT + selectedRow() * ROW_HEIGHT - sy;
    ctx.strokeStyle = COLORS.selectionBorder;
    ctx.lineWidth = 2;
    ctx.strokeRect(cx, cy, DEFAULT_COL_WIDTH, ROW_HEIGHT);
    ctx.lineWidth = 1;
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

    for (let c = 0; c <= colCount; c++) {
      const col = startCol + c;
      const x = ROW_NUMBER_WIDTH + col * DEFAULT_COL_WIDTH - sx;
      if (x < ROW_NUMBER_WIDTH || x > w) continue;
      ctx.beginPath();
      ctx.moveTo(Math.round(x) + 0.5, HEADER_HEIGHT);
      ctx.lineTo(Math.round(x) + 0.5, h);
      ctx.stroke();
    }

    for (let r = 0; r <= rowCount; r++) {
      const row = startRow + r;
      const y = HEADER_HEIGHT + row * ROW_HEIGHT - sy;
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
      const x = ROW_NUMBER_WIDTH + col * DEFAULT_COL_WIDTH - sx;
      const cellRight = x + DEFAULT_COL_WIDTH;
      if (cellRight < ROW_NUMBER_WIDTH || x > w) continue;

      ctx.strokeStyle = COLORS.gridBorder;
      ctx.beginPath();
      ctx.moveTo(Math.round(cellRight) + 0.5, 0);
      ctx.lineTo(Math.round(cellRight) + 0.5, HEADER_HEIGHT);
      ctx.stroke();

      // Highlight selected column header
      if (isColInSelection(col)) {
        ctx.fillStyle = COLORS.selectionBg;
        ctx.fillRect(x, 0, DEFAULT_COL_WIDTH, HEADER_HEIGHT);
        ctx.fillStyle = COLORS.selectionBorder;
      } else {
        ctx.fillStyle = COLORS.headerText;
      }
      ctx.fillText(col_to_letter(col), x + DEFAULT_COL_WIDTH / 2, HEADER_HEIGHT / 2);
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
      const y = HEADER_HEIGHT + row * ROW_HEIGHT - sy;
      const cellBottom = y + ROW_HEIGHT;
      if (cellBottom < HEADER_HEIGHT || y > h) continue;

      ctx.strokeStyle = COLORS.gridBorder;
      ctx.beginPath();
      ctx.moveTo(0, Math.round(cellBottom) + 0.5);
      ctx.lineTo(ROW_NUMBER_WIDTH, Math.round(cellBottom) + 0.5);
      ctx.stroke();

      // Highlight selected row number
      if (isRowInSelection(row)) {
        ctx.fillStyle = COLORS.selectionBg;
        ctx.fillRect(0, y, ROW_NUMBER_WIDTH, ROW_HEIGHT);
        ctx.fillStyle = COLORS.selectionBorder;
      } else {
        ctx.fillStyle = COLORS.headerText;
      }
      ctx.fillText(String(row + 1), ROW_NUMBER_WIDTH / 2, y + ROW_HEIGHT / 2);
    }
  }

  function drawCorner(ctx: CanvasRenderingContext2D) {
    ctx.fillStyle = COLORS.cornerBg;
    ctx.fillRect(0, 0, ROW_NUMBER_WIDTH, HEADER_HEIGHT);
    ctx.strokeStyle = COLORS.gridBorder;
    ctx.strokeRect(0, 0, ROW_NUMBER_WIDTH, HEADER_HEIGHT);
  }

  // -----------------------------------------------------------------------
  // Hit testing & event handlers
  // -----------------------------------------------------------------------

  function hitTest(
    clientX: number,
    clientY: number,
  ): { row: number; col: number } | null {
    if (!containerRef) return null;
    const rect = containerRef.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    if (x < ROW_NUMBER_WIDTH || y < HEADER_HEIGHT) return null;
    const col = Math.floor((x - ROW_NUMBER_WIDTH + scrollX()) / DEFAULT_COL_WIDTH);
    const row = Math.floor((y - HEADER_HEIGHT + scrollY()) / ROW_HEIGHT);
    if (col < 0 || col >= TOTAL_COLS || row < 0 || row >= TOTAL_ROWS) return null;
    return { row, col };
  }

  function handleMouseDown(e: MouseEvent) {
    const hit = hitTest(e.clientX, e.clientY);
    if (!hit) return;

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
      props.onSelectionChange(hit.row, hit.col);
    }
    draw();
  }

  function ensureCellVisible(row: number, col: number) {
    const sx = scrollX();
    const sy = scrollY();
    const viewW = canvasWidth() - ROW_NUMBER_WIDTH;
    const viewH = canvasHeight() - HEADER_HEIGHT;

    const cellLeft = col * DEFAULT_COL_WIDTH;
    const cellRight = cellLeft + DEFAULT_COL_WIDTH;
    if (cellLeft < sx) {
      setScrollX(cellLeft);
    } else if (cellRight > sx + viewW) {
      setScrollX(cellRight - viewW);
    }

    const cellTop = row * ROW_HEIGHT;
    const cellBottom = cellTop + ROW_HEIGHT;
    if (cellTop < sy) {
      setScrollY(cellTop);
    } else if (cellBottom > sy + viewH) {
      setScrollY(cellBottom - viewH);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
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
      props.onSelectionChange(row, col);
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
    setScrollX(Math.max(0, Math.min(maxX, scrollX() + e.deltaX)));
    setScrollY(Math.max(0, Math.min(maxY, scrollY() + e.deltaY)));
    scheduleDraw();
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
  });

  return (
    <div
      ref={containerRef}
      class="virtual-grid-container"
      tabIndex={0}
      onMouseDown={handleMouseDown}
      onKeyDown={handleKeyDown}
      onWheel={handleWheel}
      style={{ outline: 'none' }}
    >
      <canvas ref={canvasRef} class="virtual-grid-canvas" />
    </div>
  );
};

export default VirtualGrid;
