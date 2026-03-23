import type { Component } from 'solid-js';
import { createSignal, createEffect, onCleanup, For } from 'solid-js';
import {
  renderChartSvg,
  deleteChart,
  updateChart,
} from '../../bridge/tauri';
import type { ChartInfo } from '../../bridge/tauri';

export interface ChartOverlay {
  /** Chart metadata from the backend. */
  info: ChartInfo;
  /** Position on screen (pixels from grid top-left). */
  x: number;
  y: number;
  /** Display dimensions (may differ from chart's intrinsic size). */
  width: number;
  height: number;
}

export interface ChartContainerProps {
  /** List of charts to display as floating overlays. */
  charts: ChartOverlay[];
  /** Called when a chart is deleted (by ID). */
  onDelete: (chartId: string) => void;
  /** Called when a chart's position changes after drag. */
  onMove: (chartId: string, x: number, y: number) => void;
  /** Called when a chart's size changes after resize. */
  onResize: (chartId: string, width: number, height: number) => void;
  /** Called when a chart is double-clicked to edit. */
  onEditChart?: (chartId: string) => void;
  /** Called when chart SVG is re-rendered (e.g. after resize). */
  onSvgUpdate?: (chartId: string, svg: string) => void;
}

/** A single floating chart overlay. */
const ChartPanel: Component<{
  chart: ChartOverlay;
  onDelete: (chartId: string) => void;
  onMove: (chartId: string, x: number, y: number) => void;
  onResize: (chartId: string, width: number, height: number) => void;
  onEditChart?: (chartId: string) => void;
  onSvgUpdate?: (chartId: string, svg: string) => void;
}> = (props) => {
  const [svgContent, setSvgContent] = createSignal('');
  const [dragging, setDragging] = createSignal(false);
  const [resizing, setResizing] = createSignal(false);
  const [dragOffset, setDragOffset] = createSignal({ x: 0, y: 0 });

  /** Fetch and set the chart SVG. */
  const fetchSvg = (chartId: string) => {
    void renderChartSvg(chartId)
      .then((svg) => setSvgContent(svg))
      .catch(() => {
        setSvgContent(
          `<svg xmlns="http://www.w3.org/2000/svg" width="${props.chart.width}" height="${props.chart.height}">` +
            `<rect width="100%" height="100%" fill="#f5f5f5" stroke="#ccc" rx="4"/>` +
            `<text x="50%" y="50%" text-anchor="middle" font-family="sans-serif" font-size="12" fill="#999">Chart unavailable</text>` +
            `</svg>`,
        );
      });
  };

  // Fetch the SVG on mount or when chart ID changes.
  createEffect(() => {
    fetchSvg(props.chart.info.id);
  });

  const handleClose = () => {
    void deleteChart(props.chart.info.id)
      .then(() => props.onDelete(props.chart.info.id))
      .catch(() => props.onDelete(props.chart.info.id));
  };

  // -- Double-click to edit --
  const handleBodyDoubleClick = () => {
    props.onEditChart?.(props.chart.info.id);
  };

  // -- Drag logic --
  const handleDragStart = (e: MouseEvent) => {
    e.preventDefault();
    setDragging(true);
    setDragOffset({
      x: e.clientX - props.chart.x,
      y: e.clientY - props.chart.y,
    });
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (dragging()) {
      const offset = dragOffset();
      props.onMove(
        props.chart.info.id,
        e.clientX - offset.x,
        e.clientY - offset.y,
      );
    }
    if (resizing()) {
      const newWidth = Math.max(200, e.clientX - props.chart.x);
      const newHeight = Math.max(150, e.clientY - props.chart.y);
      props.onResize(props.chart.info.id, newWidth, newHeight);
    }
  };

  const handleMouseUp = () => {
    const wasResizing = resizing();
    setDragging(false);
    setResizing(false);

    // On resize end, update backend dimensions and re-render SVG.
    if (wasResizing) {
      const chartId = props.chart.info.id;
      const w = props.chart.width;
      const h = props.chart.height;
      void updateChart(chartId, undefined, undefined, undefined, w, h)
        .then(() => renderChartSvg(chartId))
        .then((svg) => {
          setSvgContent(svg);
          props.onSvgUpdate?.(chartId, svg);
        })
        .catch(() => {
          // Re-fetch to stay in sync even on error.
          fetchSvg(chartId);
        });
    }
  };

  // -- Resize logic --
  const handleResizeStart = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setResizing(true);
  };

  // Attach global listeners when dragging/resizing.
  createEffect(() => {
    if (dragging() || resizing()) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
    } else {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    }
  });

  onCleanup(() => {
    window.removeEventListener('mousemove', handleMouseMove);
    window.removeEventListener('mouseup', handleMouseUp);
  });

  const titleText = () =>
    props.chart.info.title ?? `${props.chart.info.chart_type} chart`;

  /** Download chart as PNG. */
  const handleDownloadPng = () => {
    const svg = svgContent();
    if (!svg) return;

    const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const img = new Image();
    img.onload = () => {
      const canvas = document.createElement('canvas');
      const dpr = window.devicePixelRatio || 1;
      canvas.width = props.chart.width * dpr;
      canvas.height = props.chart.height * dpr;
      const ctx = canvas.getContext('2d');
      if (!ctx) {
        URL.revokeObjectURL(url);
        return;
      }
      ctx.scale(dpr, dpr);
      ctx.drawImage(img, 0, 0, props.chart.width, props.chart.height);
      URL.revokeObjectURL(url);

      const pngUrl = canvas.toDataURL('image/png');
      const link = document.createElement('a');
      link.download = `${titleText().replace(/[^a-zA-Z0-9]/g, '_')}.png`;
      link.href = pngUrl;
      link.click();
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
    };
    img.src = url;
  };

  return (
    <div
      class="chart-overlay"
      style={{
        left: `${props.chart.x}px`,
        top: `${props.chart.y}px`,
        width: `${props.chart.width}px`,
        height: `${props.chart.height + 28}px`,
      }}
    >
      {/* Header bar (drag handle) */}
      <div class="chart-overlay-header" onMouseDown={handleDragStart}>
        <span class="chart-overlay-title">{titleText()}</span>
        <div class="chart-overlay-header-actions">
          <button
            class="chart-overlay-action-btn"
            onClick={handleDownloadPng}
            title="Download as PNG"
          >
            <svg
              width="12"
              height="12"
              viewBox="0 0 12 12"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            >
              <path d="M6 1v7M3 6l3 3 3-3M2 10h8" />
            </svg>
          </button>
          <button
            class="chart-overlay-close"
            onClick={handleClose}
            title="Delete chart"
          >
            <svg
              width="12"
              height="12"
              viewBox="0 0 12 12"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            >
              <line x1="2" y1="2" x2="10" y2="10" />
              <line x1="10" y1="2" x2="2" y2="10" />
            </svg>
          </button>
        </div>
      </div>

      {/* SVG chart body */}
      <div
        class="chart-overlay-body"
        innerHTML={svgContent()}
        onDblClick={handleBodyDoubleClick}
      />

      {/* Resize handle (bottom-right corner) */}
      <div
        class="chart-overlay-resize"
        onMouseDown={handleResizeStart}
      />
    </div>
  );
};

/** Renders all active chart overlays on the grid. */
const ChartContainer: Component<ChartContainerProps> = (props) => {
  return (
    <For each={props.charts}>
      {(chart) => (
        <ChartPanel
          chart={chart}
          onDelete={props.onDelete}
          onMove={props.onMove}
          onResize={props.onResize}
          onEditChart={props.onEditChart}
          onSvgUpdate={props.onSvgUpdate}
        />
      )}
    </For>
  );
};

export default ChartContainer;
