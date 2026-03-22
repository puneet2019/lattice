import type { Component } from 'solid-js';
import { createSignal, For, Show, onMount, onCleanup } from 'solid-js';

export interface ToolbarProps {
  onBold: () => void;
  onItalic: () => void;
  onUnderline: () => void;
  onFontSize: (size: number) => void;
  onFontFamily: (family: string) => void;
  onFontColor: (color: string) => void;
  onBgColor: (color: string) => void;
  onAlign: (align: 'left' | 'center' | 'right') => void;
  onNumberFormat: (format: string) => void;
  onUndo: () => void;
  onRedo: () => void;
  onFreezeToggle: () => void;
  onSplitToggle: () => void;
  onInsertChart: () => void;
  boldActive: boolean;
  italicActive: boolean;
  underlineActive: boolean;
  freezeActive: boolean;
  splitActive: boolean;
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
  const [currentFontSize, setCurrentFontSize] = createSignal(11);
  const [lastFontColor, setLastFontColor] = createSignal('#000000');
  const [lastBgColor, setLastBgColor] = createSignal('#ffff00');

  let toolbarRef: HTMLDivElement | undefined;

  const currentFamily = () => props.currentFontFamily ?? 'Arial';

  const anyDropdownOpen = () =>
    showFontFamilyDropdown() || showFontSizeDropdown() ||
    showFontColorPicker() || showBgColorPicker();

  const closeAllDropdowns = () => {
    setShowFontFamilyDropdown(false);
    setShowFontSizeDropdown(false);
    setShowFontColorPicker(false);
    setShowBgColorPicker(false);
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

  return (
    <div class="toolbar" ref={toolbarRef}>
      {/* Undo / Redo */}
      <button class="toolbar-btn" title="Undo (Cmd+Z)" onClick={props.onUndo}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M3 8h8a3 3 0 0 1 0 6H9" />
          <path d="M5 5L3 8l2 3" />
        </svg>
      </button>
      <button class="toolbar-btn" title="Redo (Cmd+Shift+Z)" onClick={props.onRedo}>
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
          onClick={() => { const wasOpen = showFontSizeDropdown(); closeAllDropdowns(); if (!wasOpen) setShowFontSizeDropdown(true); }}
        >
          {currentFontSize()}
        </button>
        <Show when={showFontSizeDropdown()}>
          <div class="toolbar-dropdown-menu">
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
        onClick={props.onBold}
      >
        <strong>B</strong>
      </button>
      <button
        class={`toolbar-btn ${props.italicActive ? 'active' : ''}`}
        title="Italic (Cmd+I)"
        onClick={props.onItalic}
      >
        <em>I</em>
      </button>
      <button
        class={`toolbar-btn ${props.underlineActive ? 'active' : ''}`}
        title="Underline (Cmd+U)"
        onClick={props.onUnderline}
      >
        <span style={{ "text-decoration": "underline" }}>U</span>
      </button>

      <div class="toolbar-separator" />

      {/* Font Color */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn"
          title="Text color"
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
          </div>
        </Show>
      </div>

      {/* Background Color */}
      <div class="toolbar-dropdown" style={{ position: 'relative' }}>
        <button
          class="toolbar-btn"
          title="Fill color"
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
          </div>
        </Show>
      </div>

      <div class="toolbar-separator" />

      {/* Alignment */}
      <button class="toolbar-btn" title="Align left" onClick={() => props.onAlign('left')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="2" y1="4" x2="14" y2="4" />
          <line x1="2" y1="8" x2="10" y2="8" />
          <line x1="2" y1="12" x2="14" y2="12" />
        </svg>
      </button>
      <button class="toolbar-btn" title="Align center" onClick={() => props.onAlign('center')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="2" y1="4" x2="14" y2="4" />
          <line x1="4" y1="8" x2="12" y2="8" />
          <line x1="2" y1="12" x2="14" y2="12" />
        </svg>
      </button>
      <button class="toolbar-btn" title="Align right" onClick={() => props.onAlign('right')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <line x1="2" y1="4" x2="14" y2="4" />
          <line x1="6" y1="8" x2="14" y2="8" />
          <line x1="2" y1="12" x2="14" y2="12" />
        </svg>
      </button>

      <div class="toolbar-separator" />

      {/* Number Format Buttons */}
      <button
        class="toolbar-btn"
        title="Format as currency"
        onClick={(e) => handleNumberFormat(e.currentTarget, '$#,##0.00')}
      >
        $
      </button>
      <button
        class="toolbar-btn"
        title="Format as percent"
        onClick={(e) => handleNumberFormat(e.currentTarget, '0%')}
      >
        %
      </button>
      <button
        class="toolbar-btn toolbar-btn-wide"
        title="Increase decimal places"
        onClick={(e) => handleNumberFormat(e.currentTarget, '.0+')}
        style={{ "font-size": "11px" }}
      >
        .0→
      </button>
      <button
        class="toolbar-btn toolbar-btn-wide"
        title="Decrease decimal places"
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
        onClick={props.onInsertChart}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="2" y="8" width="3" height="6" rx="0.5" fill="currentColor" opacity="0.3" />
          <rect x="6.5" y="4" width="3" height="10" rx="0.5" fill="currentColor" opacity="0.5" />
          <rect x="11" y="6" width="3" height="8" rx="0.5" fill="currentColor" opacity="0.7" />
        </svg>
      </button>

      <div class="toolbar-separator" />

      <span class="toolbar-brand">Lattice</span>
    </div>
  );
};

export default Toolbar;
