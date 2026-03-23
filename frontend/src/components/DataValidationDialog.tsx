import type { Component } from 'solid-js';
import { createSignal, Show, For, onMount } from 'solid-js';
import {
  setValidation,
  getValidation,
  removeValidation,
} from '../bridge/tauri';
import type { ValidationData } from '../bridge/tauri';

export interface DataValidationDialogProps {
  activeSheet: string;
  row: number;
  col: number;
  cellRef: string;
  onClose: () => void;
  onSaved: () => void;
}

type RuleType = 'list' | 'number' | 'text_length' | 'date' | 'custom';

const RULE_TYPE_LABELS: { id: RuleType; label: string }[] = [
  { id: 'list', label: 'List of items' },
  { id: 'number', label: 'Number' },
  { id: 'text_length', label: 'Text length' },
  { id: 'date', label: 'Date' },
  { id: 'custom', label: 'Custom formula' },
];

const DataValidationDialog: Component<DataValidationDialogProps> = (props) => {
  const [ruleType, setRuleType] = createSignal<RuleType>('list');
  const [listItems, setListItems] = createSignal('');
  const [minVal, setMinVal] = createSignal('');
  const [maxVal, setMaxVal] = createSignal('');
  const [minDate, setMinDate] = createSignal('');
  const [maxDate, setMaxDate] = createSignal('');
  const [formula, setFormula] = createSignal('');
  const [allowBlank, setAllowBlank] = createSignal(true);
  const [errorMessage, setErrorMessage] = createSignal('');
  const [hasExisting, setHasExisting] = createSignal(false);

  onMount(async () => {
    try {
      const existing = await getValidation(props.activeSheet, props.row, props.col);
      if (existing) {
        setHasExisting(true);
        setRuleType(existing.rule_type as RuleType);
        setListItems(existing.list_items ?? '');
        setMinVal(existing.min != null ? String(existing.min) : '');
        setMaxVal(existing.max != null ? String(existing.max) : '');
        setMinDate(existing.min_date ?? '');
        setMaxDate(existing.max_date ?? '');
        setFormula(existing.formula ?? '');
        setAllowBlank(existing.allow_blank);
        setErrorMessage(existing.error_message ?? '');
      }
    } catch {
      // Backend not available (browser dev mode)
    }
  });

  const handleSave = async () => {
    try {
      await setValidation(
        props.activeSheet,
        props.row,
        props.col,
        ruleType(),
        listItems() || undefined,
        minVal() ? Number(minVal()) : undefined,
        maxVal() ? Number(maxVal()) : undefined,
        minDate() || undefined,
        maxDate() || undefined,
        formula() || undefined,
        allowBlank(),
        errorMessage() || undefined,
      );
      props.onSaved();
    } catch (e) {
      console.error('Failed to save validation:', e);
    }
  };

  const handleRemove = async () => {
    try {
      await removeValidation(props.activeSheet, props.row, props.col);
      props.onSaved();
    } catch {
      // Ignore
    }
  };

  return (
    <div class="format-dialog-backdrop" onClick={props.onClose}>
      <div class="format-dialog" onClick={(e) => e.stopPropagation()} style={{ width: '420px' }}>
        <div class="format-dialog-header">
          <h2>Data Validation</h2>
          <button class="chart-overlay-close" onClick={props.onClose}>
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M2 2l8 8M10 2l-8 8" />
            </svg>
          </button>
        </div>

        <div class="format-dialog-body">
          <div class="format-dialog-section">
            <label class="format-dialog-label">Cell: {props.cellRef}</label>
          </div>

          <div class="format-dialog-section">
            <label class="format-dialog-label">Criteria</label>
            <select
              class="format-dialog-select"
              value={ruleType()}
              onChange={(e) => setRuleType(e.currentTarget.value as RuleType)}
            >
              <For each={RULE_TYPE_LABELS}>
                {(rt) => <option value={rt.id}>{rt.label}</option>}
              </For>
            </select>
          </div>

          {/* List items */}
          <Show when={ruleType() === 'list'}>
            <div class="format-dialog-section">
              <label class="format-dialog-label">Items (comma-separated)</label>
              <input
                type="text"
                class="format-dialog-input"
                value={listItems()}
                onInput={(e) => setListItems(e.currentTarget.value)}
                placeholder="Option 1, Option 2, Option 3"
              />
            </div>
          </Show>

          {/* Number range */}
          <Show when={ruleType() === 'number'}>
            <div class="format-dialog-section format-dialog-row">
              <div style={{ flex: '1' }}>
                <label class="format-dialog-label">Min</label>
                <input
                  type="number"
                  class="format-dialog-input"
                  value={minVal()}
                  onInput={(e) => setMinVal(e.currentTarget.value)}
                  placeholder="(none)"
                />
              </div>
              <div style={{ flex: '1' }}>
                <label class="format-dialog-label">Max</label>
                <input
                  type="number"
                  class="format-dialog-input"
                  value={maxVal()}
                  onInput={(e) => setMaxVal(e.currentTarget.value)}
                  placeholder="(none)"
                />
              </div>
            </div>
          </Show>

          {/* Text length */}
          <Show when={ruleType() === 'text_length'}>
            <div class="format-dialog-section format-dialog-row">
              <div style={{ flex: '1' }}>
                <label class="format-dialog-label">Min length</label>
                <input
                  type="number"
                  class="format-dialog-input"
                  value={minVal()}
                  onInput={(e) => setMinVal(e.currentTarget.value)}
                  placeholder="(none)"
                  min="0"
                />
              </div>
              <div style={{ flex: '1' }}>
                <label class="format-dialog-label">Max length</label>
                <input
                  type="number"
                  class="format-dialog-input"
                  value={maxVal()}
                  onInput={(e) => setMaxVal(e.currentTarget.value)}
                  placeholder="(none)"
                  min="0"
                />
              </div>
            </div>
          </Show>

          {/* Date range */}
          <Show when={ruleType() === 'date'}>
            <div class="format-dialog-section format-dialog-row">
              <div style={{ flex: '1' }}>
                <label class="format-dialog-label">After</label>
                <input
                  type="date"
                  class="format-dialog-input"
                  value={minDate()}
                  onInput={(e) => setMinDate(e.currentTarget.value)}
                />
              </div>
              <div style={{ flex: '1' }}>
                <label class="format-dialog-label">Before</label>
                <input
                  type="date"
                  class="format-dialog-input"
                  value={maxDate()}
                  onInput={(e) => setMaxDate(e.currentTarget.value)}
                />
              </div>
            </div>
          </Show>

          {/* Custom formula */}
          <Show when={ruleType() === 'custom'}>
            <div class="format-dialog-section">
              <label class="format-dialog-label">Formula (must evaluate to TRUE)</label>
              <input
                type="text"
                class="format-dialog-input"
                value={formula()}
                onInput={(e) => setFormula(e.currentTarget.value)}
                placeholder="=A1>0"
              />
            </div>
          </Show>

          <div class="format-dialog-section">
            <label class="format-dialog-checkbox">
              <input
                type="checkbox"
                checked={allowBlank()}
                onChange={(e) => setAllowBlank(e.currentTarget.checked)}
              />
              Allow blank
            </label>
          </div>

          <div class="format-dialog-section">
            <label class="format-dialog-label">Error message (optional)</label>
            <input
              type="text"
              class="format-dialog-input"
              value={errorMessage()}
              onInput={(e) => setErrorMessage(e.currentTarget.value)}
              placeholder="Invalid value"
            />
          </div>
        </div>

        <div class="format-dialog-footer">
          <Show when={hasExisting()}>
            <button
              class="chart-dialog-btn"
              style={{ color: 'var(--danger-color, #d93025)' }}
              onClick={handleRemove}
            >
              Remove
            </button>
          </Show>
          <div style={{ flex: '1' }} />
          <button class="chart-dialog-btn" onClick={props.onClose}>Cancel</button>
          <button class="chart-dialog-btn chart-dialog-btn-primary" onClick={handleSave}>Save</button>
        </div>
      </div>
    </div>
  );
};

export default DataValidationDialog;
