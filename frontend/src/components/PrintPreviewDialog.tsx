import type { Component } from 'solid-js';
import { createSignal, createEffect, onMount, Show } from 'solid-js';
import { exportPrintHtml } from '../bridge/tauri';
import type { PrintSettingsParams } from '../bridge/tauri';

export interface PrintPreviewDialogProps {
  activeSheet: string;
  sheets: string[];
  onClose: () => void;
  onStatusChange: (msg: string) => void;
}

type PaperSize = 'letter' | 'a4' | 'legal' | 'tabloid';
type Orientation = 'portrait' | 'landscape';
type MarginPreset = 'normal' | 'narrow' | 'wide' | 'custom';
type ScaleMode = 'none' | 'fit-width' | 'fit-height' | 'fit-page' | 'custom';
type PrintRange = 'current' | 'selection' | 'all';

const PAPER_SIZES: { value: PaperSize; label: string }[] = [
  { value: 'letter', label: 'Letter (8.5 x 11 in)' },
  { value: 'a4', label: 'A4 (210 x 297 mm)' },
  { value: 'legal', label: 'Legal (8.5 x 14 in)' },
  { value: 'tabloid', label: 'Tabloid (11 x 17 in)' },
];

const MARGIN_PRESETS: { value: MarginPreset; label: string }[] = [
  { value: 'normal', label: 'Normal' },
  { value: 'narrow', label: 'Narrow' },
  { value: 'wide', label: 'Wide' },
  { value: 'custom', label: 'Custom' },
];

const PrintPreviewDialog: Component<PrintPreviewDialogProps> = (props) => {
  let iframeRef: HTMLIFrameElement | undefined;

  // Settings state
  const [printRange, setPrintRange] = createSignal<PrintRange>('current');
  const [paperSize, setPaperSize] = createSignal<PaperSize>('letter');
  const [orientation, setOrientation] = createSignal<Orientation>('portrait');
  const [scaleMode, setScaleMode] = createSignal<ScaleMode>('none');
  const [customScale, setCustomScale] = createSignal(100);
  const [marginPreset, setMarginPreset] = createSignal<MarginPreset>('normal');
  const [customMargins, setCustomMargins] = createSignal<[number, number, number, number]>([1.5, 1.5, 1.5, 1.5]);
  const [showGridlines, setShowGridlines] = createSignal(true);
  const [showHeaders, setShowHeaders] = createSignal(false);
  const [repeatFrozenRows, setRepeatFrozenRows] = createSignal(false);
  const [previewHtml, setPreviewHtml] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  // Header/footer state
  const [headersFootersEnabled, setHeadersFootersEnabled] = createSignal(false);
  const [headerLeft, setHeaderLeft] = createSignal('');
  const [headerCenter, setHeaderCenter] = createSignal('');
  const [headerRight, setHeaderRight] = createSignal('');
  const [footerLeft, setFooterLeft] = createSignal('');
  const [footerCenter, setFooterCenter] = createSignal('&P');
  const [footerRight, setFooterRight] = createSignal('');

  /** Build the settings params from current state. */
  const buildSettings = (): PrintSettingsParams => {
    let scale = 1.0;
    if (scaleMode() === 'custom') {
      scale = customScale() / 100;
    } else if (scaleMode() === 'fit-width' || scaleMode() === 'fit-height' || scaleMode() === 'fit-page') {
      // These are handled by the browser at print time via CSS; pass 1.0 for now
      scale = 1.0;
    }

    const result: PrintSettingsParams = {
      paperSize: paperSize(),
      orientation: orientation(),
      showGridlines: showGridlines(),
      showHeaders: showHeaders(),
      scale,
      margins: marginPreset(),
    };

    if (marginPreset() === 'custom') {
      result.customMargins = customMargins();
    }

    return result;
  };

  /** Fetch the print preview HTML from the backend. */
  const refreshPreview = async () => {
    setLoading(true);
    try {
      const settings = buildSettings();
      const html = await exportPrintHtml(props.activeSheet, settings);
      setPreviewHtml(html);
    } catch (e) {
      props.onStatusChange(`Print preview failed: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  // Update preview when settings change.
  createEffect(() => {
    // Access all reactive settings to track them
    paperSize();
    orientation();
    scaleMode();
    customScale();
    marginPreset();
    customMargins();
    showGridlines();
    showHeaders();
    void refreshPreview();
  });

  // Write the HTML to the iframe when it updates.
  createEffect(() => {
    const html = previewHtml();
    if (iframeRef && html) {
      const doc = iframeRef.contentDocument;
      if (doc) {
        doc.open();
        doc.write(html);
        doc.close();
      }
    }
  });

  onMount(() => {
    void refreshPreview();
  });

  /** Print the preview content. */
  const handlePrint = () => {
    if (iframeRef?.contentWindow) {
      iframeRef.contentWindow.print();
    }
  };

  /** Handle keyboard shortcuts in the dialog. */
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      props.onClose();
    } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handlePrint();
    }
  };

  /** Update a single custom margin value. */
  const updateMargin = (index: number, value: number) => {
    const m = [...customMargins()] as [number, number, number, number];
    m[index] = value;
    setCustomMargins(m);
  };

  return (
    <div class="print-preview-overlay" onKeyDown={handleKeyDown} tabIndex={-1} ref={(el) => el.focus()}>
      <div class="print-preview-dialog">
        {/* Preview pane */}
        <div class="print-preview-left">
          <div class="print-preview-header">
            <h2>Print Preview</h2>
            <button class="print-preview-close" onClick={props.onClose} title="Close">&times;</button>
          </div>
          <div class="print-preview-iframe-wrapper">
            <Show when={loading()}>
              <div class="print-preview-loading">Loading preview...</div>
            </Show>
            <iframe
              ref={iframeRef}
              class="print-preview-iframe"
              title="Print Preview"
              sandbox="allow-same-origin"
            />
          </div>
        </div>

        {/* Settings panel */}
        <div class="print-preview-right">
          <h3>Print Settings</h3>

          {/* Print range */}
          <div class="print-settings-section">
            <label class="print-settings-label">Print range</label>
            <select
              class="print-settings-select"
              value={printRange()}
              onChange={(e) => setPrintRange(e.currentTarget.value as PrintRange)}
            >
              <option value="current">Current sheet</option>
              <option value="selection">Selection only</option>
              <option value="all">All sheets</option>
            </select>
          </div>

          {/* Paper size */}
          <div class="print-settings-section">
            <label class="print-settings-label">Paper size</label>
            <select
              class="print-settings-select"
              value={paperSize()}
              onChange={(e) => setPaperSize(e.currentTarget.value as PaperSize)}
            >
              {PAPER_SIZES.map((ps) => (
                <option value={ps.value}>{ps.label}</option>
              ))}
            </select>
          </div>

          {/* Orientation */}
          <div class="print-settings-section">
            <label class="print-settings-label">Orientation</label>
            <div class="print-settings-toggle-group">
              <button
                class={`print-settings-toggle ${orientation() === 'portrait' ? 'active' : ''}`}
                onClick={() => setOrientation('portrait')}
              >
                Portrait
              </button>
              <button
                class={`print-settings-toggle ${orientation() === 'landscape' ? 'active' : ''}`}
                onClick={() => setOrientation('landscape')}
              >
                Landscape
              </button>
            </div>
          </div>

          {/* Scale */}
          <div class="print-settings-section">
            <label class="print-settings-label">Scale</label>
            <select
              class="print-settings-select"
              value={scaleMode()}
              onChange={(e) => setScaleMode(e.currentTarget.value as ScaleMode)}
            >
              <option value="none">100% (Normal)</option>
              <option value="fit-width">Fit to width</option>
              <option value="fit-height">Fit to height</option>
              <option value="fit-page">Fit to page</option>
              <option value="custom">Custom %</option>
            </select>
            <Show when={scaleMode() === 'custom'}>
              <div class="print-settings-scale-input">
                <input
                  type="number"
                  class="print-settings-number"
                  value={customScale()}
                  min={10}
                  max={400}
                  step={5}
                  onInput={(e) => {
                    const v = parseInt(e.currentTarget.value, 10);
                    if (!isNaN(v) && v >= 10 && v <= 400) setCustomScale(v);
                  }}
                />
                <span>%</span>
              </div>
            </Show>
          </div>

          {/* Margins */}
          <div class="print-settings-section">
            <label class="print-settings-label">Margins</label>
            <select
              class="print-settings-select"
              value={marginPreset()}
              onChange={(e) => setMarginPreset(e.currentTarget.value as MarginPreset)}
            >
              {MARGIN_PRESETS.map((mp) => (
                <option value={mp.value}>{mp.label}</option>
              ))}
            </select>
            <Show when={marginPreset() === 'custom'}>
              <div class="print-settings-margins-grid">
                {(['Top', 'Bottom', 'Left', 'Right'] as const).map((side, i) => (
                  <div class="print-settings-margin-item">
                    <span class="print-settings-margin-label">{side}</span>
                    <input
                      type="number"
                      class="print-settings-number"
                      value={customMargins()[i]}
                      min={0}
                      max={10}
                      step={0.1}
                      onInput={(e) => {
                        const v = parseFloat(e.currentTarget.value);
                        if (!isNaN(v)) updateMargin(i, v);
                      }}
                    />
                    <span class="print-settings-margin-unit">cm</span>
                  </div>
                ))}
              </div>
            </Show>
          </div>

          {/* Headers & Footers */}
          <div class="print-settings-section">
            <label class="print-settings-checkbox-label">
              <input
                type="checkbox"
                checked={headersFootersEnabled()}
                onChange={(e) => setHeadersFootersEnabled(e.currentTarget.checked)}
              />
              Headers & Footers
            </label>
            <Show when={headersFootersEnabled()}>
              <div class="print-settings-hf-grid">
                <span class="print-settings-hf-label">Header</span>
                <input class="print-settings-hf-input" placeholder="Left" value={headerLeft()} onInput={(e) => setHeaderLeft(e.currentTarget.value)} />
                <input class="print-settings-hf-input" placeholder="Center" value={headerCenter()} onInput={(e) => setHeaderCenter(e.currentTarget.value)} />
                <input class="print-settings-hf-input" placeholder="Right" value={headerRight()} onInput={(e) => setHeaderRight(e.currentTarget.value)} />
                <span class="print-settings-hf-label">Footer</span>
                <input class="print-settings-hf-input" placeholder="Left" value={footerLeft()} onInput={(e) => setFooterLeft(e.currentTarget.value)} />
                <input class="print-settings-hf-input" placeholder="Center (&P=page)" value={footerCenter()} onInput={(e) => setFooterCenter(e.currentTarget.value)} />
                <input class="print-settings-hf-input" placeholder="Right" value={footerRight()} onInput={(e) => setFooterRight(e.currentTarget.value)} />
              </div>
              <div class="print-settings-hf-hint">&P = page number, &D = date</div>
            </Show>
          </div>

          {/* Print options */}
          <div class="print-settings-section">
            <label class="print-settings-label">Print options</label>
            <label class="print-settings-checkbox-label">
              <input
                type="checkbox"
                checked={showGridlines()}
                onChange={(e) => setShowGridlines(e.currentTarget.checked)}
              />
              Show gridlines
            </label>
            <label class="print-settings-checkbox-label">
              <input
                type="checkbox"
                checked={showHeaders()}
                onChange={(e) => setShowHeaders(e.currentTarget.checked)}
              />
              Show row/column headers
            </label>
            <label class="print-settings-checkbox-label">
              <input
                type="checkbox"
                checked={repeatFrozenRows()}
                onChange={(e) => setRepeatFrozenRows(e.currentTarget.checked)}
              />
              Repeat frozen rows on every page
            </label>
          </div>

          {/* Action buttons */}
          <div class="print-settings-actions">
            <button class="print-settings-btn print-settings-btn-primary" onClick={handlePrint}>
              Print
            </button>
            <button class="print-settings-btn" onClick={props.onClose}>
              Cancel
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default PrintPreviewDialog;
