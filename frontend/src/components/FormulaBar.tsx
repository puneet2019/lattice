import type { Component } from 'solid-js';
import { createEffect, createSignal, Show, For } from 'solid-js';
import { resolveNamedRange } from '../bridge/tauri';
import type { NamedRangeInfo } from '../bridge/tauri';

/** Same palette used for canvas reference highlights in VirtualGrid. */
const FORMULA_REF_COLORS = [
  '#1a73e8', // blue
  '#ea4335', // red
  '#34a853', // green
  '#9334e6', // purple
  '#e8710a', // orange
  '#00897b', // teal
];

/** Background colors (lighter versions) for reference spans in the formula bar. */
const FORMULA_REF_BG_COLORS = [
  'rgba(26, 115, 232, 0.12)',
  'rgba(234, 67, 53, 0.12)',
  'rgba(52, 168, 83, 0.12)',
  'rgba(147, 52, 230, 0.12)',
  'rgba(232, 113, 10, 0.12)',
  'rgba(0, 137, 123, 0.12)',
];

/** Regex pattern for cell references (A1, $B$2, A1:B10, etc.) */
const REF_PATTERN = /\$?[A-Za-z]{1,3}\$?\d+(?::\$?[A-Za-z]{1,3}\$?\d+)?/g;

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

/** Extract plain text from a contenteditable div, preserving newlines. */
function getPlainText(el: HTMLElement): string {
  // Use innerText which respects <br> as newlines
  return el.innerText ?? '';
}

/** Get the cursor offset as a character position within the plain text of the element. */
function getCursorOffset(el: HTMLElement): number {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0) return -1;

  const range = sel.getRangeAt(0);
  const preRange = document.createRange();
  preRange.selectNodeContents(el);
  preRange.setEnd(range.startContainer, range.startOffset);
  return preRange.toString().length;
}

/** Set the cursor to a specific character offset within the plain text of the element. */
function setCursorOffset(el: HTMLElement, offset: number): void {
  const sel = window.getSelection();
  if (!sel) return;

  // Walk text nodes to find the right position
  let remaining = offset;
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, null);
  let node = walker.nextNode();
  while (node) {
    const len = (node.textContent ?? '').length;
    if (remaining <= len) {
      const range = document.createRange();
      range.setStart(node, remaining);
      range.collapse(true);
      sel.removeAllRanges();
      sel.addRange(range);
      return;
    }
    remaining -= len;
    node = walker.nextNode();
  }

  // If offset is beyond the end, place cursor at the end
  const range = document.createRange();
  range.selectNodeContents(el);
  range.collapse(false);
  sel.removeAllRanges();
  sel.addRange(range);
}

/** Build colorized HTML for a formula string. Non-formula text is returned as plain escaped HTML. */
function colorizeFormula(text: string): string {
  if (!text.startsWith('=')) {
    return escapeHtml(text);
  }

  const parts: string[] = [];
  let lastIndex = 0;
  let colorIdx = 0;
  let match: RegExpExecArray | null;

  // Reset regex state
  REF_PATTERN.lastIndex = 0;
  while ((match = REF_PATTERN.exec(text)) !== null) {
    // Add text before this match
    if (match.index > lastIndex) {
      parts.push(escapeHtml(text.slice(lastIndex, match.index)));
    }
    // Wrap the reference in a colored span
    const color = FORMULA_REF_COLORS[colorIdx % FORMULA_REF_COLORS.length];
    const bg = FORMULA_REF_BG_COLORS[colorIdx % FORMULA_REF_BG_COLORS.length];
    parts.push(
      `<span style="color:${color};background:${bg};border-radius:2px;padding:0 1px;" data-ref="true">${escapeHtml(match[0])}</span>`,
    );
    colorIdx++;
    lastIndex = match.index + match[0].length;
  }

  // Add remaining text
  if (lastIndex < text.length) {
    parts.push(escapeHtml(text.slice(lastIndex)));
  }

  return parts.join('');
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
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
  let editorRef: HTMLDivElement | undefined;

  /** Whether we should suppress the next colorization (to avoid loops). */
  let suppressColorize = false;

  // Sync local value when the cell content prop changes (and we're not editing).
  createEffect(() => {
    if (!editing()) {
      setLocalValue(props.content);
      updateEditorContent(props.content);
    }
  });

  /** Update the contenteditable div with colorized or plain HTML. */
  function updateEditorContent(text: string) {
    if (!editorRef) return;
    suppressColorize = true;
    editorRef.innerHTML = colorizeFormula(text);
    suppressColorize = false;
  }

  /** Colorize the editor content while preserving cursor position. */
  function colorizeEditor() {
    if (!editorRef || suppressColorize) return;
    const text = getPlainText(editorRef);
    const isFormula = text.startsWith('=');

    // Only colorize if it's a formula and contains references
    if (!isFormula) return;

    const cursorPos = getCursorOffset(editorRef);
    suppressColorize = true;
    editorRef.innerHTML = colorizeFormula(text);
    suppressColorize = false;

    // Restore cursor position
    if (cursorPos >= 0 && document.activeElement === editorRef) {
      setCursorOffset(editorRef, cursorPos);
    }
  }

  // Auto-resize editor height based on content
  const autoResize = () => {
    if (!editorRef) return;
    if (expanded()) {
      editorRef.style.height = 'auto';
      const scrollH = editorRef.scrollHeight;
      const minH = 76;
      editorRef.style.height = `${Math.max(minH, scrollH)}px`;
    } else {
      editorRef.style.height = '20px';
    }
  };

  createEffect(() => {
    void expanded();
    void localValue();
    autoResize();
  });

  const handleFocus = () => {
    setEditing(true);
    setLocalValue(props.content);
    updateEditorContent(props.content);
    // Place cursor at the end after focus
    requestAnimationFrame(() => {
      if (editorRef) {
        setCursorOffset(editorRef, props.content.length);
      }
    });
  };

  const handleInput = () => {
    if (!editorRef) return;
    const text = getPlainText(editorRef);
    setLocalValue(text);
    props.onContentChange?.(text);
    autoResize();

    // Debounce colorization to avoid jank on every keystroke
    if (text.startsWith('=')) {
      // Use requestAnimationFrame for a tiny delay before colorizing
      requestAnimationFrame(() => {
        colorizeEditor();
      });
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      props.onCommit(localValue());
      setEditing(false);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      setLocalValue(props.content);
      updateEditorContent(props.content);
      setEditing(false);
      props.onCancel();
    }
    // Shift+Enter inserts a newline (default contenteditable behavior)
    // Arrow keys move cursor within text (default contenteditable behavior)
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
      <div class="formula-bar-cell-ref" onClick={handleNameBoxClick} style={{ position: 'relative' }} aria-label="Cell reference">
        {nameBoxEditing() ? (
          <>
            <input
              ref={nameBoxRef}
              class="formula-bar-name-input"
              type="text"
              aria-label="Cell reference"
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
      <div
        ref={editorRef}
        class="formula-bar-input"
        contentEditable={true}
        aria-label="Formula bar"
        onInput={handleInput}
        onFocus={handleFocus}
        onBlur={() => {
          if (editing()) {
            props.onCommit(localValue());
            setEditing(false);
          }
        }}
        onKeyDown={handleKeyDown}
        spellcheck={false}
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
