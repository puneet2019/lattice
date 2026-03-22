import type { Component } from 'solid-js';
import { For, Show } from 'solid-js';
import {
  extractCurrentToken,
  filterFormulaFunctions,
} from './formulaFunctions';

// Re-export utilities and types for backward compatibility
export { FORMULA_FUNCTIONS, extractCurrentToken, filterFormulaFunctions } from './formulaFunctions';
export type { FormulaFunction } from './formulaFunctions';

export interface FormulaAutoCompleteProps {
  /** Current editor text (including =). */
  inputValue: string;
  /** CSS position for the dropdown. */
  left: number;
  top: number;
  width: number;
  /** Whether the dropdown should be visible. */
  visible: boolean;
  /** Selected index (managed externally for keyboard nav). */
  selectedIndex: number;
  /** Called when user clicks a suggestion. Value is the function name (e.g. "SUM"). */
  onAccept: (funcName: string) => void;
  /** Called when the user dismisses the dropdown. */
  onDismiss: () => void;
}

const FormulaAutoComplete: Component<FormulaAutoCompleteProps> = (props) => {
  const filtered = () => {
    if (!props.visible) return [];
    const token = extractCurrentToken(props.inputValue);
    return filterFormulaFunctions(token);
  };

  return (
    <Show when={props.visible && filtered().length > 0}>
      <div
        class="autocomplete-dropdown formula-autocomplete"
        style={{
          position: 'absolute',
          left: `${props.left}px`,
          top: `${props.top}px`,
          width: `${Math.max(props.width, 260)}px`,
          'z-index': '25',
        }}
      >
        <For each={filtered()}>
          {(item, index) => (
            <div
              class={`autocomplete-item ${index() === props.selectedIndex ? 'selected' : ''}`}
              onMouseDown={(e) => {
                e.preventDefault();
                props.onAccept(item.name);
              }}
            >
              <span class="formula-ac-name">{item.name}</span>
              <span class="formula-ac-sig">{item.signature}</span>
            </div>
          )}
        </For>
      </div>
    </Show>
  );
};

export default FormulaAutoComplete;
