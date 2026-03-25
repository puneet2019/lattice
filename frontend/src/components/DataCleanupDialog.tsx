import type { Component } from 'solid-js';
import { createSignal, For, Show } from 'solid-js';
import { col_to_letter } from '../bridge/tauri_helpers';
import { getRange, setCell } from '../bridge/tauri';

/** Which cleanup tab is active. */
type CleanupTab = 'trim' | 'duplicates' | 'find-fix';

export interface DataCleanupDialogProps {
  activeSheet: string;
  /** Selection range: [minRow, minCol, maxRow, maxCol]. */
  selectionRange: [number, number, number, number];
  onClose: () => void;
  /** Called after cleanup to refresh the grid. */
  onDataChanged: () => void;
  onStatusChange: (msg: string) => void;
}

const DataCleanupDialog: Component<DataCleanupDialogProps> = (props) => {
  const [activeTab, setActiveTab] = createSignal<CleanupTab>('trim');
  const [processing, setProcessing] = createSignal(false);
  const [resultMessage, setResultMessage] = createSignal('');

  // Remove duplicates: which columns to compare
  const [dupColumns, setDupColumns] = createSignal<Set<number>>(new Set());

  // Initialize with all columns selected
  const columnRange = () => {
    const [, minCol, , maxCol] = props.selectionRange;
    const cols: number[] = [];
    for (let c = minCol; c <= maxCol; c++) {
      cols.push(c);
    }
    return cols;
  };

  // Initialize dupColumns with all columns on first render
  const initDupColumns = () => {
    if (dupColumns().size === 0) {
      setDupColumns(new Set(columnRange()));
    }
  };

  const toggleDupColumn = (col: number) => {
    const current = new Set(dupColumns());
    if (current.has(col)) {
      current.delete(col);
    } else {
      current.add(col);
    }
    setDupColumns(current);
  };

  const selectAllDupColumns = () => {
    setDupColumns(new Set(columnRange()));
  };

  const deselectAllDupColumns = () => {
    setDupColumns(new Set<number>());
  };

  /** Trim whitespace from all text cells in the selection. */
  const handleTrimWhitespace = async () => {
    setProcessing(true);
    setResultMessage('');
    const [minRow, minCol, maxRow, maxCol] = props.selectionRange;

    try {
      const data = await getRange(props.activeSheet, minRow, minCol, maxRow, maxCol);
      let trimmedCount = 0;
      const promises: Promise<void>[] = [];

      for (let r = 0; r < data.length; r++) {
        for (let c = 0; c < data[r].length; c++) {
          const cell = data[r][c];
          if (!cell || !cell.value) continue;
          // Skip formulas
          if (cell.formula) continue;

          const trimmed = cell.value.trim();
          if (trimmed !== cell.value) {
            trimmedCount++;
            promises.push(
              setCell(props.activeSheet, minRow + r, minCol + c, trimmed, undefined).catch(() => {}),
            );
          }
        }
      }

      await Promise.all(promises);
      props.onDataChanged();
      setResultMessage(`Trimmed whitespace from ${trimmedCount} cell${trimmedCount !== 1 ? 's' : ''}.`);
      props.onStatusChange(`Trimmed ${trimmedCount} cells`);
    } catch (e) {
      setResultMessage(`Error: ${e}`);
    }
    setProcessing(false);
  };

  /** Remove duplicate rows based on selected columns. */
  const handleRemoveDuplicates = async () => {
    setProcessing(true);
    setResultMessage('');
    const [minRow, minCol, maxRow, maxCol] = props.selectionRange;
    const compareCols = Array.from(dupColumns());

    if (compareCols.length === 0) {
      setResultMessage('Select at least one column to compare.');
      setProcessing(false);
      return;
    }

    try {
      const data = await getRange(props.activeSheet, minRow, minCol, maxRow, maxCol);
      // Build row fingerprints based on selected columns
      const seen = new Set<string>();
      const duplicateRows: number[] = [];

      for (let r = 0; r < data.length; r++) {
        const key = compareCols.map((col) => {
          const colIdx = col - minCol;
          if (colIdx < 0 || colIdx >= data[r].length) return '';
          return data[r][colIdx]?.value ?? '';
        }).join('\t');

        if (seen.has(key)) {
          duplicateRows.push(r);
        } else {
          seen.add(key);
        }
      }

      // Clear duplicate rows (set all cells to empty)
      const promises: Promise<void>[] = [];
      for (const r of duplicateRows) {
        for (let c = minCol; c <= maxCol; c++) {
          promises.push(
            setCell(props.activeSheet, minRow + r, c, '', undefined).catch(() => {}),
          );
        }
      }

      await Promise.all(promises);
      props.onDataChanged();
      const kept = data.length - duplicateRows.length;
      setResultMessage(
        `${duplicateRows.length} duplicate row${duplicateRows.length !== 1 ? 's' : ''} removed. ${kept} unique row${kept !== 1 ? 's' : ''} remain.`,
      );
      props.onStatusChange(`Removed ${duplicateRows.length} duplicate rows`);
    } catch (e) {
      setResultMessage(`Error: ${e}`);
    }
    setProcessing(false);
  };

  /** Find and fix: detect extra internal spaces, leading zeros, etc. */
  const handleFindAndFix = async () => {
    setProcessing(true);
    setResultMessage('');
    const [minRow, minCol, maxRow, maxCol] = props.selectionRange;

    try {
      const data = await getRange(props.activeSheet, minRow, minCol, maxRow, maxCol);
      let fixedCount = 0;
      const promises: Promise<void>[] = [];

      for (let r = 0; r < data.length; r++) {
        for (let c = 0; c < data[r].length; c++) {
          const cell = data[r][c];
          if (!cell || !cell.value) continue;
          if (cell.formula) continue;

          let value = cell.value;
          let changed = false;

          // Fix 1: Trim leading/trailing whitespace
          const trimmed = value.trim();
          if (trimmed !== value) {
            value = trimmed;
            changed = true;
          }

          // Fix 2: Collapse multiple internal spaces to single space
          const collapsed = value.replace(/  +/g, ' ');
          if (collapsed !== value) {
            value = collapsed;
            changed = true;
          }

          // Fix 3: Remove leading zeros from numbers (e.g. "007" -> "7")
          // But keep "0" itself and don't modify non-numeric strings
          if (/^0\d+$/.test(value)) {
            const num = parseInt(value, 10);
            if (!isNaN(num)) {
              value = String(num);
              changed = true;
            }
          }

          if (changed) {
            fixedCount++;
            promises.push(
              setCell(props.activeSheet, minRow + r, minCol + c, value, undefined).catch(() => {}),
            );
          }
        }
      }

      await Promise.all(promises);
      props.onDataChanged();
      setResultMessage(`Fixed ${fixedCount} cell${fixedCount !== 1 ? 's' : ''} (whitespace, extra spaces, leading zeros).`);
      props.onStatusChange(`Fixed ${fixedCount} cells`);
    } catch (e) {
      setResultMessage(`Error: ${e}`);
    }
    setProcessing(false);
  };

  const handleApply = () => {
    switch (activeTab()) {
      case 'trim':
        void handleTrimWhitespace();
        break;
      case 'duplicates':
        void handleRemoveDuplicates();
        break;
      case 'find-fix':
        void handleFindAndFix();
        break;
    }
  };

  const handleBackdropClick = (e: MouseEvent) => {
    if ((e.target as HTMLElement).classList.contains('paste-special-backdrop')) {
      props.onClose();
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      props.onClose();
    }
    if (e.key === 'Enter' && !processing()) {
      e.preventDefault();
      handleApply();
    }
  };

  // Initialize duplicate columns on switch to that tab
  const switchTab = (tab: CleanupTab) => {
    setActiveTab(tab);
    setResultMessage('');
    if (tab === 'duplicates') {
      initDupColumns();
    }
  };

  return (
    <div
      class="paste-special-backdrop"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
    >
      <div class="paste-special-dialog" style={{ "min-width": '400px' }}>
        <div class="paste-special-header">
          <h2>Data Cleanup</h2>
          <button
            class="chart-overlay-close"
            onClick={() => props.onClose()}
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

        <div class="paste-special-body">
          {/* Tab bar */}
          <div class="data-cleanup-tabs">
            <button
              class={`data-cleanup-tab ${activeTab() === 'trim' ? 'active' : ''}`}
              onClick={() => switchTab('trim')}
            >
              Trim whitespace
            </button>
            <button
              class={`data-cleanup-tab ${activeTab() === 'duplicates' ? 'active' : ''}`}
              onClick={() => switchTab('duplicates')}
            >
              Remove duplicates
            </button>
            <button
              class={`data-cleanup-tab ${activeTab() === 'find-fix' ? 'active' : ''}`}
              onClick={() => switchTab('find-fix')}
            >
              Find and fix
            </button>
          </div>

          {/* Tab content */}
          <div class="data-cleanup-content">
            <Show when={activeTab() === 'trim'}>
              <p class="data-cleanup-desc">
                Remove leading and trailing spaces from all text cells in the
                selected range.
              </p>
            </Show>

            <Show when={activeTab() === 'duplicates'}>
              <p class="data-cleanup-desc">
                Remove rows with duplicate values. Select which columns to
                compare:
              </p>
              <div class="data-cleanup-col-actions">
                <button class="data-cleanup-link-btn" onClick={selectAllDupColumns}>
                  Select all
                </button>
                <button class="data-cleanup-link-btn" onClick={deselectAllDupColumns}>
                  Deselect all
                </button>
              </div>
              <div class="data-cleanup-col-list">
                <For each={columnRange()}>
                  {(col) => (
                    <label class="data-cleanup-col-item">
                      <input
                        type="checkbox"
                        checked={dupColumns().has(col)}
                        onChange={() => toggleDupColumn(col)}
                      />
                      <span>Column {col_to_letter(col)}</span>
                    </label>
                  )}
                </For>
              </div>
            </Show>

            <Show when={activeTab() === 'find-fix'}>
              <p class="data-cleanup-desc">
                Automatically detect and fix common data issues:
              </p>
              <ul class="data-cleanup-fix-list">
                <li>Leading and trailing whitespace</li>
                <li>Multiple consecutive spaces</li>
                <li>Leading zeros in numbers (e.g. "007" to "7")</li>
              </ul>
            </Show>

            <Show when={resultMessage()}>
              <div class="data-cleanup-result">
                {resultMessage()}
              </div>
            </Show>
          </div>
        </div>

        <div class="paste-special-footer">
          <button
            class="chart-dialog-btn"
            onClick={() => props.onClose()}
          >
            Close
          </button>
          <button
            class="chart-dialog-btn chart-dialog-btn-primary"
            onClick={handleApply}
            disabled={processing()}
          >
            {processing() ? 'Processing...' : 'Apply'}
          </button>
        </div>
      </div>
    </div>
  );
};

export default DataCleanupDialog;
