import type { Component } from 'solid-js';
import { createSignal, For, Show } from 'solid-js';

/** Paste mode matching the Rust PasteMode enum. */
export type PasteMode =
  | 'All'
  | 'ValuesOnly'
  | 'FormulasOnly'
  | 'FormattingOnly'
  | 'Transposed'
  | 'ColumnWidthsOnly';

/** Arithmetic operation to apply during paste. */
export type PasteOperation =
  | 'None'
  | 'Add'
  | 'Subtract'
  | 'Multiply'
  | 'Divide';

/** Radio option metadata. */
interface PasteModeOption {
  value: PasteMode;
  label: string;
  description: string;
}

/** Operation option metadata. */
interface PasteOperationOption {
  value: PasteOperation;
  label: string;
}

const PASTE_MODES: PasteModeOption[] = [
  { value: 'All', label: 'All', description: 'Values, formulas, and formatting' },
  { value: 'ValuesOnly', label: 'Values only', description: 'Paste values without formulas' },
  { value: 'FormulasOnly', label: 'Formulas only', description: 'Paste formulas without formatting' },
  { value: 'FormattingOnly', label: 'Formatting only', description: 'Apply formatting without changing values' },
  { value: 'Transposed', label: 'Transposed', description: 'Swap rows and columns' },
  { value: 'ColumnWidthsOnly', label: 'Column widths only', description: 'Copy column widths to target columns' },
];

const PASTE_OPERATIONS: PasteOperationOption[] = [
  { value: 'None', label: 'None' },
  { value: 'Add', label: 'Add' },
  { value: 'Subtract', label: 'Subtract' },
  { value: 'Multiply', label: 'Multiply' },
  { value: 'Divide', label: 'Divide' },
];

export interface PasteSpecialDialogProps {
  /** Called when the user confirms the paste with the selected mode and operation. */
  onPaste: (mode: PasteMode, operation: PasteOperation) => void;
  /** Called when the dialog is cancelled/closed. */
  onClose: () => void;
}

const PasteSpecialDialog: Component<PasteSpecialDialogProps> = (props) => {
  const [selectedMode, setSelectedMode] = createSignal<PasteMode>('All');
  const [selectedOperation, setSelectedOperation] = createSignal<PasteOperation>('None');

  const handlePaste = () => {
    props.onPaste(selectedMode(), selectedOperation());
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
      handlePaste();
    }
  };

  /** Operations only apply to value-based paste modes. */
  const showOperations = () => {
    const mode = selectedMode();
    return mode === 'All' || mode === 'ValuesOnly';
  };

  return (
    <div
      class="paste-special-backdrop"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
    >
      <div class="paste-special-dialog">
        <div class="paste-special-header">
          <h2>Paste Special</h2>
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
          <div class="paste-special-field">
            <label class="paste-special-label">Paste mode</label>
            <div class="paste-special-options">
              <For each={PASTE_MODES}>
                {(option) => (
                  <label
                    class={`paste-special-option ${selectedMode() === option.value ? 'selected' : ''}`}
                  >
                    <input
                      type="radio"
                      name="paste-mode"
                      value={option.value}
                      checked={selectedMode() === option.value}
                      onChange={() => {
                        setSelectedMode(option.value);
                        // Reset operation when switching to non-value mode
                        if (option.value !== 'All' && option.value !== 'ValuesOnly') {
                          setSelectedOperation('None');
                        }
                      }}
                    />
                    <div class="paste-special-option-content">
                      <span class="paste-special-option-label">{option.label}</span>
                      <span class="paste-special-option-desc">{option.description}</span>
                    </div>
                  </label>
                )}
              </For>
            </div>
          </div>

          <Show when={showOperations()}>
            <div class="paste-special-field" style={{ "margin-top": "12px" }}>
              <label class="paste-special-label">Operation</label>
              <div class="paste-special-operations">
                <For each={PASTE_OPERATIONS}>
                  {(option) => (
                    <label
                      class={`paste-special-operation ${selectedOperation() === option.value ? 'selected' : ''}`}
                    >
                      <input
                        type="radio"
                        name="paste-operation"
                        value={option.value}
                        checked={selectedOperation() === option.value}
                        onChange={() => setSelectedOperation(option.value)}
                      />
                      <span class="paste-special-operation-label">{option.label}</span>
                    </label>
                  )}
                </For>
              </div>
            </div>
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
            onClick={handlePaste}
          >
            Paste
          </button>
        </div>
      </div>
    </div>
  );
};

export default PasteSpecialDialog;
