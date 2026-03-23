import type { Component } from 'solid-js';
import { createSignal, onMount, For, Show } from 'solid-js';
import { col_to_letter } from '../bridge/tauri_helpers';
import { getSheetHeaders, createPivotTable } from '../bridge/tauri';
import type { PivotValueInput } from '../bridge/tauri';

export interface PivotDialogProps {
  activeSheet: string;
  /** Selection range: [minRow, minCol, maxRow, maxCol]. */
  selectionRange: [number, number, number, number];
  onClose: () => void;
  /** Called after pivot table creation to refresh the UI. */
  onCreated: (targetSheet: string) => void;
  onStatusChange: (msg: string) => void;
}

/** Aggregation options available in the value field dropdown. */
const AGGREGATION_OPTIONS = ['Sum', 'Count', 'Average', 'Min', 'Max'] as const;

interface ValueFieldEntry {
  col: number;
  aggregation: string;
}

const PivotDialog: Component<PivotDialogProps> = (props) => {
  const [headers, setHeaders] = createSignal<string[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [creating, setCreating] = createSignal(false);
  const [errorMessage, setErrorMessage] = createSignal('');

  // Source range in A1 notation, derived from the selection range.
  const defaultRange = () => {
    const [minRow, minCol, maxRow, maxCol] = props.selectionRange;
    return `${col_to_letter(minCol)}${minRow + 1}:${col_to_letter(maxCol)}${maxRow + 1}`;
  };

  const [sourceRange, setSourceRange] = createSignal(defaultRange());
  const [rowField, setRowField] = createSignal(0);
  const [valueFields, setValueFields] = createSignal<ValueFieldEntry[]>([
    { col: 0, aggregation: 'Sum' },
  ]);
  const [targetSheet, setTargetSheet] = createSignal('Pivot1');

  // Fetch headers on mount.
  onMount(async () => {
    try {
      const hdrs = await getSheetHeaders(props.activeSheet, props.selectionRange[0]);
      setHeaders(hdrs);
      // Set sensible defaults: first column as row field, second as value field.
      if (hdrs.length > 1) {
        setRowField(0);
        setValueFields([{ col: 1, aggregation: 'Sum' }]);
      }
    } catch (e) {
      setErrorMessage(`Failed to load headers: ${e}`);
    }
    setLoading(false);
  });

  /** Column options for dropdowns. */
  const columnOptions = () => {
    const hdrs = headers();
    return hdrs.map((header, idx) => ({
      value: idx,
      label: header ? `${col_to_letter(idx)} - ${header}` : `Column ${col_to_letter(idx)}`,
    }));
  };

  const addValueField = () => {
    const current = valueFields();
    const usedCols = new Set(current.map((e) => e.col));
    let nextCol = 0;
    for (let c = 0; c < headers().length; c++) {
      if (!usedCols.has(c)) {
        nextCol = c;
        break;
      }
    }
    setValueFields([...current, { col: nextCol, aggregation: 'Sum' }]);
  };

  const removeValueField = (idx: number) => {
    const current = valueFields();
    if (current.length <= 1) return;
    setValueFields(current.filter((_, i) => i !== idx));
  };

  const updateValueField = (
    idx: number,
    field: 'col' | 'aggregation',
    value: number | string,
  ) => {
    setValueFields(
      valueFields().map((e, i) =>
        i === idx
          ? {
              ...e,
              [field]: field === 'col' ? Number(value) : value,
            }
          : e,
      ),
    );
  };

  const handleCreate = async () => {
    setCreating(true);
    setErrorMessage('');

    const range = sourceRange().trim();
    if (!range) {
      setErrorMessage('Source range is required.');
      setCreating(false);
      return;
    }

    const target = targetSheet().trim();
    if (!target) {
      setErrorMessage('Target sheet name is required.');
      setCreating(false);
      return;
    }

    const pivotValues: PivotValueInput[] = valueFields().map((vf) => ({
      col: vf.col,
      aggregation: vf.aggregation,
    }));

    try {
      await createPivotTable(
        props.activeSheet,
        range,
        [rowField()],
        pivotValues,
        target,
      );
      props.onStatusChange(`Pivot table created on sheet "${target}"`);
      props.onCreated(target);
      props.onClose();
    } catch (e) {
      setErrorMessage(`Failed to create pivot table: ${e}`);
    }
    setCreating(false);
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
  };

  const selectStyle = {
    flex: '1',
    padding: '4px 8px',
    border: '1px solid var(--grid-border, #e0e0e0)',
    'border-radius': '4px',
    'font-size': '13px',
    background: 'var(--cell-bg, #fff)',
    color: 'var(--cell-text, #202124)',
  };

  const inputStyle = {
    width: '100%',
    padding: '4px 8px',
    border: '1px solid var(--grid-border, #e0e0e0)',
    'border-radius': '4px',
    'font-size': '13px',
    'box-sizing': 'border-box' as const,
  };

  const labelStyle = {
    'font-size': '12px',
    'font-weight': '600' as const,
    color: 'var(--header-text, #5f6368)',
    'margin-bottom': '4px',
    display: 'block',
  };

  return (
    <div
      class="paste-special-backdrop"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
    >
      <div class="paste-special-dialog" style={{ 'min-width': '420px' }}>
        <div class="paste-special-header">
          <h2>Create Pivot Table</h2>
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
          <Show when={loading()}>
            <p style={{ 'font-size': '13px', color: 'var(--header-text, #5f6368)' }}>
              Loading column headers...
            </p>
          </Show>

          <Show when={!loading()}>
            {/* Source range */}
            <div style={{ 'margin-bottom': '12px' }}>
              <label style={labelStyle}>Source range</label>
              <input
                type="text"
                value={sourceRange()}
                onInput={(e) => setSourceRange(e.currentTarget.value)}
                placeholder="e.g. A1:D20"
                style={inputStyle}
              />
            </div>

            {/* Row field */}
            <div style={{ 'margin-bottom': '12px' }}>
              <label style={labelStyle}>Row field (group by)</label>
              <select
                value={rowField()}
                onChange={(e) => setRowField(Number(e.currentTarget.value))}
                style={{ ...selectStyle, width: '100%' }}
              >
                <For each={columnOptions()}>
                  {(opt) => (
                    <option value={opt.value}>{opt.label}</option>
                  )}
                </For>
              </select>
            </div>

            {/* Value fields */}
            <div style={{ 'margin-bottom': '12px' }}>
              <label style={labelStyle}>Value fields</label>
              <For each={valueFields()}>
                {(entry, idx) => (
                  <div
                    style={{
                      display: 'flex',
                      'align-items': 'center',
                      gap: '8px',
                      'margin-bottom': '8px',
                    }}
                  >
                    <select
                      value={entry.col}
                      onChange={(e) =>
                        updateValueField(idx(), 'col', e.currentTarget.value)
                      }
                      style={selectStyle}
                    >
                      <For each={columnOptions()}>
                        {(opt) => (
                          <option value={opt.value}>{opt.label}</option>
                        )}
                      </For>
                    </select>
                    <select
                      value={entry.aggregation}
                      onChange={(e) =>
                        updateValueField(idx(), 'aggregation', e.currentTarget.value)
                      }
                      style={{
                        padding: '4px 8px',
                        border: '1px solid var(--grid-border, #e0e0e0)',
                        'border-radius': '4px',
                        'font-size': '13px',
                        background: 'var(--cell-bg, #fff)',
                        color: 'var(--cell-text, #202124)',
                        'min-width': '90px',
                      }}
                    >
                      <For each={[...AGGREGATION_OPTIONS]}>
                        {(agg) => <option value={agg}>{agg}</option>}
                      </For>
                    </select>
                    {valueFields().length > 1 && (
                      <button
                        class="chart-overlay-close"
                        onClick={() => removeValueField(idx())}
                        title="Remove value field"
                        style={{ 'flex-shrink': '0' }}
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
                onClick={addValueField}
                style={{ 'font-size': '12px' }}
              >
                + Add value field
              </button>
            </div>

            {/* Target sheet */}
            <div style={{ 'margin-bottom': '4px' }}>
              <label style={labelStyle}>Target sheet</label>
              <input
                type="text"
                value={targetSheet()}
                onInput={(e) => setTargetSheet(e.currentTarget.value)}
                placeholder="e.g. Pivot1"
                style={inputStyle}
              />
            </div>

            <Show when={errorMessage()}>
              <div
                style={{
                  'margin-top': '8px',
                  padding: '8px',
                  background: 'var(--error-bg, #fce8e6)',
                  color: 'var(--error-text, #c5221f)',
                  'border-radius': '4px',
                  'font-size': '12px',
                }}
              >
                {errorMessage()}
              </div>
            </Show>
          </Show>
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
            onClick={() => void handleCreate()}
            disabled={creating() || loading()}
          >
            {creating() ? 'Creating...' : 'Create'}
          </button>
        </div>
      </div>
    </div>
  );
};

export default PivotDialog;
