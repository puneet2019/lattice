import type { Component } from 'solid-js';
import { createSignal, createEffect, Show, For } from 'solid-js';
import { getColumnStats } from '../bridge/tauri';
import type { ColumnStats } from '../bridge/tauri';

export interface ColumnStatsPanelProps {
  /** Name of the active sheet. */
  activeSheet: string;
  /** Currently selected column (0-based). */
  col: number;
  /** Total number of columns in the sheet. */
  totalCols: number;
  /** Called when the panel should close. */
  onClose: () => void;
  /** Called when user navigates to a different column. */
  onColumnChange: (col: number) => void;
}

/** Convert 0-based column index to letter (A, B, ..., Z, AA, ...). */
function colLetter(col: number): string {
  let result = '';
  let c = col;
  do {
    result = String.fromCharCode(65 + (c % 26)) + result;
    c = Math.floor(c / 26) - 1;
  } while (c >= 0);
  return result;
}

/** Format a number for display (up to 4 decimal places). */
function fmt(n: number | null): string {
  if (n === null || n === undefined) return '--';
  if (Number.isInteger(n)) return n.toLocaleString();
  return n.toLocaleString(undefined, { maximumFractionDigits: 4 });
}

const ColumnStatsPanel: Component<ColumnStatsPanelProps> = (props) => {
  const [stats, setStats] = createSignal<ColumnStats | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const fetchStats = async (sheet: string, col: number) => {
    setLoading(true);
    setError(null);
    try {
      const result = await getColumnStats(sheet, col);
      setStats(result);
    } catch (e) {
      setError(String(e));
      setStats(null);
    } finally {
      setLoading(false);
    }
  };

  createEffect(() => {
    void fetchStats(props.activeSheet, props.col);
  });

  const handlePrev = () => {
    if (props.col > 0) {
      props.onColumnChange(props.col - 1);
    }
  };

  const handleNext = () => {
    if (props.col < props.totalCols - 1) {
      props.onColumnChange(props.col + 1);
    }
  };

  const maxBucket = () => {
    const s = stats();
    if (!s || s.histogram.length === 0) return 1;
    return Math.max(...s.histogram, 1);
  };

  return (
    <div class="column-stats-panel">
      <div class="column-stats-header">
        <button
          class="column-stats-nav"
          disabled={props.col <= 0}
          onClick={handlePrev}
          title="Previous column"
        >
          &#9664;
        </button>
        <span class="column-stats-title">Column {colLetter(props.col)}</span>
        <button
          class="column-stats-nav"
          disabled={props.col >= props.totalCols - 1}
          onClick={handleNext}
          title="Next column"
        >
          &#9654;
        </button>
        <button
          class="column-stats-close"
          onClick={props.onClose}
          title="Close"
        >
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
            <line x1="2" y1="2" x2="10" y2="10" />
            <line x1="10" y1="2" x2="2" y2="10" />
          </svg>
        </button>
      </div>

      <Show when={loading()}>
        <div class="column-stats-loading">Loading...</div>
      </Show>

      <Show when={error()}>
        <div class="column-stats-error">{error()}</div>
      </Show>

      <Show when={stats() && !loading()}>
        <div class="column-stats-body">
          <div class="column-stats-row">
            <span class="column-stats-label">Count</span>
            <span class="column-stats-value">{stats()!.count}</span>
          </div>
          <div class="column-stats-row">
            <span class="column-stats-label">Unique</span>
            <span class="column-stats-value">{stats()!.unique}</span>
          </div>
          <div class="column-stats-row">
            <span class="column-stats-label">Sum</span>
            <span class="column-stats-value">{fmt(stats()!.sum)}</span>
          </div>
          <div class="column-stats-row">
            <span class="column-stats-label">Average</span>
            <span class="column-stats-value">{fmt(stats()!.average)}</span>
          </div>
          <div class="column-stats-row">
            <span class="column-stats-label">Median</span>
            <span class="column-stats-value">{fmt(stats()!.median)}</span>
          </div>
          <div class="column-stats-row">
            <span class="column-stats-label">Min</span>
            <span class="column-stats-value">{fmt(stats()!.min)}</span>
          </div>
          <div class="column-stats-row">
            <span class="column-stats-label">Max</span>
            <span class="column-stats-value">{fmt(stats()!.max)}</span>
          </div>
          <div class="column-stats-row">
            <span class="column-stats-label">Std Dev</span>
            <span class="column-stats-value">{fmt(stats()!.std_dev)}</span>
          </div>

          <Show when={stats()!.histogram.length > 0}>
            <div class="column-stats-histogram-label">Distribution</div>
            <div class="column-stats-histogram">
              <For each={stats()!.histogram}>
                {(bucket) => {
                  const height = Math.max((bucket / maxBucket()) * 100, 2);
                  return (
                    <div
                      class="column-stats-bar"
                      style={{ height: `${height}%` }}
                      title={`${bucket}`}
                    />
                  );
                }}
              </For>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default ColumnStatsPanel;
