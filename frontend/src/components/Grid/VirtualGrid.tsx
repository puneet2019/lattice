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

const VirtualGrid: Component<VirtualGridProps> = (_props) => {
  let containerRef: HTMLDivElement | undefined;
  let canvasRef: HTMLCanvasElement | undefined;

  const [scrollX, setScrollX] = createSignal(0);
  const [scrollY, setScrollY] = createSignal(0);
  const [canvasWidth, setCanvasWidth] = createSignal(800);
  const [canvasHeight, setCanvasHeight] = createSignal(600);

  // -----------------------------------------------------------------------
  // Viewport helpers
  // -----------------------------------------------------------------------

  const totalContentWidth = () => ROW_NUMBER_WIDTH + TOTAL_COLS * DEFAULT_COL_WIDTH;
  const totalContentHeight = () => HEADER_HEIGHT + TOTAL_ROWS * ROW_HEIGHT;

  const firstVisibleCol = () => Math.floor(scrollX() / DEFAULT_COL_WIDTH);
  const firstVisibleRow = () => Math.floor(scrollY() / ROW_HEIGHT);

  const visibleColCount = () => {
    const count = Math.ceil((canvasWidth() - ROW_NUMBER_WIDTH) / DEFAULT_COL_WIDTH) + 2;
    return Math.min(count, TOTAL_COLS - firstVisibleCol());
  };

  const visibleRowCount = () => {
    const count = Math.ceil((canvasHeight() - HEADER_HEIGHT) / ROW_HEIGHT) + 2;
    return Math.min(count, TOTAL_ROWS - firstVisibleRow());
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
    drawColumnHeaders(ctx, sx, startCol, colCount, w);
    drawRowNumbers(ctx, sy, startRow, rowCount, h);
    drawCorner(ctx);
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

      ctx.fillStyle = COLORS.headerText;
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

      ctx.fillStyle = COLORS.headerText;
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
  // Scroll
  // -----------------------------------------------------------------------

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    const maxX = Math.max(0, totalContentWidth() - canvasWidth());
    const maxY = Math.max(0, totalContentHeight() - canvasHeight());
    setScrollX(Math.max(0, Math.min(maxX, scrollX() + e.deltaX)));
    setScrollY(Math.max(0, Math.min(maxY, scrollY() + e.deltaY)));
    draw();
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
      onWheel={handleWheel}
      style={{ outline: 'none' }}
    >
      <canvas ref={canvasRef} class="virtual-grid-canvas" />
    </div>
  );
};

export default VirtualGrid;
