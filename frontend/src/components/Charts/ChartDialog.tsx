import type { Component, JSX } from 'solid-js';
import { createSignal, createEffect, For, Show } from 'solid-js';
import { renderChartSvg, createChart, deleteChart, updateChart } from '../../bridge/tauri';
import type { ChartTypeStr } from '../../bridge/tauri';

/** SVG icon silhouettes for each chart type. */
function chartTypeIcon(type: ChartTypeStr): JSX.Element {
  const s = 'currentColor';
  switch (type) {
    case 'bar':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill={s}><rect x="3" y="14" width="4" height="8" rx="0.5" /><rect x="10" y="8" width="4" height="14" rx="0.5" /><rect x="17" y="4" width="4" height="18" rx="0.5" /></svg>);
    case 'line':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke={s} stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3,18 8,10 13,14 18,6 22,8" /></svg>);
    case 'pie':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="9" stroke={s} stroke-width="1.5" /><path d="M12 3 A9 9 0 0 1 21 12 L12 12 Z" fill={s} opacity="0.7" /><path d="M12 12 L12 3 A9 9 0 0 0 5.4 18 Z" fill={s} opacity="0.35" /></svg>);
    case 'scatter':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill={s}><circle cx="6" cy="16" r="2" /><circle cx="10" cy="10" r="2" /><circle cx="16" cy="14" r="2" /><circle cx="14" cy="6" r="2" /><circle cx="20" cy="8" r="2" /></svg>);
    case 'area':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill="none"><path d="M3 22 L3 16 L8 10 L13 14 L18 6 L22 8 L22 22 Z" fill={s} opacity="0.3" /><polyline points="3,16 8,10 13,14 18,6 22,8" fill="none" stroke={s} stroke-width="1.5" stroke-linejoin="round" /></svg>);
    case 'stacked_bar':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill={s}><rect x="3" y="14" width="4" height="8" rx="0.5" opacity="0.5" /><rect x="3" y="10" width="4" height="4" rx="0.5" /><rect x="10" y="8" width="4" height="14" rx="0.5" opacity="0.5" /><rect x="10" y="3" width="4" height="5" rx="0.5" /><rect x="17" y="10" width="4" height="12" rx="0.5" opacity="0.5" /><rect x="17" y="5" width="4" height="5" rx="0.5" /></svg>);
    case 'stacked_area':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill="none"><path d="M3 22 L3 16 L8 12 L13 15 L18 8 L22 10 L22 22 Z" fill={s} opacity="0.2" /><path d="M3 22 L3 18 L8 15 L13 17 L18 12 L22 14 L22 22 Z" fill={s} opacity="0.35" /></svg>);
    case 'combo':
      return (<svg width="24" height="24" viewBox="0 0 24 24"><rect x="4" y="14" width="4" height="8" rx="0.5" fill={s} opacity="0.4" /><rect x="10" y="10" width="4" height="12" rx="0.5" fill={s} opacity="0.4" /><rect x="16" y="12" width="4" height="10" rx="0.5" fill={s} opacity="0.4" /><polyline points="6,10 12,6 18,8" fill="none" stroke={s} stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>);
    case 'histogram':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill={s}><rect x="2" y="18" width="4" height="4" rx="0.5" /><rect x="6" y="12" width="4" height="10" rx="0.5" /><rect x="10" y="6" width="4" height="16" rx="0.5" /><rect x="14" y="10" width="4" height="12" rx="0.5" /><rect x="18" y="16" width="4" height="6" rx="0.5" /></svg>);
    case 'candlestick':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke={s} stroke-width="1.5"><line x1="6" y1="4" x2="6" y2="20" /><rect x="4" y="8" width="4" height="6" fill={s} opacity="0.3" stroke={s} /><line x1="14" y1="6" x2="14" y2="18" /><rect x="12" y="10" width="4" height="4" fill={s} opacity="0.6" stroke={s} /><line x1="20" y1="5" x2="20" y2="19" /><rect x="18" y="8" width="4" height="8" fill={s} opacity="0.3" stroke={s} /></svg>);
    case 'waterfall':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill={s}><rect x="2" y="4" width="4" height="18" rx="0.5" opacity="0.5" /><rect x="7" y="4" width="4" height="8" rx="0.5" opacity="0.7" /><rect x="12" y="8" width="4" height="6" rx="0.5" opacity="0.4" /><rect x="17" y="6" width="4" height="16" rx="0.5" opacity="0.5" /></svg>);
    case 'treemap':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke={s} stroke-width="1"><rect x="2" y="2" width="20" height="20" rx="1" /><line x1="12" y1="2" x2="12" y2="22" /><line x1="2" y1="12" x2="12" y2="12" /><line x1="12" y1="10" x2="22" y2="10" /><line x1="18" y1="10" x2="18" y2="22" /></svg>);
    case 'radar':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke={s} stroke-width="1.2"><polygon points="12,3 20,9 18,18 6,18 4,9" opacity="0.15" fill={s} /><polygon points="12,3 20,9 18,18 6,18 4,9" /><polygon points="12,7 17,11 16,16 8,16 7,11" stroke-dasharray="2 1" opacity="0.5" /></svg>);
    case 'bubble':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill={s}><circle cx="8" cy="14" r="4" opacity="0.35" /><circle cx="16" cy="10" r="3" opacity="0.5" /><circle cx="14" cy="18" r="2.5" opacity="0.25" /><circle cx="6" cy="7" r="2" opacity="0.4" /></svg>);
    case 'gauge':
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill="none"><path d="M4 18 A10 10 0 0 1 20 18" stroke={s} stroke-width="2.5" stroke-linecap="round" /><line x1="12" y1="17" x2="16" y2="9" stroke={s} stroke-width="1.5" stroke-linecap="round" /></svg>);
    default:
      return (<svg width="24" height="24" viewBox="0 0 24 24" fill={s}><rect x="4" y="4" width="16" height="16" rx="2" opacity="0.3" /></svg>);
  }
}

/** Chart type group for organized display. */
interface ChartTypeGroup {
  title: string;
  types: { value: ChartTypeStr; label: string }[];
}

/** Chart types organized by category. */
const CHART_TYPE_GROUPS: ChartTypeGroup[] = [
  {
    title: 'Basic',
    types: [
      { value: 'bar', label: 'Bar' },
      { value: 'line', label: 'Line' },
      { value: 'pie', label: 'Pie' },
      { value: 'scatter', label: 'Scatter' },
      { value: 'area', label: 'Area' },
      { value: 'stacked_bar', label: 'Stacked Bar' },
      { value: 'stacked_area', label: 'Stacked Area' },
    ],
  },
  {
    title: 'Advanced',
    types: [
      { value: 'combo', label: 'Combo' },
      { value: 'histogram', label: 'Histogram' },
    ],
  },
  {
    title: 'Financial',
    types: [
      { value: 'candlestick', label: 'Candlestick' },
      { value: 'waterfall', label: 'Waterfall' },
    ],
  },
  {
    title: 'Specialty',
    types: [
      { value: 'treemap', label: 'Treemap' },
      { value: 'radar', label: 'Radar' },
      { value: 'bubble', label: 'Bubble' },
      { value: 'gauge', label: 'Gauge' },
    ],
  },
];

/** Flat list of all chart types (for backward compat). */
const _CHART_TYPES = CHART_TYPE_GROUPS.flatMap((g) => g.types);

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
                          {chartTypeIcon(ct.value)}
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
