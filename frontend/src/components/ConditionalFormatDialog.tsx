import type { Component } from 'solid-js';
import { createSignal, For, Show, onMount } from 'solid-js';
import {
  addConditionalFormat,
  listConditionalFormats,
  removeConditionalFormat,
} from '../bridge/tauri';
import type { ConditionalFormatOutput, RuleTypeInput, ConditionalStyleInput } from '../bridge/tauri';
import { col_to_letter } from '../bridge/tauri_helpers';

export interface ConditionalFormatDialogProps {
  activeSheet: string;
  /** Current selection range: [minRow, minCol, maxRow, maxCol]. */
  selRange: [number, number, number, number];
  onClose: () => void;
  onStatusChange: (msg: string) => void;
  onRefresh: () => void;
}

const RULE_KINDS = [
  { label: 'Cell value is...', value: 'cell_value' },
  { label: 'Text contains...', value: 'text_contains' },
  { label: 'Is blank', value: 'is_blank' },
  { label: 'Is not blank', value: 'is_not_blank' },
  { label: 'Is error', value: 'is_error' },
  { label: 'Color scale', value: 'color_scale' },
  { label: 'Data bar', value: 'data_bar' },
  { label: 'Icon set', value: 'icon_set' },
];
const OPERATORS = [
  { label: 'Greater than', value: '>' },  { label: 'Less than', value: '<' },
  { label: 'Equal to', value: '=' },      { label: 'Not equal to', value: '!=' },
  { label: '>= ', value: '>=' },          { label: '<=', value: '<=' },
  { label: 'Between', value: 'between' },
];
const STYLE_COLORS = [
  '#ff0000', '#00aa00', '#0000ff', '#ff9900', '#9900ff',
  '#000000', '#cc0000', '#e69138', '#f1c232', '#6aa84f',
];
const BG_COLORS = [
  '', '#fce5cd', '#fff2cc', '#d9ead3', '#cfe2f3',
  '#f4cccc', '#d9d2e9', '#ead1dc', '#ea9999', '#b6d7a8',
];

const ConditionalFormatDialog: Component<ConditionalFormatDialogProps> = (props) => {
  const [existingRules, setExistingRules] = createSignal<ConditionalFormatOutput[]>([]);
  const [showAddForm, setShowAddForm] = createSignal(false);

  // New rule form state
  const [ruleKind, setRuleKind] = createSignal('cell_value');
  const [operator, setOperator] = createSignal('>');
  const [value1, setValue1] = createSignal('');
  const [value2, setValue2] = createSignal('');
  const [textNeedle, setTextNeedle] = createSignal('');
  const [styleBold, setStyleBold] = createSignal(false);
  const [styleItalic, setStyleItalic] = createSignal(false);
  const [styleFontColor, setStyleFontColor] = createSignal('');
  const [styleBgColor, setStyleBgColor] = createSignal('');

  // Color scale state
  const [csMinColor, setCsMinColor] = createSignal('#ffffff');
  const [csMidColor, setCsMidColor] = createSignal('');
  const [csMaxColor, setCsMaxColor] = createSignal('#ff0000');

  // Data bar state
  const [dbColor, setDbColor] = createSignal('#4285f4');

  // Icon set state
  const [iconSetPreset, setIconSetPreset] = createSignal('arrows');

  const rangeLabel = () => {
    const [r1, c1, r2, c2] = props.selRange;
    if (r1 === r2 && c1 === c2) return `${col_to_letter(c1)}${r1 + 1}`;
    return `${col_to_letter(c1)}${r1 + 1}:${col_to_letter(c2)}${r2 + 1}`;
  };

  const loadRules = async () => {
    try {
      const rules = await listConditionalFormats(props.activeSheet);
      setExistingRules(rules);
    } catch {
      // Backend may not support yet
    }
  };

  onMount(() => {
    void loadRules();
  });

  const handleAddRule = async () => {
    const [r1, c1, r2, c2] = props.selRange;
    const kind = ruleKind();

    const ruleType: RuleTypeInput = { kind };
    if (kind === 'cell_value') {
      ruleType.operator = operator();
      ruleType.value1 = parseFloat(value1()) || 0;
      if (operator() === 'between') {
        ruleType.value2 = parseFloat(value2()) || 0;
      }
    } else if (kind === 'text_contains') {
      ruleType.text = textNeedle();
    } else if (kind === 'color_scale') {
      ruleType.min_color = csMinColor();
      ruleType.max_color = csMaxColor();
      if (csMidColor()) ruleType.mid_color = csMidColor();
    } else if (kind === 'data_bar') {
      ruleType.bar_color = dbColor();
    } else if (kind === 'icon_set') {
      const presets: Record<string, { icons: string[]; thresholds: number[] }> = {
        arrows: { icons: ['\u2191', '\u2192', '\u2193'], thresholds: [67, 33] },
        traffic: { icons: ['\u{1F7E2}', '\u{1F7E1}', '\u{1F534}'], thresholds: [67, 33] },
        flags: { icons: ['\u{1F7E9}', '\u{1F7E8}', '\u{1F7E5}'], thresholds: [67, 33] },
      };
      const preset = presets[iconSetPreset()] ?? presets.arrows;
      ruleType.icons = preset.icons;
      ruleType.thresholds = preset.thresholds;
    }

    const style: ConditionalStyleInput = {};
    // Visual rule types don't use style overrides
    if (kind !== 'color_scale' && kind !== 'data_bar' && kind !== 'icon_set') {
      if (styleBold()) style.bold = true;
      if (styleItalic()) style.italic = true;
      if (styleFontColor()) style.font_color = styleFontColor();
      if (styleBgColor()) style.bg_color = styleBgColor();
    }

    try {
      await addConditionalFormat(props.activeSheet, r1, c1, r2, c2, ruleType, style);
      props.onStatusChange('Conditional format added');
      props.onRefresh();
      setShowAddForm(false);
      void loadRules();
    } catch (e) {
      props.onStatusChange(`Error: ${e}`);
    }
  };

  const handleRemoveRule = async (range: ConditionalFormatOutput, ruleIndex: number) => {
    try {
      await removeConditionalFormat(
        props.activeSheet,
        range.start_row,
        range.start_col,
        range.end_row,
        range.end_col,
        ruleIndex,
      );
      props.onStatusChange('Rule removed');
      props.onRefresh();
      void loadRules();
    } catch (e) {
      props.onStatusChange(`Error: ${e}`);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      props.onClose();
    }
  };

  return (
    <div class="format-dialog-backdrop" onClick={props.onClose} onKeyDown={handleKeyDown} tabIndex={-1}>
      <div class="format-dialog" onClick={(e) => e.stopPropagation()} style={{ width: '480px' }}>
        <div class="format-dialog-header">
          <h2>Conditional Formatting</h2>
          <button class="chart-overlay-close" onClick={props.onClose}>
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M2 2l8 8M10 2l-8 8" />
            </svg>
          </button>
        </div>

        <div class="format-dialog-body">
          {/* Current range */}
          <div class="format-dialog-section">
            <label class="format-dialog-label">Apply to range</label>
            <div style={{ "font-size": "13px", color: "var(--cell-text)", padding: "4px 0" }}>
              {rangeLabel()}
            </div>
          </div>

          {/* Existing rules */}
          <div class="format-dialog-section">
            <label class="format-dialog-label">Rules</label>
            <Show when={existingRules().length === 0}>
              <p style={{ color: 'var(--header-text)', "font-size": '12px' }}>No rules defined yet.</p>
            </Show>
            <For each={existingRules()}>
              {(range) => (
                <div style={{ "margin-bottom": "8px" }}>
                  <div style={{ "font-size": "11px", color: "var(--header-text)", "margin-bottom": "4px" }}>
                    {col_to_letter(range.start_col)}{range.start_row + 1}:{col_to_letter(range.end_col)}{range.end_row + 1}
                  </div>
                  <For each={range.rules}>
                    {(rule, ruleIdx) => (
                      <div class="format-dialog-row" style={{
                        padding: "6px 8px",
                        border: "1px solid var(--grid-border)",
                        "border-radius": "4px",
                        "margin-bottom": "4px",
                        "justify-content": "space-between",
                      }}>
                        <div style={{ "font-size": "12px", flex: "1" }}>
                          <span>{rule.description}</span>
                          <Show when={rule.bg_color}>
                            <span style={{
                              display: "inline-block",
                              width: "12px",
                              height: "12px",
                              background: rule.bg_color ?? '',
                              "border-radius": "2px",
                              "margin-left": "6px",
                              "vertical-align": "middle",
                            }} />
                          </Show>
                          <Show when={rule.bold}><strong style={{ "margin-left": "4px" }}>B</strong></Show>
                          <Show when={rule.italic}><em style={{ "margin-left": "4px" }}>I</em></Show>
                        </div>
                        <button
                          class="toolbar-btn"
                          style={{ width: "20px", height: "20px", "font-size": "11px" }}
                          title="Remove rule"
                          onClick={() => handleRemoveRule(range, ruleIdx())}
                        >
                          X
                        </button>
                      </div>
                    )}
                  </For>
                </div>
              )}
            </For>
          </div>

          {/* Add rule button / form */}
          <Show when={!showAddForm()}>
            <button
              class="chart-dialog-btn chart-dialog-btn-primary"
              onClick={() => setShowAddForm(true)}
              style={{ "margin-bottom": "8px" }}
            >
              Add Rule
            </button>
          </Show>

          <Show when={showAddForm()}>
            <div style={{ border: "1px solid var(--grid-border)", "border-radius": "6px", padding: "12px", "margin-bottom": "8px" }}>
              {/* Rule type */}
              <div class="format-dialog-section">
                <label class="format-dialog-label">Rule type</label>
                <select
                  class="format-dialog-select"
                  value={ruleKind()}
                  onChange={(e) => setRuleKind(e.currentTarget.value)}
                >
                  <For each={RULE_KINDS}>
                    {(k) => <option value={k.value}>{k.label}</option>}
                  </For>
                </select>
              </div>

              {/* Cell value options */}
              <Show when={ruleKind() === 'cell_value'}>
                <div class="format-dialog-section">
                  <label class="format-dialog-label">Operator</label>
                  <select
                    class="format-dialog-select"
                    value={operator()}
                    onChange={(e) => setOperator(e.currentTarget.value)}
                  >
                    <For each={OPERATORS}>
                      {(op) => <option value={op.value}>{op.label}</option>}
                    </For>
                  </select>
                </div>
                <div class="format-dialog-section">
                  <label class="format-dialog-label">Value</label>
                  <input
                    type="number"
                    class="format-dialog-select"
                    value={value1()}
                    onInput={(e) => setValue1(e.currentTarget.value)}
                    placeholder="Value"
                  />
                </div>
                <Show when={operator() === 'between'}>
                  <div class="format-dialog-section">
                    <label class="format-dialog-label">And</label>
                    <input
                      type="number"
                      class="format-dialog-select"
                      value={value2()}
                      onInput={(e) => setValue2(e.currentTarget.value)}
                      placeholder="Value 2"
                    />
                  </div>
                </Show>
              </Show>

              {/* Text contains */}
              <Show when={ruleKind() === 'text_contains'}>
                <div class="format-dialog-section">
                  <label class="format-dialog-label">Contains text</label>
                  <input
                    type="text"
                    class="format-dialog-select"
                    value={textNeedle()}
                    onInput={(e) => setTextNeedle(e.currentTarget.value)}
                    placeholder="Search text"
                  />
                </div>
              </Show>

              {/* Color scale */}
              <Show when={ruleKind() === 'color_scale'}>
                <div class="format-dialog-section">
                  <label class="format-dialog-label">Min color</label>
                  <input type="color" value={csMinColor()} onInput={(e) => setCsMinColor(e.currentTarget.value)} style={{ width: '60px', height: '28px' }} />
                </div>
                <div class="format-dialog-section">
                  <label class="format-dialog-label">Mid color (optional)</label>
                  <div class="format-dialog-row" style={{ gap: '6px', "align-items": "center" }}>
                    <input type="color" value={csMidColor() || '#ffff00'} onInput={(e) => setCsMidColor(e.currentTarget.value)} style={{ width: '60px', height: '28px' }} />
                    <Show when={csMidColor()}>
                      <button class="toolbar-btn" style={{ "font-size": "11px", width: "20px", height: "20px" }} title="Clear mid color" onClick={() => setCsMidColor('')}>X</button>
                    </Show>
                  </div>
                </div>
                <div class="format-dialog-section">
                  <label class="format-dialog-label">Max color</label>
                  <input type="color" value={csMaxColor()} onInput={(e) => setCsMaxColor(e.currentTarget.value)} style={{ width: '60px', height: '28px' }} />
                </div>
                {/* Preview gradient */}
                <div style={{ height: '16px', "border-radius": '4px', margin: '4px 0', background: `linear-gradient(to right, ${csMinColor()}${csMidColor() ? `, ${csMidColor()}` : ''}, ${csMaxColor()})` }} />
              </Show>

              {/* Data bar */}
              <Show when={ruleKind() === 'data_bar'}>
                <div class="format-dialog-section">
                  <label class="format-dialog-label">Bar color</label>
                  <input type="color" value={dbColor()} onInput={(e) => setDbColor(e.currentTarget.value)} style={{ width: '60px', height: '28px' }} />
                </div>
                {/* Preview bar */}
                <div style={{ height: '16px', "border-radius": '4px', margin: '4px 0', background: dbColor(), opacity: '0.4', width: '70%' }} />
              </Show>

              {/* Icon set */}
              <Show when={ruleKind() === 'icon_set'}>
                <div class="format-dialog-section">
                  <label class="format-dialog-label">Icon preset</label>
                  <select class="format-dialog-select" value={iconSetPreset()} onChange={(e) => setIconSetPreset(e.currentTarget.value)}>
                    <option value="arrows">{'\u2191 \u2192 \u2193'} Arrows</option>
                    <option value="traffic">{'\u{1F7E2} \u{1F7E1} \u{1F534}'} Traffic lights</option>
                    <option value="flags">{'\u{1F7E9} \u{1F7E8} \u{1F7E5}'} Flags</option>
                  </select>
                </div>
              </Show>

              {/* Style (only for non-visual rule types) */}
              <Show when={ruleKind() !== 'color_scale' && ruleKind() !== 'data_bar' && ruleKind() !== 'icon_set'}>
              <div class="format-dialog-section">
                <label class="format-dialog-label">Format to apply</label>
                <div class="format-dialog-row">
                  <label class="format-dialog-checkbox">
                    <input type="checkbox" checked={styleBold()} onChange={(e) => setStyleBold(e.currentTarget.checked)} />
                    <strong>Bold</strong>
                  </label>
                  <label class="format-dialog-checkbox">
                    <input type="checkbox" checked={styleItalic()} onChange={(e) => setStyleItalic(e.currentTarget.checked)} />
                    <em>Italic</em>
                  </label>
                </div>
              </div>

              <div class="format-dialog-section">
                <label class="format-dialog-label">Font color</label>
                <div class="format-dialog-row" style={{ gap: '3px' }}>
                  <div
                    class="toolbar-color-swatch toolbar-color-none"
                    style={{ width: '22px', height: '22px', border: !styleFontColor() ? '2px solid var(--selection-border)' : '1px solid var(--grid-border)' }}
                    onClick={() => setStyleFontColor('')}
                    title="Default"
                  >
                    <span style={{ "font-size": "8px" }}>-</span>
                  </div>
                  <For each={STYLE_COLORS}>
                    {(color) => (
                      <div
                        class="toolbar-color-swatch"
                        style={{
                          background: color,
                          width: '22px',
                          height: '22px',
                          border: styleFontColor() === color ? '2px solid var(--selection-border)' : '1px solid var(--grid-border)',
                        }}
                        onClick={() => setStyleFontColor(color)}
                        title={color}
                      />
                    )}
                  </For>
                </div>
              </div>

              <div class="format-dialog-section">
                <label class="format-dialog-label">Background color</label>
                <div class="format-dialog-row" style={{ gap: '3px' }}>
                  <div
                    class="toolbar-color-swatch toolbar-color-none"
                    style={{ width: '22px', height: '22px', border: !styleBgColor() ? '2px solid var(--selection-border)' : '1px solid var(--grid-border)' }}
                    onClick={() => setStyleBgColor('')}
                    title="None"
                  >
                    <span style={{ "font-size": "8px" }}>X</span>
                  </div>
                  <For each={BG_COLORS.slice(1)}>
                    {(color) => (
                      <div
                        class="toolbar-color-swatch"
                        style={{
                          background: color,
                          width: '22px',
                          height: '22px',
                          border: styleBgColor() === color ? '2px solid var(--selection-border)' : '1px solid var(--grid-border)',
                        }}
                        onClick={() => setStyleBgColor(color)}
                        title={color}
                      />
                    )}
                  </For>
                </div>
              </div>
              </Show>

              <div class="format-dialog-row" style={{ "margin-top": "8px", gap: "8px" }}>
                <button class="chart-dialog-btn chart-dialog-btn-primary" onClick={handleAddRule}>Add</button>
                <button class="chart-dialog-btn" onClick={() => setShowAddForm(false)}>Cancel</button>
              </div>
            </div>
          </Show>
        </div>

        <div class="format-dialog-footer">
          <button class="chart-dialog-btn" onClick={props.onClose}>Done</button>
        </div>
      </div>
    </div>
  );
};

export default ConditionalFormatDialog;
