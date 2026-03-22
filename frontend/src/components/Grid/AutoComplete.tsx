import type { Component } from 'solid-js';
import { createSignal, createEffect, For, Show } from 'solid-js';

export interface AutoCompleteProps {
  /** Current text typed by the user. */
  inputValue: string;
  /** Pool of unique values to suggest from. */
  suggestions: string[];
  /** CSS position for the dropdown. */
  left: number;
  top: number;
  width: number;
  /** Whether the dropdown should be visible. */
  visible: boolean;
  /** Called when the user accepts a suggestion. */
  onAccept: (value: string) => void;
  /** Called when the user dismisses the dropdown. */
  onDismiss: () => void;
}

/**
 * A simple DOM dropdown that shows column value suggestions during cell editing.
 * Positioned absolutely below the cell editor.
 */
const AutoComplete: Component<AutoCompleteProps> = (props) => {
  const [selectedIndex, setSelectedIndex] = createSignal(0);

  // Filter suggestions by prefix (case-insensitive), excluding exact match
  const filtered = () => {
    const input = props.inputValue.toLowerCase().trim();
    if (!input) return [];
    return props.suggestions.filter((s) => {
      const lower = s.toLowerCase();
      return lower.startsWith(input) && lower !== input;
    });
  };

  // Reset selection index when filtered list changes
  createEffect(() => {
    // Access filtered to subscribe
    const list = filtered();
    if (list.length > 0) {
      setSelectedIndex(0);
    }
  });

  /** Handle keyboard navigation. Returns true if the event was consumed. */
  function handleKeyDown(e: KeyboardEvent): boolean {
    const list = filtered();
    if (!props.visible || list.length === 0) return false;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, list.length - 1));
      return true;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
      return true;
    }
    if (e.key === 'Tab' || (e.key === 'Enter' && list.length > 0)) {
      // Only consume if there are suggestions visible
      const idx = selectedIndex();
      if (idx >= 0 && idx < list.length) {
        e.preventDefault();
        props.onAccept(list[idx]);
        return true;
      }
    }
    if (e.key === 'Escape') {
      props.onDismiss();
      return true;
    }
    return false;
  }

  // Expose the key handler so the parent can forward keyboard events
  // We attach it to the component instance via a ref pattern
  (AutoComplete as unknown as { handleKeyDown: typeof handleKeyDown }).handleKeyDown = handleKeyDown;

  return (
    <Show when={props.visible && filtered().length > 0}>
      <div
        class="autocomplete-dropdown"
        style={{
          position: 'absolute',
          left: `${props.left}px`,
          top: `${props.top}px`,
          width: `${Math.max(props.width, 120)}px`,
          'z-index': '20',
        }}
      >
        <For each={filtered()}>
          {(item, index) => (
            <div
              class={`autocomplete-item ${index() === selectedIndex() ? 'selected' : ''}`}
              onMouseDown={(e) => {
                e.preventDefault(); // prevent blur on the editor
                props.onAccept(item);
              }}
              onMouseEnter={() => setSelectedIndex(index())}
            >
              {item}
            </div>
          )}
        </For>
      </div>
    </Show>
  );
};

export default AutoComplete;

/**
 * Collect unique non-empty values for a given column from the cell cache.
 * Excludes formula indicators and image data URLs.
 */
export function getColumnSuggestions(
  cellCache: Map<string, { value: string }>,
  col: number,
): string[] {
  const seen = new Set<string>();
  cellCache.forEach((cell, key) => {
    const parts = key.split(':');
    if (parseInt(parts[1], 10) !== col) return;
    const v = cell.value?.trim();
    if (!v) return;
    // Skip numeric values, formulas, and data URLs
    if (!isNaN(Number(v))) return;
    if (v.startsWith('data:image/')) return;
    seen.add(v);
  });
  return Array.from(seen).sort();
}
