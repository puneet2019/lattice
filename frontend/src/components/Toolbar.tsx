import type { Component } from 'solid-js';
import { createSignal, For, Show, onMount, onCleanup } from 'solid-js';
import type { BordersUpdate } from '../bridge/tauri';

export interface ToolbarProps {
  onBold: () => void;
  onItalic: () => void;
  onUnderline: () => void;
  onStrikethrough: () => void;
  onFontSize: (size: number) => void;
  onFontFamily: (family: string) => void;
  onFontColor: (color: string) => void;
  onBgColor: (color: string) => void;
  onBorders: (borders: BordersUpdate) => void;
  onAlign: (align: 'left' | 'center' | 'right') => void;
  onVAlign: (align: 'top' | 'middle' | 'bottom') => void;
  onTextWrap: (wrap: 'Overflow' | 'Wrap' | 'Clip') => void;
  onNumberFormat: (format: string) => void;
  onUndo: () => void;
  onRedo: () => void;
  onFreezeToggle: () => void;
  onSplitToggle: () => void;
  onInsertChart: () => void;
  onFilterToggle: () => void;
  onConditionalFormat: () => void;
  onPaintFormat: () => void;
  onMerge: () => void;
  onUnmerge: () => void;
  onIndent: () => void;
  onOutdent: () => void;
  onTextRotation: (degrees: number) => void;
  onInsertFunction: (fn: string) => void;
  boldActive: boolean;
  italicActive: boolean;
  underlineActive: boolean;
  strikethroughActive: boolean;
  freezeActive: boolean;
  splitActive: boolean;
  filterActive: boolean;
  paintFormatActive: boolean;
  currentFontFamily?: string;
}

const FONT_SIZES = [6, 7, 8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72];

const FONT_FAMILIES = [
  'Arial',
  'Helvetica',
  'Times New Roman',
  'Courier New',
  'Georgia',
  'Verdana',
];

const COMMON_FUNCTIONS = ['SUM', 'AVERAGE', 'COUNT', 'MAX', 'MIN'];

/** Validate a CSS hex color string (3, 4, 6, or 8 hex digits). */
function isValidHexColor(s: string): boolean {
  return /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(s);
}

/** Normalise user input into a # prefixed hex string. */
function normaliseHex(raw: string): string {
  const trimmed = raw.trim();
  return trimmed.startsWith('#') ? trimmed : `#${trimmed}`;
}

const PRESET_COLORS = [
  '#000000', '#434343', '#666666', '#999999', '#b7b7b7', '#cccccc', '#d9d9d9', '#efefef', '#f3f3f3', '#ffffff',
  '#980000', '#ff0000', '#ff9900', '#ffff00', '#00ff00', '#00ffff', '#4a86e8', '#0000ff', '#9900ff', '#ff00ff',
  '#e6b8af', '#f4cccc', '#fce5cd', '#fff2cc', '#d9ead3', '#d0e0e3', '#c9daf8', '#cfe2f3', '#d9d2e9', '#ead1dc',
  '#dd7e6b', '#ea9999', '#f9cb9c', '#ffe599', '#b6d7a8', '#a2c4c9', '#a4c2f4', '#9fc5e8', '#b4a7d6', '#d5a6bd',
  '#cc4125', '#e06666', '#f6b26b', '#ffd966', '#93c47d', '#76a5af', '#6d9eeb', '#6fa8dc', '#8e7cc3', '#c27ba0',
  '#a61c00', '#cc0000', '#e69138', '#f1c232', '#6aa84f', '#45818e', '#3c78d8', '#3d85c6', '#674ea7', '#a64d79',
  '#85200c', '#990000', '#b45f06', '#bf9000', '#38761d', '#134f5c', '#1155cc', '#0b5394', '#351c75', '#741b47',
];

const Toolbar: Component<ToolbarProps> = (props) => {
  const [showFontFamilyDropdown, setShowFontFamilyDropdown] = createSignal(false);
  const [showFontSizeDropdown, setShowFontSizeDropdown] = createSignal(false);
  const [showFontColorPicker, setShowFontColorPicker] = createSignal(false);
  const [showBgColorPicker, setShowBgColorPicker] = createSignal(false);
  const [showBordersDropdown, setShowBordersDropdown] = createSignal(false);
  const [showTextWrapDropdown, setShowTextWrapDropdown] = createSignal(false);
  const [showFunctionDropdown, setShowFunctionDropdown] = createSignal(false);
  const [showMergeDropdown, setShowMergeDropdown] = createSignal(false);
  const [showTextRotationDropdown, setShowTextRotationDropdown] = createSignal(false);
  const [customRotation, setCustomRotation] = createSignal('');
  const [currentFontSize, setCurrentFontSize] = createSignal(11);
  const [lastFontColor, setLastFontColor] = createSignal('#000000');
  const [lastBgColor, setLastBgColor] = createSignal('#ffff00');
  const [fontHexInput, setFontHexInput] = createSignal('');
  const [bgHexInput, setBgHexInput] = createSignal('');

  let toolbarRef: HTMLDivElement | undefined;

  const currentFamily = () => props.currentFontFamily ?? 'Arial';

  const anyDropdownOpen = () =>
    showFontFamilyDropdown() || showFontSizeDropdown() ||
    showFontColorPicker() || showBgColorPicker() ||
    showBordersDropdown() || showTextWrapDropdown() ||
    showFunctionDropdown() || showMergeDropdown() ||
    showTextRotationDropdown();

  const closeAllDropdowns = () => {
    setShowFontFamilyDropdown(false);
    setShowFontSizeDropdown(false);
    setShowFontColorPicker(false);
    setShowBgColorPicker(false);
    setShowBordersDropdown(false);
    setShowTextWrapDropdown(false);
    setShowFunctionDropdown(false);
    setShowMergeDropdown(false);
    setShowTextRotationDropdown(false);
  };

  // -----------------------------------------------------------------------
  // Global listeners: close dropdowns on click-outside, Escape, window blur
  // -----------------------------------------------------------------------
  const handleDocumentMouseDown = (e: MouseEvent) => {
    if (!anyDropdownOpen()) return;
    // If the click is inside a toolbar-dropdown, let the dropdown handle it
    const target = e.target as HTMLElement;
    if (target.closest('.toolbar-dropdown')) return;
    closeAllDropdowns();
  };

  const handleDocumentKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && anyDropdownOpen()) {
      closeAllDropdowns();
      e.preventDefault();
    }
  };

  const handleWindowBlur = () => {
    closeAllDropdowns();
  };

  onMount(() => {
    document.addEventListener('mousedown', handleDocumentMouseDown);
    document.addEventListener('keydown', handleDocumentKeyDown);
    window.addEventListener('blur', handleWindowBlur);
  });

  onCleanup(() => {
    document.removeEventListener('mousedown', handleDocumentMouseDown);
    document.removeEventListener('keydown', handleDocumentKeyDown);
    window.removeEventListener('blur', handleWindowBlur);
  });

  // -----------------------------------------------------------------------
  // Handlers
  // -----------------------------------------------------------------------

  const handleFontFamilySelect = (family: string) => {
    setShowFontFamilyDropdown(false);
    props.onFontFamily(family);
  };

  const handleFontSizeSelect = (size: number) => {
    setCurrentFontSize(size);
    setShowFontSizeDropdown(false);
    props.onFontSize(size);
  };

  const handleFontColor = (color: string) => {
    setLastFontColor(color);
    setShowFontColorPicker(false);
    props.onFontColor(color);
  };

  const handleBgColor = (color: string) => {
    if (color) setLastBgColor(color);
    setShowBgColorPicker(false);
    props.onBgColor(color);
  };

  /** Apply brief active flash to a number format button. */
  const handleNumberFormat = (btn: HTMLButtonElement, format: string) => {
    btn.classList.add('active');
    setTimeout(() => btn.classList.remove('active'), 150);
    props.onNumberFormat(format);
  };

  const thinEdge = { style: 'thin', color: '#000000' };
  const noneEdge = { style: 'none' };

  const BORDER_PRESETS: { label: string; borders: BordersUpdate }[] = [
    { label: 'All borders', borders: { top: thinEdge, bottom: thinEdge, left: thinEdge, right: thinEdge } },
    { label: 'Outer borders', borders: { top: thinEdge, bottom: thinEdge, left: thinEdge, right: thinEdge } },
    { label: 'No borders', borders: { top: noneEdge, bottom: noneEdge, left: noneEdge, right: noneEdge } },
    { label: 'Top border', borders: { top: thinEdge } },
    { label: 'Bottom border', borders: { bottom: thinEdge } },
    { label: 'Left border', borders: { left: thinEdge } },
    { label: 'Right border', borders: { right: thinEdge } },
  ];

  const handleBorderPreset = (borders: BordersUpdate) => {
    setShowBordersDropdown(false);
    props.onBorders(borders);
  };

  return (
    <div class="toolbar" ref={toolbarRef} role="toolbar" aria-label="Formatting toolbar">
      {/* Undo / Redo */}
      <button class="toolbar-btn" title="Undo (Cmd+Z)" aria-label="Undo" onClick={props.onUndo}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M3 8h8a3 3 0 0 1 0 6H9" />
          <path d="M5 5L3 8l2 3" />
        </svg>
      </button>
      <button class="toolbar-btn" title="Redo (Cmd+Shift+Z)" aria-label="Redo" onClick={props.onRedo}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M13 8H5a3 3 0 0 0 0 6h2" />
          <path d="M11 5l2 3-2 3" />
        </svg>
      </button>

      <div class="toolbar-separator" />

      {/* Font Family */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn toolbar-font-family-btn"
          title="Font family"
          aria-label="Font family"
          onClick={() => { const wasOpen = showFontFamilyDropdown(); closeAllDropdowns(); if (!wasOpen) setShowFontFamilyDropdown(true); }}
        >
          {currentFamily()}
        </button>
        <Show when={showFontFamilyDropdown()}>
          <div class="toolbar-dropdown-menu toolbar-font-family-menu">
            <For each={FONT_FAMILIES}>
              {(family) => (
                <div
                  class={`toolbar-dropdown-item ${family === currentFamily() ? 'active' : ''}`}
                  style={{ "font-family": family }}
                  onClick={() => handleFontFamilySelect(family)}
                >
                  {family}
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>

      {/* Font Size */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn toolbar-btn-wide"
          title="Font size"
          aria-label="Font size"
          onClick={() => { const wasOpen = showFontSizeDropdown(); closeAllDropdowns(); if (!wasOpen) setShowFontSizeDropdown(true); }}
        >
          {currentFontSize()}
        </button>
        <Show when={showFontSizeDropdown()}>
          <div class="toolbar-dropdown-menu">
            <input
              type="number"
              min="1"
              max="400"
              placeholder="Custom"
              class="toolbar-font-size-input"
              style={{
                width: '100%',
                padding: '4px 8px',
                border: 'none',
                'border-bottom': '1px solid var(--grid-border, #e0e0e0)',
                'font-size': '12px',
                outline: 'none',
                'box-sizing': 'border-box',
                background: 'transparent',
                color: 'inherit',
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  const val = parseInt(e.currentTarget.value, 10);
                  if (val > 0 && val <= 400) {
                    handleFontSizeSelect(val);
                  }
                }
                e.stopPropagation();
              }}
              onClick={(e) => e.stopPropagation()}
            />
            <For each={FONT_SIZES}>
              {(size) => (
                <div
                  class={`toolbar-dropdown-item ${size === currentFontSize() ? 'active' : ''}`}
                  onClick={() => handleFontSizeSelect(size)}
                >
                  {size}
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>

      <div class="toolbar-separator" />

      {/* Bold / Italic / Underline */}
      <button
        class={`toolbar-btn ${props.boldActive ? 'active' : ''}`}
        title="Bold (Cmd+B)"
        aria-label="Bold"
        aria-pressed={props.boldActive}
        onClick={props.onBold}
      >
        <strong>B</strong>
      </button>
      <button
        class={`toolbar-btn ${props.italicActive ? 'active' : ''}`}
        title="Italic (Cmd+I)"
        aria-label="Italic"
        aria-pressed={props.italicActive}
        onClick={props.onItalic}
      >
        <em>I</em>
      </button>
      <button
        class={`toolbar-btn ${props.underlineActive ? 'active' : ''}`}
        title="Underline (Cmd+U)"
        aria-label="Underline"
        aria-pressed={props.underlineActive}
        onClick={props.onUnderline}
      >
        <span style={{ "text-decoration": "underline" }}>U</span>
      </button>
      <button
        class={`toolbar-btn ${props.strikethroughActive ? 'active' : ''}`}
        title="Strikethrough (Cmd+Shift+K)"
        aria-label="Strikethrough"
        aria-pressed={props.strikethroughActive}
        onClick={props.onStrikethrough}
      >
        <span style={{ "text-decoration": "line-through" }}>S</span>
      </button>

      <div class="toolbar-separator" />

      {/* Paint Format */}
      <button
        class={`toolbar-btn ${props.paintFormatActive ? 'active' : ''}`}
        title="Paint format"
        aria-label="Paint format"
        aria-pressed={props.paintFormatActive}
        onClick={props.onPaintFormat}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="3" y="2" width="8" height="5" rx="0.5" />
          <path d="M11 4.5h1.5v3h-5v1.5" />
          <line x1="7.5" y1="9" x2="7.5" y2="14" />
          <line x1="6" y1="14" x2="9" y2="14" />
        </svg>
      </button>

      <div class="toolbar-separator" />

      {/* Font Color */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn"
          title="Text color"
          aria-label="Text color"
          onClick={() => { const wasOpen = showFontColorPicker(); closeAllDropdowns(); if (!wasOpen) setShowFontColorPicker(true); }}
        >
          <span style={{ "border-bottom": `3px solid ${lastFontColor()}`, "line-height": "1" }}>A</span>
        </button>
        <Show when={showFontColorPicker()}>
          <div class="toolbar-color-picker">
            <For each={PRESET_COLORS}>
              {(color) => (
                <div
                  class="toolbar-color-swatch"
                  style={{ background: color }}
                  onClick={() => handleFontColor(color)}
                  title={color}
                />
              )}
            </For>
            <div class="toolbar-hex-row">
              <div
                class="toolbar-hex-preview"
                style={{ background: isValidHexColor(normaliseHex(fontHexInput())) ? normaliseHex(fontHexInput()) : lastFontColor() }}
              />
              <input
                class="toolbar-hex-input"
                type="text"
                placeholder="#000000"
                maxLength={9}
                value={fontHexInput()}
                onInput={(e) => setFontHexInput(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    const hex = normaliseHex(fontHexInput());
                    if (isValidHexColor(hex)) {
                      handleFontColor(hex);
                      setFontHexInput('');
                    }
                  }
                  e.stopPropagation();
                }}
              />
            </div>
          </div>
        </Show>
      </div>

      {/* Background Color */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn"
          title="Fill color"
          aria-label="Fill color"
          onClick={() => { const wasOpen = showBgColorPicker(); closeAllDropdowns(); if (!wasOpen) setShowBgColorPicker(true); }}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <rect x="2" y="2" width="12" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5" />
            <rect x="3" y="10" width="10" height="3" fill={lastBgColor()} />
          </svg>
        </button>
        <Show when={showBgColorPicker()}>
          <div class="toolbar-color-picker">
            <div
              class="toolbar-color-swatch toolbar-color-none"
              onClick={() => handleBgColor('')}
              title="No fill"
            >
              <span style={{ "font-size": "10px" }}>X</span>
            </div>
            <For each={PRESET_COLORS}>
              {(color) => (
                <div
                  class="toolbar-color-swatch"
                  style={{ background: color }}
                  onClick={() => handleBgColor(color)}
                  title={color}
                />
              )}
            </For>
            <div class="toolbar-hex-row">
              <div
                class="toolbar-hex-preview"
                style={{ background: isValidHexColor(normaliseHex(bgHexInput())) ? normaliseHex(bgHexInput()) : lastBgColor() }}
              />
              <input
                class="toolbar-hex-input"
                type="text"
                placeholder="#000000"
                maxLength={9}
                value={bgHexInput()}
                onInput={(e) => setBgHexInput(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    const hex = normaliseHex(bgHexInput());
                    if (isValidHexColor(hex)) {
                      handleBgColor(hex);
                      setBgHexInput('');
                    }
                  }
                  e.stopPropagation();
                }}
              />
            </div>
          </div>
        </Show>
      </div>

      {/* Borders */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn"
          title="Borders"
          aria-label="Borders"
          onClick={() => { const wasOpen = showBordersDropdown(); closeAllDropdowns(); if (!wasOpen) setShowBordersDropdown(true); }}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <rect x="2" y="2" width="12" height="12" rx="0.5" />
            <line x1="8" y1="2" x2="8" y2="14" />
            <line x1="2" y1="8" x2="14" y2="8" />
          </svg>
        </button>
        <Show when={showBordersDropdown()}>
          <div class="toolbar-dropdown-menu" style={{ "min-width": "140px" }}>
            <For each={BORDER_PRESETS}>
              {(preset) => (
                <div
                  class="toolbar-dropdown-item"
                  onClick={() => handleBorderPreset(preset.borders)}
                >
                  {preset.label}
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>

      <div class="toolbar-separator" />

      {/* Alignment */}
      <button class="toolbar-btn" title="Align left" aria-label="Align left" onClick={() => props.onAlign('left')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="2" y1="4" x2="14" y2="4" />
          <line x1="2" y1="8" x2="10" y2="8" />
          <line x1="2" y1="12" x2="14" y2="12" />
        </svg>
      </button>
      <button class="toolbar-btn" title="Align center" aria-label="Align center" onClick={() => props.onAlign('center')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="2" y1="4" x2="14" y2="4" />
          <line x1="4" y1="8" x2="12" y2="8" />
          <line x1="2" y1="12" x2="14" y2="12" />
        </svg>
      </button>
      <button class="toolbar-btn" title="Align right" aria-label="Align right" onClick={() => props.onAlign('right')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="2" y1="4" x2="14" y2="4" />
          <line x1="6" y1="8" x2="14" y2="8" />
          <line x1="2" y1="12" x2="14" y2="12" />
        </svg>
      </button>

      <div class="toolbar-separator" />

      {/* Vertical Alignment */}
      <button class="toolbar-btn" title="Align top" aria-label="Align top" onClick={() => props.onVAlign('top')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="2" y1="3" x2="14" y2="3" />
          <line x1="5" y1="7" x2="11" y2="7" />
          <line x1="6" y1="10" x2="10" y2="10" />
        </svg>
      </button>
      <button class="toolbar-btn" title="Align middle" aria-label="Align middle" onClick={() => props.onVAlign('middle')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="6" y1="4" x2="10" y2="4" />
          <line x1="4" y1="8" x2="12" y2="8" />
          <line x1="6" y1="12" x2="10" y2="12" />
        </svg>
      </button>
      <button class="toolbar-btn" title="Align bottom" aria-label="Align bottom" onClick={() => props.onVAlign('bottom')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="6" y1="6" x2="10" y2="6" />
          <line x1="5" y1="9" x2="11" y2="9" />
          <line x1="2" y1="13" x2="14" y2="13" />
        </svg>
      </button>

      {/* Indent / Outdent */}
      <button class="toolbar-btn" title="Decrease indent" aria-label="Decrease indent" onClick={props.onOutdent}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="2" y1="3" x2="14" y2="3" />
          <line x1="6" y1="7" x2="14" y2="7" />
          <line x1="6" y1="11" x2="14" y2="11" />
          <path d="M4 9L2 7l2-2" />
        </svg>
      </button>
      <button class="toolbar-btn" title="Increase indent" aria-label="Increase indent" onClick={props.onIndent}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="2" y1="3" x2="14" y2="3" />
          <line x1="6" y1="7" x2="14" y2="7" />
          <line x1="6" y1="11" x2="14" y2="11" />
          <path d="M2 5l2 2-2 2" />
        </svg>
      </button>

      {/* Text Wrap */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn"
          title="Text wrapping"
          aria-label="Text wrapping"
          onClick={() => { const wasOpen = showTextWrapDropdown(); closeAllDropdowns(); if (!wasOpen) setShowTextWrapDropdown(true); }}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <line x1="2" y1="4" x2="14" y2="4" />
            <path d="M2 8h9a2 2 0 0 1 0 4H9" />
            <path d="M10 10.5L8.5 12l1.5 1.5" />
          </svg>
        </button>
        <Show when={showTextWrapDropdown()}>
          <div class="toolbar-dropdown-menu" style={{ "min-width": "100px" }}>
            <div class="toolbar-dropdown-item" onClick={() => { setShowTextWrapDropdown(false); props.onTextWrap('Overflow'); }}>
              Overflow
            </div>
            <div class="toolbar-dropdown-item" onClick={() => { setShowTextWrapDropdown(false); props.onTextWrap('Wrap'); }}>
              Wrap
            </div>
            <div class="toolbar-dropdown-item" onClick={() => { setShowTextWrapDropdown(false); props.onTextWrap('Clip'); }}>
              Clip
            </div>
          </div>
        </Show>
      </div>

      {/* Merge Cells */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn"
          title="Merge cells"
          aria-label="Merge cells"
          onClick={() => { const wasOpen = showMergeDropdown(); closeAllDropdowns(); if (!wasOpen) setShowMergeDropdown(true); }}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <rect x="2" y="3" width="12" height="10" rx="1" />
            <line x1="8" y1="3" x2="8" y2="13" stroke-dasharray="2 2" />
          </svg>
        </button>
        <Show when={showMergeDropdown()}>
          <div class="toolbar-dropdown-menu" style={{ "min-width": "120px" }}>
            <div class="toolbar-dropdown-item" onClick={() => { setShowMergeDropdown(false); props.onMerge(); }}>
              Merge all
            </div>
            <div class="toolbar-dropdown-item" onClick={() => { setShowMergeDropdown(false); props.onUnmerge(); }}>
              Unmerge
            </div>
          </div>
        </Show>
      </div>

      {/* Text Rotation */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn"
          title="Text rotation"
          aria-label="Text rotation"
          onClick={() => { const wasOpen = showTextRotationDropdown(); closeAllDropdowns(); if (!wasOpen) setShowTextRotationDropdown(true); }}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <text x="3" y="12" font-size="11" fill="currentColor" stroke="none" transform="rotate(-30 8 8)">A</text>
            <path d="M12 4a6 6 0 0 1-1.5 5.5" />
            <path d="M11.5 8l-1 1.5 1.5.5" />
          </svg>
        </button>
        <Show when={showTextRotationDropdown()}>
          <div class="toolbar-dropdown-menu" style={{ "min-width": "150px" }}>
            <div class="toolbar-dropdown-item" onClick={() => { setShowTextRotationDropdown(false); props.onTextRotation(0); }}>
              0° (Normal)
            </div>
            <div class="toolbar-dropdown-item" onClick={() => { setShowTextRotationDropdown(false); props.onTextRotation(45); }}>
              45° (Diagonal up)
            </div>
            <div class="toolbar-dropdown-item" onClick={() => { setShowTextRotationDropdown(false); props.onTextRotation(-45); }}>
              -45° (Diagonal down)
            </div>
            <div class="toolbar-dropdown-item" onClick={() => { setShowTextRotationDropdown(false); props.onTextRotation(90); }}>
              90° (Vertical up)
            </div>
            <div class="toolbar-dropdown-item" onClick={() => { setShowTextRotationDropdown(false); props.onTextRotation(-90); }}>
              -90° (Vertical down)
            </div>
            <div class="toolbar-dropdown-item" style={{ "border-top": "1px solid var(--grid-border)", "padding-top": "6px", "margin-top": "2px" }}>
              <input
                class="toolbar-hex-input"
                type="number"
                min="-360"
                max="360"
                placeholder="Custom..."
                value={customRotation()}
                onInput={(e) => setCustomRotation(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    const val = parseInt(customRotation(), 10);
                    if (!isNaN(val)) {
                      setShowTextRotationDropdown(false);
                      props.onTextRotation(val);
                      setCustomRotation('');
                    }
                  }
                  e.stopPropagation();
                }}
                onClick={(e) => e.stopPropagation()}
                style={{ width: '80px' }}
              />
            </div>
          </div>
        </Show>
      </div>

      <div class="toolbar-separator" />

      {/* Number Format Buttons */}
      <button
        class="toolbar-btn"
        title="Format as currency"
        aria-label="Format as currency"
        onClick={(e) => handleNumberFormat(e.currentTarget, '$#,##0.00')}
      >
        $
      </button>
      <button
        class="toolbar-btn"
        title="Format as percent"
        aria-label="Format as percent"
        onClick={(e) => handleNumberFormat(e.currentTarget, '0%')}
      >
        %
      </button>
      <button
        class="toolbar-btn toolbar-btn-wide"
        title="Increase decimal places"
        aria-label="Increase decimal places"
        onClick={(e) => handleNumberFormat(e.currentTarget, '.0+')}
        style={{ "font-size": "11px" }}
      >
        .0→
      </button>
      <button
        class="toolbar-btn toolbar-btn-wide"
        title="Decrease decimal places"
        aria-label="Decrease decimal places"
        onClick={(e) => handleNumberFormat(e.currentTarget, '.0-')}
        style={{ "font-size": "11px" }}
      >
        ←.0
      </button>

      <div class="toolbar-separator" />

      {/* Freeze Panes */}
      <button
        class={`toolbar-btn toolbar-btn-wide ${props.freezeActive ? 'active' : ''}`}
        title="Freeze panes at selection"
        aria-label="Freeze panes"
        aria-pressed={props.freezeActive}
        onClick={props.onFreezeToggle}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="2" y="2" width="12" height="12" rx="1" />
          <line x1="6" y1="2" x2="6" y2="14" stroke-dasharray="2 1" />
          <line x1="2" y1="6" x2="14" y2="6" stroke-dasharray="2 1" />
        </svg>
      </button>

      {/* Split Panes */}
      <button
        class={`toolbar-btn toolbar-btn-wide ${props.splitActive ? 'active' : ''}`}
        title="Split panes at selection"
        aria-label="Split panes"
        aria-pressed={props.splitActive}
        onClick={props.onSplitToggle}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="2" y="2" width="12" height="12" rx="1" />
          <line x1="8" y1="2" x2="8" y2="14" />
          <line x1="2" y1="8" x2="14" y2="8" />
        </svg>
      </button>

      <div class="toolbar-separator" />

      {/* Insert Chart */}
      <button
        class="toolbar-btn toolbar-btn-wide"
        title="Insert chart"
        aria-label="Insert chart"
        onClick={props.onInsertChart}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="2" y="8" width="3" height="6" rx="0.5" fill="currentColor" opacity="0.3" />
          <rect x="6.5" y="4" width="3" height="10" rx="0.5" fill="currentColor" opacity="0.5" />
          <rect x="11" y="6" width="3" height="8" rx="0.5" fill="currentColor" opacity="0.7" />
        </svg>
      </button>

      {/* Filter toggle */}
      <button
        class={`toolbar-btn toolbar-btn-wide ${props.filterActive ? 'active' : ''}`}
        title={props.filterActive ? 'Remove filter' : 'Create filter'}
        aria-label={props.filterActive ? 'Remove filter' : 'Create filter'}
        aria-pressed={props.filterActive}
        onClick={props.onFilterToggle}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M2 3h12L9 8.5V12l-2 1.5V8.5L2 3z" />
          {props.filterActive && (
            <circle cx="12" cy="12" r="3" fill="var(--selection-border)" stroke="none" />
          )}
        </svg>
      </button>

      {/* Conditional Formatting */}
      <button
        class="toolbar-btn toolbar-btn-wide"
        title="Conditional formatting"
        aria-label="Conditional formatting"
        onClick={props.onConditionalFormat}
        style={{ "font-size": "11px" }}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="2" y="2" width="12" height="12" rx="1" />
          <circle cx="5.5" cy="8" r="1.5" fill="#cc0000" stroke="none" />
          <circle cx="8" cy="8" r="1.5" fill="#f1c232" stroke="none" />
          <circle cx="10.5" cy="8" r="1.5" fill="#6aa84f" stroke="none" />
        </svg>
      </button>

      <div class="toolbar-separator" />

      {/* Insert Function (Sigma) */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn"
          title="Insert function"
          aria-label="Insert function"
          onClick={() => props.onInsertFunction('SUM')}
        >
          <span style={{ "font-size": "16px", "font-weight": "bold" }}>&Sigma;</span>
        </button>
        <button
          class="toolbar-btn"
          title="More functions"
          style={{ width: '16px', "margin-left": '-4px' }}
          onClick={() => { const wasOpen = showFunctionDropdown(); closeAllDropdowns(); if (!wasOpen) setShowFunctionDropdown(true); }}
        >
          <svg width="8" height="8" viewBox="0 0 8 8" fill="currentColor">
            <path d="M1 2.5L4 5.5L7 2.5" />
          </svg>
        </button>
        <Show when={showFunctionDropdown()}>
          <div class="toolbar-dropdown-menu" style={{ "min-width": "100px" }}>
            <For each={COMMON_FUNCTIONS}>
              {(fn) => (
                <div
                  class="toolbar-dropdown-item"
                  onClick={() => { setShowFunctionDropdown(false); props.onInsertFunction(fn); }}
                >
                  {fn}
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>

      <div class="toolbar-separator" />

      <span class="toolbar-brand">
        <img src="/logo.svg" alt="Lattice" class="toolbar-brand-logo" />
        Lattice
      </span>
    </div>
  );
};

export default Toolbar;
