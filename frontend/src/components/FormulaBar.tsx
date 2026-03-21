import type { Component } from 'solid-js';
import { createEffect, createSignal } from 'solid-js';

export interface FormulaBarProps {
  /** The A1-style reference of the active cell, e.g. "A1". */
  cellRef: string;
  /** The current content of the active cell (value or formula). */
  content: string;
  /** Called when the user edits the formula bar and presses Enter. */
  onCommit: (value: string) => void;
  /** Called when the user presses Escape. */
  onCancel: () => void;
}

const FormulaBar: Component<FormulaBarProps> = (props) => {
  const [editing, setEditing] = createSignal(false);
  const [localValue, setLocalValue] = createSignal('');

  // Sync local value when the cell content prop changes (and we're not editing).
  createEffect(() => {
    if (!editing()) {
      setLocalValue(props.content);
    }
  });

  const handleFocus = () => {
    setEditing(true);
    setLocalValue(props.content);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      props.onCommit(localValue());
      setEditing(false);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      setLocalValue(props.content);
      setEditing(false);
      props.onCancel();
    }
  };

  return (
    <div class="formula-bar">
      <div class="formula-bar-cell-ref">{props.cellRef}</div>
      <input
        class="formula-bar-input"
        type="text"
        value={localValue()}
        onInput={(e) => setLocalValue(e.currentTarget.value)}
        onFocus={handleFocus}
        onBlur={() => {
          if (editing()) {
            props.onCommit(localValue());
            setEditing(false);
          }
        }}
        onKeyDown={handleKeyDown}
      />
    </div>
  );
};

export default FormulaBar;
