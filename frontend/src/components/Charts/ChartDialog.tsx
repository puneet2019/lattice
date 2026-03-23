import type { Component } from 'solid-js';
import { createSignal, createEffect, For, Show } from 'solid-js';
import { renderChartSvg, createChart, deleteChart, updateChart } from '../../bridge/tauri';
import type { ChartTypeStr } from '../../bridge/tauri';

/** Chart type group for organized display. */
interface ChartTypeGroup {
  title: string;
  types: { value: ChartTypeStr; label: string; icon: string }[];
}

/** Chart types organized by category. */
const CHART_TYPE_GROUPS: ChartTypeGroup[] = [
  {
    title: 'Basic',
    types: [
      { value: 'bar', label: 'Bar', icon: 'B' },
      { value: 'line', label: 'Line', icon: 'L' },
      { value: 'pie', label: 'Pie', icon: 'P' },
      { value: 'scatter', label: 'Scatter', icon: 'S' },
      { value: 'area', label: 'Area', icon: 'A' },
      { value: 'stacked_bar', label: 'Stacked Bar', icon: 'SB' },
      { value: 'stacked_area', label: 'Stacked Area', icon: 'SA' },
    ],
  },
  {
    title: 'Advanced',
    types: [
      { value: 'combo', label: 'Combo', icon: 'Cb' },
      { value: 'histogram', label: 'Histogram', icon: 'H' },
    ],
  },
  {
    title: 'Financial',
    types: [
      { value: 'candlestick', label: 'Candlestick', icon: 'Cs' },
      { value: 'waterfall', label: 'Waterfall', icon: 'W' },
    ],
  },
  {
    title: 'Specialty',
    types: [
      { value: 'treemap', label: 'Treemap', icon: 'T' },
      { value: 'radar', label: 'Radar', icon: 'R' },
      { value: 'bubble', label: 'Bubble', icon: 'Bb' },
      { value: 'gauge', label: 'Gauge', icon: 'G' },
    ],
  },
];

/** Flat list of all chart types (for backward compat). */
const CHART_TYPES = CHART_TYPE_GROUPS.flatMap((g) => g.types);

export interface ChartDialogProps {
  /** Current active sheet name. */
  activeSheet: string;
  /** Called when a chart is created or updated (returns chart ID). */
  onInsert: (chartId: string) => void;
  /** Called when dialog is cancelled/closed. */
  onClose: () => void;
  /** If set, dialog is in edit mode for an existing chart. */
  editChartId?: string;
  /** Initial chart type when editing. */
  initialChartType?: ChartTypeStr;
  /** Initial data range when editing. */
  initialDataRange?: string;
  /** Initial title when editing. */
  initialTitle?: string;
}

const ChartDialog: Component<ChartDialogProps> = (props) => {
  const isEditMode = () => !!props.editChartId;

  const [chartType, setChartType] = createSignal<ChartTypeStr>(
    props.initialChartType ?? 'bar',
  );
  const [dataRange, setDataRange] = createSignal(props.initialDataRange ?? '');
  const [title, setTitle] = createSignal(props.initialTitle ?? '');
  const [previewSvg, setPreviewSvg] = createSignal('');
  const [previewChartId, setPreviewChartId] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal('');

  // Debounced preview: create a temporary chart and render its SVG.
  // In edit mode we render the existing chart's SVG for the initial preview.
  let previewTimeout: ReturnType<typeof setTimeout> | undefined;

  const updatePreview = () => {
    const range = dataRange();
    if (!range || !range.includes(':')) {
      setPreviewSvg('');
      return;
    }

    if (previewTimeout) clearTimeout(previewTimeout);
    previewTimeout = setTimeout(() => {
      void (async () => {
        try {
          // In edit mode, update the existing chart for preview.
          if (isEditMode()) {
            const editId = props.editChartId!;
            await updateChart(
              editId,
              chartType(),
              range,
              title() || undefined,
            );
            const svg = await renderChartSvg(editId);
            setPreviewSvg(svg);
            setError('');
            return;
          }

          // In create mode, clean up previous preview chart.
          const prevId = previewChartId();
          if (prevId) {
            try {
              await deleteChart(prevId);
            } catch {
              // Ignore cleanup failures.
            }
          }

          const id = await createChart(
            props.activeSheet,
            chartType(),
            range,
            title() || undefined,
          );
          setPreviewChartId(id);
          const svg = await renderChartSvg(id);
          setPreviewSvg(svg);
          setError('');
        } catch (e) {
          setPreviewSvg('');
          setError(String(e));
        }
      })();
    }, 300);
  };

  // Trigger preview when inputs change.
  createEffect(() => {
    // Read reactive values to track them.
    chartType();
    dataRange();
    title();
    updatePreview();
  });

  const handleInsert = async () => {
    const range = dataRange();
    if (!range || !range.includes(':')) {
      setError('Please enter a valid data range (e.g., A1:C10)');
      return;
    }

    setLoading(true);
    try {
      if (isEditMode()) {
        // Update the existing chart and return its ID.
        const editId = props.editChartId!;
        await updateChart(
          editId,
          chartType(),
          range,
          title() || undefined,
        );
        props.onInsert(editId);
      } else {
        // If we already have a preview chart, use it.
        const existingId = previewChartId();
        if (existingId) {
          setPreviewChartId(null);
          props.onInsert(existingId);
        } else {
          const chartId = await createChart(
            props.activeSheet,
            chartType(),
            range,
            title() || undefined,
          );
          props.onInsert(chartId);
        }
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleCancel = async () => {
    // In create mode, clean up preview chart if one exists.
    if (!isEditMode()) {
      const prevId = previewChartId();
      if (prevId) {
        try {
          await deleteChart(prevId);
        } catch {
          // Ignore.
        }
      }
    }
    props.onClose();
  };

  const handleBackdropClick = (e: MouseEvent) => {
    if ((e.target as HTMLElement).classList.contains('chart-dialog-backdrop')) {
      void handleCancel();
    }
  };

  return (
    <div class="chart-dialog-backdrop" onClick={handleBackdropClick}>
      <div class="chart-dialog">
        <div class="chart-dialog-header">
          <h2>{isEditMode() ? 'Edit Chart' : 'Insert Chart'}</h2>
          <button
            class="chart-overlay-close"
            onClick={() => void handleCancel()}
            title="Close"
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

        <div class="chart-dialog-body">
          {/* Chart type selector */}
          <div class="chart-dialog-field">
            <label class="chart-dialog-label">Chart Type</label>
            <For each={CHART_TYPE_GROUPS}>
              {(group) => (
                <div class="chart-dialog-type-group">
                  <span class="chart-dialog-type-group-label">{group.title}</span>
                  <div class="chart-dialog-type-grid">
                    <For each={group.types}>
                      {(ct) => (
                        <button
                          class={`chart-dialog-type-btn ${chartType() === ct.value ? 'active' : ''}`}
                          onClick={() => setChartType(ct.value)}
                        >
                          <span style={{ "font-size": "18px", "font-weight": "600" }}>
                            {ct.icon}
                          </span>
                          <span>{ct.label}</span>
                        </button>
                      )}
                    </For>
                  </div>
                </div>
              )}
            </For>
          </div>

          {/* Data range input */}
          <div class="chart-dialog-field">
            <label class="chart-dialog-label">Data Range</label>
            <input
              class="chart-dialog-input"
              type="text"
              placeholder="e.g., A1:C10"
              value={dataRange()}
              onInput={(e) => setDataRange(e.currentTarget.value)}
            />
          </div>

          {/* Title input */}
          <div class="chart-dialog-field">
            <label class="chart-dialog-label">Title (optional)</label>
            <input
              class="chart-dialog-input"
              type="text"
              placeholder="Chart title"
              value={title()}
              onInput={(e) => setTitle(e.currentTarget.value)}
            />
          </div>

          {/* Preview */}
          <div class="chart-dialog-field">
            <label class="chart-dialog-label">Preview</label>
            <div class="chart-dialog-preview">
              <Show
                when={previewSvg()}
                fallback={
                  <span style={{ color: 'var(--header-text)', "font-size": '12px' }}>
                    {error() || 'Enter a data range to see a preview'}
                  </span>
                }
              >
                <div innerHTML={previewSvg()} />
              </Show>
            </div>
          </div>

          {/* Error display */}
          <Show when={error() && previewSvg()}>
            <div style={{ color: 'var(--danger-color)', "font-size": '11px' }}>
              {error()}
            </div>
          </Show>
        </div>

        <div class="chart-dialog-footer">
          <button
            class="chart-dialog-btn"
            onClick={() => void handleCancel()}
          >
            Cancel
          </button>
          <button
            class="chart-dialog-btn chart-dialog-btn-primary"
            onClick={() => void handleInsert()}
            disabled={loading() || !dataRange()}
          >
            {loading()
              ? isEditMode() ? 'Updating...' : 'Creating...'
              : isEditMode() ? 'Update Chart' : 'Insert Chart'}
          </button>
        </div>
      </div>
    </div>
  );
};

export default ChartDialog;
