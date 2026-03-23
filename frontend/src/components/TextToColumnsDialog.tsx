import type { Component } from 'solid-js';
import { createSignal, onMount, onCleanup } from 'solid-js';

export interface TextToColumnsDialogProps {
  onApply: (delimiter: string) => void;
  onClose: () => void;
}

const DELIMITERS = [
  { value: ',', label: 'Comma (,)' },
  { value: '\t', label: 'Tab' },
  { value: ';', label: 'Semicolon (;)' },
  { value: ' ', label: 'Space' },
  { value: '|', label: 'Pipe (|)' },
];

const TextToColumnsDialog: Component<TextToColumnsDialogProps> = (props) => {
  const [delimiter, setDelimiter] = createSignal(',');
  const [customDelimiter, setCustomDelimiter] = createSignal('');
  const [useCustom, setUseCustom] = createSignal(false);

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      props.onClose();
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      handleApply();
    }
  };

  const handleBackdropClick = (e: MouseEvent) => {
    if ((e.target as HTMLElement).classList.contains('paste-special-backdrop')) {
      props.onClose();
    }
  };

  const handleApply = () => {
    const d = useCustom() ? customDelimiter() : delimiter();
    if (!d) return;
    props.onApply(d);
  };

  onMount(() => {
    document.addEventListener('keydown', handleKeyDown);
  });

  onCleanup(() => {
    document.removeEventListener('keydown', handleKeyDown);
  });

  return (
    <div class="paste-special-backdrop" onClick={handleBackdropClick}>
      <div class="paste-special-dialog" style={{ "min-width": '340px' }}>
        <div class="paste-special-header">
          <h2>Text to Columns</h2>
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
          <p style={{ "font-size": '12px', color: 'var(--header-text)', margin: '0 0 12px' }}>
            Split the selected column into multiple columns by delimiter.
          </p>

          <div style={{ display: 'flex', "flex-direction": 'column', gap: '6px' }}>
            {DELIMITERS.map((d) => (
              <label style={{ display: 'flex', "align-items": 'center', gap: '8px', "font-size": '12px', cursor: 'pointer' }}>
                <input
                  type="radio"
                  name="delimiter"
                  checked={!useCustom() && delimiter() === d.value}
                  onChange={() => { setUseCustom(false); setDelimiter(d.value); }}
                />
                {d.label}
              </label>
            ))}
            <label style={{ display: 'flex', "align-items": 'center', gap: '8px', "font-size": '12px', cursor: 'pointer' }}>
              <input
                type="radio"
                name="delimiter"
                checked={useCustom()}
                onChange={() => setUseCustom(true)}
              />
              Custom:
              <input
                type="text"
                class="chart-dialog-input"
                style={{ width: '80px', "font-size": '12px' }}
                value={customDelimiter()}
                onInput={(e) => { setCustomDelimiter(e.currentTarget.value); setUseCustom(true); }}
                placeholder="e.g. ::"
              />
            </label>
          </div>
        </div>

        <div class="paste-special-footer">
          <button class="chart-dialog-btn" onClick={() => props.onClose()}>
            Cancel
          </button>
          <button
            class="chart-dialog-btn chart-dialog-btn-primary"
            onClick={handleApply}
            disabled={useCustom() && !customDelimiter()}
          >
            Split
          </button>
        </div>
      </div>
    </div>
  );
};

export default TextToColumnsDialog;
