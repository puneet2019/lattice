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
  /** Called when the user navigates to a cell via the name box (types ref + Enter). */
  onNavigate: (cellRef: string) => void;
  /** Called when formula bar content changes (syncs with cell editor). */
  onContentChange?: (value: string) => void;
}

const FormulaBar: Component<FormulaBarProps> = (props) => {
  const [editing, setEditing] = createSignal(false);
  const [localValue, setLocalValue] = createSignal('');
  const [nameBoxEditing, setNameBoxEditing] = createSignal(false);
  const [nameBoxValue, setNameBoxValue] = createSignal('');
  const [expanded, setExpanded] = createSignal(false);

  let nameBoxRef: HTMLInputElement | undefined;
  let textareaRef: HTMLTextAreaElement | undefined;

  // Sync local value when the cell content prop changes (and we're not editing).
  createEffect(() => {
    if (!editing()) {
      setLocalValue(props.content);
    }
  });

  // Auto-resize textarea height based on content
  const autoResize = () => {
    if (!textareaRef) return;
    if (expanded()) {
      // In expanded mode, show at least 4 lines
      textareaRef.style.height = 'auto';
      const scrollH = textareaRef.scrollHeight;
      const minH = 76; // ~4 lines at 19px each
      textareaRef.style.height = `${Math.max(minH, scrollH)}px`;
    } else {
      // In collapsed mode, single line (match the 28px min-height)
      textareaRef.style.height = '20px';
    }
  };

  createEffect(() => {
    // Re-run auto-resize when expanded state or value changes
    void expanded();
    void localValue();
    autoResize();
  });

  const handleFocus = () => {
    setEditing(true);
    setLocalValue(props.content);
  };

  const handleInput = (value: string) => {
    setLocalValue(value);
    props.onContentChange?.(value);
    autoResize();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      props.onCommit(localValue());
      setEditing(false);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      setLocalValue(props.content);
      setEditing(false);
      props.onCancel();
    }
    // Shift+Enter inserts a newline (default textarea behavior)
  };

  // Name box: click to edit, type a cell ref, press Enter to navigate.
  const handleNameBoxClick = () => {
    setNameBoxEditing(true);
    setNameBoxValue(props.cellRef);
    requestAnimationFrame(() => {
      if (nameBoxRef) {
        nameBoxRef.focus();
        nameBoxRef.select();
      }
    });
  };

  const handleNameBoxKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      const ref = nameBoxValue().trim().toUpperCase();
      if (ref) {
        props.onNavigate(ref);
      }
      setNameBoxEditing(false);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      setNameBoxEditing(false);
    }
  };

  return (
    <div class={`formula-bar ${expanded() ? 'formula-bar-expanded' : ''}`}>
      <div class="formula-bar-cell-ref" onClick={handleNameBoxClick}>
        {nameBoxEditing() ? (
          <input
            ref={nameBoxRef}
            class="formula-bar-name-input"
            type="text"
            value={nameBoxValue()}
            onInput={(e) => setNameBoxValue(e.currentTarget.value)}
            onKeyDown={handleNameBoxKeyDown}
            onBlur={() => setNameBoxEditing(false)}
          />
        ) : (
          <span>{props.cellRef}</span>
        )}
      </div>
      <div class="formula-bar-fx">
        <span class="formula-bar-fx-icon">fx</span>
      </div>
      <textarea
        ref={textareaRef}
        class="formula-bar-input"
        rows={1}
        value={localValue()}
        onInput={(e) => handleInput(e.currentTarget.value)}
        onFocus={handleFocus}
        onBlur={() => {
          if (editing()) {
            props.onCommit(localValue());
            setEditing(false);
          }
        }}
        onKeyDown={handleKeyDown}
      />
      <button
        class="formula-bar-expand-btn"
        title={expanded() ? 'Collapse formula bar' : 'Expand formula bar'}
        onClick={() => setExpanded(!expanded())}
      >
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5">
          {expanded() ? (
            <path d="M2 7L5 4l3 3" />
          ) : (
            <path d="M2 3l3 3 3-3" />
          )}
        </svg>
      </button>
    </div>
  );
};

export default FormulaBar;
