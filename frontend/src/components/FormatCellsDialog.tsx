import type { Component } from 'solid-js';
import { createSignal, For, Show } from 'solid-js';
import type { BorderEdgeUpdate, BordersUpdate, FormatOptions } from '../bridge/tauri';

export interface FormatCellsDialogProps {
  onApply: (format: FormatOptions) => void;
  onClose: () => void;
}

const NUMBER_FORMATS: { label: string; value: string }[] = [
  { label: 'General', value: '' },
  { label: 'Number', value: '#,##0.00' },
  { label: 'Currency', value: '$#,##0.00' },
  { label: 'Percentage', value: '0.00%' },
  { label: 'Date', value: 'mm/dd/yyyy' },
  { label: 'Scientific', value: '0.00E+00' },
  { label: 'Text', value: '@' },
];

const FONT_FAMILIES = [
  'Arial', 'Helvetica', 'Times New Roman', 'Courier New', 'Georgia', 'Verdana',
];

const FONT_SIZES = [8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 36, 48, 72];

const FILL_COLORS = [
  '', '#ffffff', '#f3f3f3', '#efefef', '#d9d9d9',
  '#fce5cd', '#fff2cc', '#d9ead3', '#cfe2f3', '#d9d2e9', '#ead1dc',
  '#ea9999', '#f9cb9c', '#ffe599', '#b6d7a8', '#9fc5e8', '#b4a7d6',
  '#e06666', '#f6b26b', '#ffd966', '#93c47d', '#6fa8dc', '#8e7cc3',
  '#ff0000', '#ff9900', '#ffff00', '#00ff00', '#0000ff', '#9900ff',
];

const BORDER_COLORS = [
  '#000000', '#434343', '#666666', '#999999',
  '#cc0000', '#e69138', '#f1c232', '#6aa84f',
  '#3c78d8', '#674ea7', '#a64d79', '#ffffff',
];

const BORDER_STYLES: { label: string; value: string }[] = [
  { label: 'Thin', value: 'thin' },
  { label: 'Medium', value: 'medium' },
  { label: 'Thick', value: 'thick' },
  { label: 'Dashed', value: 'dashed' },
  { label: 'Dotted', value: 'dotted' },
];

type TabId = 'number' | 'font' | 'fill' | 'alignment' | 'borders';
type EdgeKey = 'top' | 'bottom' | 'left' | 'right';

const FormatCellsDialog: Component<FormatCellsDialogProps> = (props) => {
  const [activeTab, setActiveTab] = createSignal<TabId>('number');

  // Number format
  const [numberFormat, setNumberFormat] = createSignal('');

  // Font
  const [fontFamily, setFontFamily] = createSignal('Arial');
  const [fontSize, setFontSize] = createSignal(11);
  const [bold, setBold] = createSignal(false);
  const [italic, setItalic] = createSignal(false);
  const [underline, setUnderline] = createSignal(false);
  const [fontColor, setFontColor] = createSignal('#000000');

  // Fill
  const [bgColor, setBgColor] = createSignal('');

  // Alignment
  const [hAlign, setHAlign] = createSignal<'left' | 'center' | 'right'>('left');
  const [vAlign, setVAlign] = createSignal<'top' | 'middle' | 'bottom'>('middle');

  // Borders
  const [borderTop, setBorderTop] = createSignal(false);
  const [borderBottom, setBorderBottom] = createSignal(false);
  const [borderLeft, setBorderLeft] = createSignal(false);
  const [borderRight, setBorderRight] = createSignal(false);
  const [borderStyle, setBorderStyle] = createSignal('thin');
  const [borderColor, setBorderColor] = createSignal('#000000');

  const toggleEdge = (edge: EdgeKey) => {
    const setters = { top: setBorderTop, bottom: setBorderBottom, left: setBorderLeft, right: setBorderRight };
    const getters = { top: borderTop, bottom: borderBottom, left: borderLeft, right: borderRight };
    setters[edge](!getters[edge]());
  };

  const buildBordersUpdate = (): BordersUpdate => {
    const edges: EdgeKey[] = ['top', 'bottom', 'left', 'right'];
    const getters = { top: borderTop, bottom: borderBottom, left: borderLeft, right: borderRight };
    const result: BordersUpdate = {};
    for (const edge of edges) {
      if (getters[edge]()) {
        (result as Record<string, BorderEdgeUpdate>)[edge] = { style: borderStyle(), color: borderColor() };
      } else {
        (result as Record<string, BorderEdgeUpdate>)[edge] = { style: 'none' };
      }
    }
    return result;
  };

  /** Get CSS border string for the preview edges. */
  const edgeCssStyle = (active: boolean): string => {
    if (!active) return '1px solid var(--grid-border)';
    const w = borderStyle() === 'thick' ? 3 : borderStyle() === 'medium' ? 2 : 1;
    const s = borderStyle() === 'dashed' ? 'dashed' : borderStyle() === 'dotted' ? 'dotted' : 'solid';
    return `${w}px ${s} ${borderColor()}`;
  };

  const handleApply = () => {
    const tab = activeTab();

    if (tab === 'number' && numberFormat()) {
      props.onApply({ number_format: numberFormat() });
    } else if (tab === 'font') {
      props.onApply({
        font_family: fontFamily(),
        font_size: fontSize(),
        bold: bold(),
        italic: italic(),
        underline: underline(),
        font_color: fontColor(),
      });
    } else if (tab === 'fill') {
      props.onApply({ bg_color: bgColor() || undefined });
    } else if (tab === 'alignment') {
      props.onApply({ h_align: hAlign(), v_align: vAlign() });
    } else if (tab === 'borders') {
      props.onApply({ borders: buildBordersUpdate() });
    } else {
      const format: FormatOptions = {};
      if (numberFormat()) format.number_format = numberFormat();
      if (fontFamily() !== 'Arial') format.font_family = fontFamily();
      if (fontSize() !== 11) format.font_size = fontSize();
      if (bold()) format.bold = true;
      if (italic()) format.italic = true;
      if (underline()) format.underline = true;
      if (fontColor() !== '#000000') format.font_color = fontColor();
      if (bgColor()) format.bg_color = bgColor();
      if (hAlign() !== 'left') format.h_align = hAlign();
      if (vAlign() !== 'middle') format.v_align = vAlign();
      props.onApply(format);
    }
  };

  const tabs: { id: TabId; label: string }[] = [
    { id: 'number', label: 'Number' },
    { id: 'font', label: 'Font' },
    { id: 'fill', label: 'Fill' },
    { id: 'alignment', label: 'Alignment' },
    { id: 'borders', label: 'Borders' },
  ];

  return (
    <div class="format-dialog-backdrop" onClick={props.onClose}>
      <div class="format-dialog" onClick={(e) => e.stopPropagation()}>
        <div class="format-dialog-header">
          <h2>Format Cells</h2>
          <button class="chart-overlay-close" onClick={props.onClose}>
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M2 2l8 8M10 2l-8 8" />
            </svg>
          </button>
        </div>

        <div class="format-dialog-tabs">
          <For each={tabs}>
            {(tab) => (
              <button
                class={`format-dialog-tab ${activeTab() === tab.id ? 'active' : ''}`}
                onClick={() => setActiveTab(tab.id)}
              >
                {tab.label}
              </button>
            )}
          </For>
        </div>

        <div class="format-dialog-body">
          <Show when={activeTab() === 'number'}>
            <div class="format-dialog-section">
              <label class="format-dialog-label">Category</label>
              <div class="format-dialog-list">
                <For each={NUMBER_FORMATS}>
                  {(fmt) => (
                    <div
                      class={`format-dialog-list-item ${numberFormat() === fmt.value ? 'active' : ''}`}
                      onClick={() => setNumberFormat(fmt.value)}
                    >
                      {fmt.label}
                    </div>
                  )}
                </For>
              </div>
            </div>
          </Show>

          <Show when={activeTab() === 'font'}>
            <div class="format-dialog-section">
              <label class="format-dialog-label">Font Family</label>
              <select
                class="format-dialog-select"
                value={fontFamily()}
                onChange={(e) => setFontFamily(e.currentTarget.value)}
              >
                <For each={FONT_FAMILIES}>
                  {(f) => <option value={f} style={{ "font-family": f }}>{f}</option>}
                </For>
              </select>
            </div>
            <div class="format-dialog-section">
              <label class="format-dialog-label">Size</label>
              <select
                class="format-dialog-select"
                value={fontSize()}
                onChange={(e) => setFontSize(Number(e.currentTarget.value))}
              >
                <For each={FONT_SIZES}>
                  {(s) => <option value={s}>{s}</option>}
                </For>
              </select>
            </div>
            <div class="format-dialog-section format-dialog-row">
              <label class="format-dialog-checkbox">
                <input type="checkbox" checked={bold()} onChange={(e) => setBold(e.currentTarget.checked)} />
                <strong>Bold</strong>
              </label>
              <label class="format-dialog-checkbox">
                <input type="checkbox" checked={italic()} onChange={(e) => setItalic(e.currentTarget.checked)} />
                <em>Italic</em>
              </label>
              <label class="format-dialog-checkbox">
                <input type="checkbox" checked={underline()} onChange={(e) => setUnderline(e.currentTarget.checked)} />
                <span style={{ "text-decoration": "underline" }}>Underline</span>
              </label>
            </div>
            <div class="format-dialog-section">
              <label class="format-dialog-label">Color</label>
              <input
                type="color"
                value={fontColor()}
                onInput={(e) => setFontColor(e.currentTarget.value)}
                class="format-dialog-color-input"
              />
            </div>
          </Show>

          <Show when={activeTab() === 'fill'}>
            <div class="format-dialog-section">
              <label class="format-dialog-label">Background Color</label>
              <div class="format-dialog-color-grid">
                <For each={FILL_COLORS}>
                  {(color) => (
                    <div
                      class={`toolbar-color-swatch ${bgColor() === color ? 'selected' : ''}`}
                      style={{
                        background: color || '#ffffff',
                        border: bgColor() === color ? '2px solid var(--selection-border)' : '1px solid var(--grid-border)',
                      }}
                      onClick={() => setBgColor(color)}
                      title={color || 'No fill'}
                    >
                      {!color && <span style={{ "font-size": "8px", color: 'var(--danger-color)' }}>X</span>}
                    </div>
                  )}
                </For>
              </div>
            </div>
          </Show>

          <Show when={activeTab() === 'alignment'}>
            <div class="format-dialog-section">
              <label class="format-dialog-label">Horizontal</label>
              <div class="format-dialog-row">
                <For each={['left', 'center', 'right'] as const}>
                  {(a) => (
                    <button
                      class={`format-dialog-align-btn ${hAlign() === a ? 'active' : ''}`}
                      onClick={() => setHAlign(a)}
                    >
                      {a.charAt(0).toUpperCase() + a.slice(1)}
                    </button>
                  )}
                </For>
              </div>
            </div>
            <div class="format-dialog-section">
              <label class="format-dialog-label">Vertical</label>
              <div class="format-dialog-row">
                <For each={['top', 'middle', 'bottom'] as const}>
                  {(a) => (
                    <button
                      class={`format-dialog-align-btn ${vAlign() === a ? 'active' : ''}`}
                      onClick={() => setVAlign(a)}
                    >
                      {a.charAt(0).toUpperCase() + a.slice(1)}
                    </button>
                  )}
                </For>
              </div>
            </div>
          </Show>

          <Show when={activeTab() === 'borders'}>
            {/* Border preview -- click edges to toggle */}
            <div class="format-dialog-section">
              <label class="format-dialog-label">Preview (click edges to toggle)</label>
              <div class="border-preview-container">
                {/* Top edge clickable zone */}
                <div
                  class={`border-preview-edge border-preview-top ${borderTop() ? 'active' : ''}`}
                  onClick={() => toggleEdge('top')}
                  title="Top border"
                />
                {/* Bottom edge clickable zone */}
                <div
                  class={`border-preview-edge border-preview-bottom ${borderBottom() ? 'active' : ''}`}
                  onClick={() => toggleEdge('bottom')}
                  title="Bottom border"
                />
                {/* Left edge clickable zone */}
                <div
                  class={`border-preview-edge border-preview-left ${borderLeft() ? 'active' : ''}`}
                  onClick={() => toggleEdge('left')}
                  title="Left border"
                />
                {/* Right edge clickable zone */}
                <div
                  class={`border-preview-edge border-preview-right ${borderRight() ? 'active' : ''}`}
                  onClick={() => toggleEdge('right')}
                  title="Right border"
                />
                {/* Inner preview box */}
                <div
                  class="border-preview-inner"
                  style={{
                    "border-top": edgeCssStyle(borderTop()),
                    "border-bottom": edgeCssStyle(borderBottom()),
                    "border-left": edgeCssStyle(borderLeft()),
                    "border-right": edgeCssStyle(borderRight()),
                  }}
                >
                  <span style={{ color: 'var(--header-text)', "font-size": '11px' }}>Cell</span>
                </div>
              </div>
            </div>

            {/* Quick presets */}
            <div class="format-dialog-section">
              <label class="format-dialog-label">Presets</label>
              <div class="format-dialog-row">
                <button class="format-dialog-align-btn" onClick={() => { setBorderTop(true); setBorderBottom(true); setBorderLeft(true); setBorderRight(true); }}>All</button>
                <button class="format-dialog-align-btn" onClick={() => { setBorderTop(false); setBorderBottom(false); setBorderLeft(false); setBorderRight(false); }}>None</button>
                <button class="format-dialog-align-btn" onClick={() => { setBorderTop(true); setBorderBottom(true); setBorderLeft(true); setBorderRight(true); }}>Outline</button>
              </div>
            </div>

            {/* Border style */}
            <div class="format-dialog-section">
              <label class="format-dialog-label">Style</label>
              <div class="format-dialog-row">
                <For each={BORDER_STYLES}>
                  {(s) => (
                    <button
                      class={`format-dialog-align-btn ${borderStyle() === s.value ? 'active' : ''}`}
                      onClick={() => setBorderStyle(s.value)}
                    >
                      {s.label}
                    </button>
                  )}
                </For>
              </div>
            </div>

            {/* Border color */}
            <div class="format-dialog-section">
              <label class="format-dialog-label">Color</label>
              <div class="format-dialog-row" style={{ gap: '4px' }}>
                <For each={BORDER_COLORS}>
                  {(color) => (
                    <div
                      class="toolbar-color-swatch"
                      style={{
                        background: color,
                        width: '22px',
                        height: '22px',
                        border: borderColor() === color ? '2px solid var(--selection-border)' : '1px solid var(--grid-border)',
                      }}
                      onClick={() => setBorderColor(color)}
                      title={color}
                    />
                  )}
                </For>
              </div>
            </div>
          </Show>
        </div>

        <div class="format-dialog-footer">
          <button class="chart-dialog-btn" onClick={props.onClose}>Cancel</button>
          <button class="chart-dialog-btn chart-dialog-btn-primary" onClick={handleApply}>Apply</button>
        </div>
      </div>
    </div>
  );
};

export default FormatCellsDialog;
