import type { Component } from 'solid-js';
import { createSignal, createEffect, For, Show } from 'solid-js';

// Re-export from utility file for backward compatibility
export { getColumnSuggestions, filterSuggestions } from './autoCompleteUtils';

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
    const list = filtered();
    if (list.length > 0) {
      setSelectedIndex(0);
    }
  });

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
