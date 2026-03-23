import type { Component } from 'solid-js';
import { createSignal, For } from 'solid-js';
import { col_to_letter } from '../bridge/tauri_helpers';
import { sortRange } from '../bridge/tauri';
import type { SortKeyInput } from '../bridge/tauri';

export interface SortDialogProps {
  activeSheet: string;
  /** Default column to sort by (0-based). */
  defaultCol: number;
  /** Total number of columns in the used range. */
  maxCol: number;
  onClose: () => void;
  onSorted: () => void;
  onStatusChange: (msg: string) => void;
}

interface SortEntry {
  col: number;
  direction: 'asc' | 'desc';
}

const SortDialog: Component<SortDialogProps> = (props) => {
  const [entries, setEntries] = createSignal<SortEntry[]>([
    { col: props.defaultCol, direction: 'asc' },
  ]);
  const [rangeInput, setRangeInput] = createSignal('');

  const columnOptions = () => {
    const opts: { value: number; label: string }[] = [];
    for (let c = 0; c <= props.maxCol; c++) {
      opts.push({ value: c, label: `Column ${col_to_letter(c)}` });
    }
    return opts;
  };

  const addEntry = () => {
    const current = entries();
    const usedCols = new Set(current.map((e) => e.col));
    let nextCol = 0;
    for (let c = 0; c <= props.maxCol; c++) {
      if (!usedCols.has(c)) {
        nextCol = c;
        break;
      }
    }
    setEntries([...current, { col: nextCol, direction: 'asc' }]);
  };

  const removeEntry = (idx: number) => {
    const current = entries();
    if (current.length <= 1) return;
    setEntries(current.filter((_, i) => i !== idx));
  };

  const updateEntry = (idx: number, field: 'col' | 'direction', value: number | string) => {
    setEntries(
      entries().map((e, i) =>
        i === idx
          ? {
              ...e,
              [field]: field === 'col' ? Number(value) : value,
            }
          : e,
      ),
    );
  };

  const handleSort = async () => {
    const keys: SortKeyInput[] = entries().map((e) => ({
      col: e.col,
      direction: e.direction,
    }));
    const range = rangeInput().trim() || null;
    try {
      await sortRange(props.activeSheet, range, keys);
      props.onSorted();
      props.onStatusChange('Sort applied');
      props.onClose();
    } catch (e) {
      props.onStatusChange(`Sort failed: ${e}`);
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
    if (e.key === 'Enter') {
      e.preventDefault();
      void handleSort();
    }
  };

  return (
    <div
      class="paste-special-backdrop"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
    >
      <div class="paste-special-dialog" style={{ "min-width": '360px' }}>
        <div class="paste-special-header">
          <h2>Sort Range</h2>
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
          <div class="paste-special-field" style={{ "margin-bottom": '12px' }}>
            <label class="paste-special-label">Range (leave empty for entire sheet)</label>
            <input
              type="text"
              class="sort-dialog-input"
              value={rangeInput()}
              onInput={(e) => setRangeInput(e.currentTarget.value)}
              placeholder="e.g. A1:D20"
              style={{
                width: '100%',
                padding: '4px 8px',
                border: '1px solid var(--grid-border, #e0e0e0)',
                "border-radius": '4px',
                "font-size": '13px',
                "box-sizing": 'border-box',
              }}
            />
          </div>

          <For each={entries()}>
            {(entry, idx) => (
              <div
                style={{
                  display: 'flex',
                  "align-items": 'center',
                  gap: '8px',
                  "margin-bottom": '8px',
                }}
              >
                <span style={{ "font-size": '12px', color: 'var(--header-text, #5f6368)', "min-width": '50px' }}>
                  {idx() === 0 ? 'Sort by' : 'Then by'}
                </span>
                <select
                  value={entry.col}
                  onChange={(e) => updateEntry(idx(), 'col', e.currentTarget.value)}
                  style={{
                    flex: '1',
                    padding: '4px 8px',
                    border: '1px solid var(--grid-border, #e0e0e0)',
                    "border-radius": '4px',
                    "font-size": '13px',
                    background: 'var(--cell-bg, #fff)',
                    color: 'var(--cell-text, #202124)',
                  }}
                >
                  <For each={columnOptions()}>
                    {(opt) => (
                      <option value={opt.value}>{opt.label}</option>
                    )}
                  </For>
                </select>
                <select
                  value={entry.direction}
                  onChange={(e) => updateEntry(idx(), 'direction', e.currentTarget.value)}
                  style={{
                    padding: '4px 8px',
                    border: '1px solid var(--grid-border, #e0e0e0)',
                    "border-radius": '4px',
                    "font-size": '13px',
                    background: 'var(--cell-bg, #fff)',
                    color: 'var(--cell-text, #202124)',
                  }}
                >
                  <option value="asc">A &rarr; Z</option>
                  <option value="desc">Z &rarr; A</option>
                </select>
                {entries().length > 1 && (
                  <button
                    class="chart-overlay-close"
                    onClick={() => removeEntry(idx())}
                    title="Remove sort key"
                    style={{ "flex-shrink": '0' }}
                  >
                    <svg
                      width="10"
                      height="10"
                      viewBox="0 0 12 12"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.5"
                    >
                      <line x1="2" y1="2" x2="10" y2="10" />
                      <line x1="10" y1="2" x2="2" y2="10" />
                    </svg>
                  </button>
                )}
              </div>
            )}
          </For>

          <button
            class="chart-dialog-btn"
            onClick={addEntry}
            style={{ "margin-top": '4px', "font-size": '12px' }}
          >
            + Add another sort column
          </button>
        </div>

        <div class="paste-special-footer">
          <button
            class="chart-dialog-btn"
            onClick={() => props.onClose()}
          >
            Cancel
          </button>
          <button
            class="chart-dialog-btn chart-dialog-btn-primary"
            onClick={() => void handleSort()}
          >
            Sort
          </button>
        </div>
      </div>
    </div>
  );
};

export default SortDialog;
