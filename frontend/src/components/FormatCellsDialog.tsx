import type { Component } from 'solid-js';
import { createSignal, createMemo, For, Show } from 'solid-js';
import type { BorderEdgeUpdate, BordersUpdate, FormatOptions } from '../bridge/tauri';

export interface FormatCellsDialogProps {
  onApply: (format: FormatOptions) => void;
  onClose: () => void;
  /** The raw value of the currently selected cell (used for live preview). */
  cellValue?: string;
}

/** Number format categories with their default format strings. */
const NUMBER_CATEGORIES: { label: string; value: string; examples?: string[] }[] = [
  { label: 'General', value: '', examples: [] },
  { label: 'Number', value: '#,##0.00', examples: ['#,##0', '#,##0.00', '#,##0.000', '0', '0.00'] },
  { label: 'Currency', value: '$#,##0.00', examples: ['$#,##0', '$#,##0.00', '$#,##0.000'] },
  { label: 'Accounting', value: '_($* #,##0.00_)', examples: ['_($* #,##0_)', '_($* #,##0.00_)'] },
  { label: 'Date', value: 'mm/dd/yyyy', examples: ['mm/dd/yyyy', 'yyyy-mm-dd', 'dd/mm/yyyy', 'mm/dd/yy', 'mmmm d, yyyy'] },
  { label: 'Time', value: 'hh:mm:ss', examples: ['hh:mm', 'hh:mm:ss', 'hh:mm AM/PM', 'hh:mm:ss AM/PM'] },
  { label: 'Percentage', value: '0.00%', examples: ['0%', '0.00%', '0.000%'] },
  { label: 'Fraction', value: '# ?/?', examples: ['# ?/?', '# ??/??', '# ???/???'] },
  { label: 'Scientific', value: '0.00E+00', examples: ['0E+00', '0.0E+00', '0.00E+00'] },
  { label: 'Text', value: '@', examples: [] },
  { label: 'Custom', value: '', examples: [] },
];

/** Apply a simple number format pattern to a numeric value for live preview.
 *  This is a best-effort client-side preview; the actual formatting happens in Rust. */
function previewFormat(value: string, format: string): string {
  if (!format || format === '@') return value;
  const num = parseFloat(value);
  if (isNaN(num)) return value;

  // Percentage
  if (format.includes('%')) {
    const decimals = (format.match(/0/g) || []).length - 1;
    return (num * 100).toFixed(Math.max(0, decimals)) + '%';
  }
  // Scientific
  if (format.includes('E+') || format.includes('E-')) {
    const decimals = format.split('.')[1]?.replace(/E.*/, '').length ?? 0;
    return num.toExponential(decimals).toUpperCase();
  }
  // Currency
  if (format.startsWith('$')) {
    const decimals = format.split('.')[1]?.replace(/[^0#]/g, '').length ?? 0;
    return '$' + num.toFixed(decimals).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  }
  // Accounting
  if (format.startsWith('_(')) {
    const decimals = format.split('.')[1]?.replace(/[^0#]/g, '').length ?? 0;
    const formatted = num.toFixed(decimals).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
    return `$ ${formatted} `;
  }
  // Number with commas
  if (format.includes(',')) {
    const decimals = format.split('.')[1]?.replace(/[^0#]/g, '').length ?? 0;
    return num.toFixed(decimals).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  }
  // Plain number with decimals
  if (format.includes('.')) {
    const decimals = format.split('.')[1]?.replace(/[^0#?]/g, '').length ?? 0;
    return num.toFixed(decimals);
  }
  // Fraction (simplified)
  if (format.includes('?/?')) {
    const wholePart = Math.floor(Math.abs(num));
    const frac = Math.abs(num) - wholePart;
    if (frac === 0) return (num < 0 ? '-' : '') + wholePart.toString();
    // Simple fraction approximation
    const denom = format.includes('???/???') ? 999 : format.includes('??/??') ? 99 : 9;
    let bestN = 0, bestD = 1, bestErr = Infinity;
    for (let d = 1; d <= denom; d++) {
      const n = Math.round(frac * d);
      const err = Math.abs(frac - n / d);
      if (err < bestErr) { bestN = n; bestD = d; bestErr = err; }
    }
    const sign = num < 0 ? '-' : '';
    return wholePart > 0 ? `${sign}${wholePart} ${bestN}/${bestD}` : `${sign}${bestN}/${bestD}`;
  }
  // Date formats -- just show the format pattern as-is since we can't parse dates from numbers here
  if (format.includes('mm') || format.includes('dd') || format.includes('yy') || format.includes('hh')) {
    return value;
  }
  // Fallback: show with no decimals
  return Math.round(num).toString();
}

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
  { label: 'Double', value: 'double' },
];

type TabId = 'number' | 'font' | 'fill' | 'alignment' | 'borders';
type EdgeKey = 'top' | 'bottom' | 'left' | 'right';

const FormatCellsDialog: Component<FormatCellsDialogProps> = (props) => {
  const [activeTab, setActiveTab] = createSignal<TabId>('number');

  // Number format
  const [numberFormat, setNumberFormat] = createSignal('');
  const [selectedCategory, setSelectedCategory] = createSignal('General');
  const [customFormat, setCustomFormat] = createSignal('');

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
  const [hAlign, setHAlign] = createSignal<'left' | 'center' | 'right' | 'justify'>('left');
  const [vAlign, setVAlign] = createSignal<'top' | 'middle' | 'bottom'>('middle');
  const [textWrap, setTextWrap] = createSignal<'Overflow' | 'Wrap' | 'Clip'>('Overflow');
  const [textRotation, setTextRotation] = createSignal(0);
  const [indentLevel, setIndentLevel] = createSignal(0);

  /** The currently-active format string (from category or custom). */
  const activeFormat = createMemo(() => {
    if (selectedCategory() === 'Custom') return customFormat();
    return numberFormat();
  });

  /** Live preview of the format applied to the cell value. */
  const formatPreview = createMemo(() => {
    const val = props.cellValue ?? '';
    const fmt = activeFormat();
    if (!val) return '';
    if (!fmt) return val;
    return previewFormat(val, fmt);
  });

  /** Get the example formats for the currently selected category. */
  const categoryExamples = createMemo(() => {
    const cat = NUMBER_CATEGORIES.find((c) => c.label === selectedCategory());
    return cat?.examples ?? [];
  });

  /** Handle selecting a category from the list. */
  const handleCategorySelect = (label: string) => {
    setSelectedCategory(label);
    const cat = NUMBER_CATEGORIES.find((c) => c.label === label);
    if (cat && label !== 'Custom') {
      setNumberFormat(cat.value);
    }
  };

  /** Handle selecting an example format within a category. */
  const handleExampleSelect = (fmt: string) => {
    setNumberFormat(fmt);
  };

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
    const styleMap: Record<string, string> = {
      dashed: 'dashed', dotted: 'dotted', double: 'double',
      thin: 'solid', medium: 'solid', thick: 'solid',
    };
    const s = styleMap[borderStyle()] ?? 'solid';
    const finalW = borderStyle() === 'double' ? 3 : w;
    return `${finalW}px ${s} ${borderColor()}`;
  };

  const handleApply = () => {
    const tab = activeTab();
    const fmt = activeFormat();

    if (tab === 'number' && fmt) {
      props.onApply({ number_format: fmt });
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
      const hVal = hAlign();
      // The FormatOptions h_align only supports left/center/right; treat justify as left with wrap
      const hForFormat = hVal === 'justify' ? 'left' as const : hVal;
      const format: FormatOptions = {
        h_align: hForFormat,
        v_align: vAlign(),
      };
      if (textWrap() !== 'Overflow') format.text_wrap = textWrap();
      props.onApply(format);
    } else if (tab === 'borders') {
      props.onApply({ borders: buildBordersUpdate() });
    } else {
      const format: FormatOptions = {};
      if (fmt) format.number_format = fmt;
      if (fontFamily() !== 'Arial') format.font_family = fontFamily();
      if (fontSize() !== 11) format.font_size = fontSize();
      if (bold()) format.bold = true;
      if (italic()) format.italic = true;
      if (underline()) format.underline = true;
      if (fontColor() !== '#000000') format.font_color = fontColor();
      if (bgColor()) format.bg_color = bgColor();
      const hVal = hAlign();
      const hForFormat = hVal === 'justify' ? 'left' as const : hVal;
      if (hForFormat !== 'left') format.h_align = hForFormat;
      if (vAlign() !== 'middle') format.v_align = vAlign();
      if (textWrap() !== 'Overflow') format.text_wrap = textWrap();
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
            {/* Live preview */}
            <Show when={props.cellValue}>
              <div class="format-dialog-section">
                <label class="format-dialog-label">Preview</label>
                <div class="format-dialog-preview">
                  {formatPreview() || props.cellValue}
                </div>
              </div>
            </Show>

            <div class="format-dialog-section" style={{ display: 'flex', gap: '12px' }}>
              {/* Category list */}
              <div style={{ flex: '0 0 140px' }}>
                <label class="format-dialog-label">Category</label>
                <div class="format-dialog-list" style={{ "max-height": '200px' }}>
                  <For each={NUMBER_CATEGORIES}>
                    {(cat) => (
                      <div
                        class={`format-dialog-list-item ${selectedCategory() === cat.label ? 'active' : ''}`}
                        onClick={() => handleCategorySelect(cat.label)}
                      >
                        {cat.label}
                      </div>
                    )}
                  </For>
                </div>
              </div>

              {/* Format examples for selected category */}
              <div style={{ flex: '1' }}>
                <Show when={selectedCategory() !== 'Custom' && categoryExamples().length > 0}>
                  <label class="format-dialog-label">Format</label>
                  <div class="format-dialog-list" style={{ "max-height": '200px' }}>
                    <For each={categoryExamples()}>
                      {(fmt) => (
                        <div
                          class={`format-dialog-list-item ${numberFormat() === fmt ? 'active' : ''}`}
                          onClick={() => handleExampleSelect(fmt)}
                        >
                          <span style={{ "font-family": 'monospace', "font-size": '11px' }}>{fmt}</span>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>
                <Show when={selectedCategory() === 'Custom'}>
                  <label class="format-dialog-label">Format string</label>
                  <input
                    type="text"
                    class="format-dialog-input"
                    placeholder="e.g. #,##0.00"
                    value={customFormat()}
                    onInput={(e) => setCustomFormat(e.currentTarget.value)}
                  />
                  <div style={{ "margin-top": '8px', color: 'var(--header-text)', "font-size": '11px' }}>
                    Excel-style patterns: #,##0.00 | $#,##0 | 0% | 0.00E+00 | @
                  </div>
                </Show>
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
                <For each={['left', 'center', 'right', 'justify'] as const}>
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
            <div class="format-dialog-section">
              <label class="format-dialog-label">Text wrapping</label>
              <div class="format-dialog-row">
                <For each={['Overflow', 'Wrap', 'Clip'] as const}>
                  {(w) => (
                    <button
                      class={`format-dialog-align-btn ${textWrap() === w ? 'active' : ''}`}
                      onClick={() => setTextWrap(w)}
                    >
                      {w}
                    </button>
                  )}
                </For>
              </div>
            </div>
            <div class="format-dialog-section" style={{ display: 'flex', gap: '16px' }}>
              <div style={{ flex: '1' }}>
                <label class="format-dialog-label">Text rotation (degrees)</label>
                <input
                  type="number"
                  class="format-dialog-input"
                  min={-90}
                  max={90}
                  value={textRotation()}
                  onInput={(e) => setTextRotation(parseInt(e.currentTarget.value) || 0)}
                />
              </div>
              <div style={{ flex: '1' }}>
                <label class="format-dialog-label">Indent level</label>
                <input
                  type="number"
                  class="format-dialog-input"
                  min={0}
                  max={15}
                  value={indentLevel()}
                  onInput={(e) => setIndentLevel(parseInt(e.currentTarget.value) || 0)}
                />
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
