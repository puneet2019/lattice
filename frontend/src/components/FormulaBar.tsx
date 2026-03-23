import type { Component } from 'solid-js';
import { createEffect, createSignal, Show, For } from 'solid-js';
import { resolveNamedRange } from '../bridge/tauri';
import type { NamedRangeInfo } from '../bridge/tauri';

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
  /** Named ranges for name box suggestions. */
  namedRanges?: NamedRangeInfo[];
}

const FormulaBar: Component<FormulaBarProps> = (props) => {
  const [editing, setEditing] = createSignal(false);
  const [localValue, setLocalValue] = createSignal('');
  const [nameBoxEditing, setNameBoxEditing] = createSignal(false);
  const [nameBoxValue, setNameBoxValue] = createSignal('');
  const [expanded, setExpanded] = createSignal(false);
  const [nameBoxSuggestions, setNameBoxSuggestions] = createSignal<NamedRangeInfo[]>([]);
  const [showNameSuggestions, setShowNameSuggestions] = createSignal(false);

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

  // Name box: click to edit, type a cell ref or named range name, press Enter to navigate.
  const handleNameBoxClick = () => {
    setNameBoxEditing(true);
    setNameBoxValue(props.cellRef);
    setShowNameSuggestions(false);
    requestAnimationFrame(() => {
      if (nameBoxRef) {
        nameBoxRef.focus();
        nameBoxRef.select();
      }
    });
  };

  const handleNameBoxInput = (value: string) => {
    setNameBoxValue(value);
    // Filter named ranges for suggestions
    const ranges = props.namedRanges ?? [];
    if (value.trim().length > 0 && ranges.length > 0) {
      const lower = value.trim().toLowerCase();
      const matches = ranges.filter((nr) =>
        nr.name.toLowerCase().startsWith(lower),
      );
      setNameBoxSuggestions(matches);
      setShowNameSuggestions(matches.length > 0);
    } else {
      setShowNameSuggestions(false);
    }
  };

  const handleNameBoxKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      const ref = nameBoxValue().trim();
      if (ref) {
        // Try to resolve as a named range first
        void resolveNamedRange(ref)
          .then((nr) => {
            // Navigate to the resolved range
            props.onNavigate(nr.range);
          })
          .catch(() => {
            // Not a named range — treat as a cell ref
            props.onNavigate(ref.toUpperCase());
          });
      }
      setNameBoxEditing(false);
      setShowNameSuggestions(false);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      setNameBoxEditing(false);
      setShowNameSuggestions(false);
    }
  };

  const acceptNameSuggestion = (nr: NamedRangeInfo) => {
    setNameBoxValue(nr.name);
    setShowNameSuggestions(false);
    // Navigate immediately
    props.onNavigate(nr.range);
    setNameBoxEditing(false);
  };

  return (
    <div class={`formula-bar ${expanded() ? 'formula-bar-expanded' : ''}`}>
      <div class="formula-bar-cell-ref" onClick={handleNameBoxClick} style={{ position: 'relative' }}>
        {nameBoxEditing() ? (
          <>
            <input
              ref={nameBoxRef}
              class="formula-bar-name-input"
              type="text"
              value={nameBoxValue()}
              onInput={(e) => handleNameBoxInput(e.currentTarget.value)}
              onKeyDown={handleNameBoxKeyDown}
              onBlur={() => {
                // Delay to allow suggestion click to fire
                setTimeout(() => {
                  setNameBoxEditing(false);
                  setShowNameSuggestions(false);
                }, 150);
              }}
            />
            <Show when={showNameSuggestions()}>
              <div
                style={{
                  position: 'absolute',
                  top: '100%',
                  left: '0',
                  width: '100%',
                  "max-height": '150px',
                  "overflow-y": 'auto',
                  background: 'var(--cell-bg, #fff)',
                  border: '1px solid var(--grid-border, #e0e0e0)',
                  "border-radius": '0 0 4px 4px',
                  "box-shadow": '0 2px 8px rgba(0,0,0,0.15)',
                  "z-index": '100',
                }}
              >
                <For each={nameBoxSuggestions()}>
                  {(nr) => (
                    <div
                      style={{
                        padding: '4px 8px',
                        cursor: 'pointer',
                        "font-size": '12px',
                        "border-bottom": '1px solid var(--grid-border, #e0e0e0)',
                      }}
                      onMouseDown={(e) => {
                        e.preventDefault();
                        acceptNameSuggestion(nr);
                      }}
                    >
                      <div style={{ "font-weight": '600', color: 'var(--cell-text, #202124)' }}>
                        {nr.name}
                      </div>
                      <div style={{ color: 'var(--header-text, #5f6368)', "font-size": '11px' }}>
                        {nr.sheet ? `${nr.sheet}!${nr.range}` : nr.range}
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </>
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
