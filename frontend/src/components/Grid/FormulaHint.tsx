import type { Component } from 'solid-js';
import { Show } from 'solid-js';
import { FORMULA_FUNCTIONS } from './formulaFunctions';

export interface FormulaHintProps {
  /** Current editor text (including =). */
  inputValue: string;
  /** Cursor position within the editor text. */
  cursorPos: number;
  /** CSS position for the tooltip. */
  left: number;
  top: number;
  /** Whether the hint should be considered for display. */
  visible: boolean;
}

/**
 * Find the innermost function call enclosing the cursor position.
 * Returns { funcName, argIndex } or null if cursor is not inside a call.
 *
 * Example: `=SUM(A1, IF(B1>0, |` (cursor at |)
 *  - innermost function is IF, argIndex = 1
 */
export function findEnclosingFunction(
  text: string,
  cursorPos: number,
): { funcName: string; argIndex: number } | null {
  // Strip the leading = if present
  const formula = text.startsWith('=') ? text.slice(1) : text;
  const pos = text.startsWith('=') ? cursorPos - 1 : cursorPos;

  if (pos < 0 || pos > formula.length) return null;

  // Walk backward from cursor to find the most recent unmatched '('
  let depth = 0;
  let i = Math.min(pos, formula.length) - 1;
  while (i >= 0) {
    const ch = formula[i];
    if (ch === ')') {
      depth++;
    } else if (ch === '(') {
      if (depth === 0) {
        // Found the unmatched open paren — extract the function name before it
        let nameEnd = i;
        let nameStart = nameEnd - 1;
        while (nameStart >= 0 && /[A-Za-z0-9_.]/.test(formula[nameStart])) {
          nameStart--;
        }
        nameStart++;
        const funcName = formula.slice(nameStart, nameEnd).toUpperCase();
        if (funcName.length === 0) return null;

        // Count commas between the '(' and cursor to determine argument index
        let argIndex = 0;
        let innerDepth = 0;
        for (let j = i + 1; j < pos && j < formula.length; j++) {
          if (formula[j] === '(') innerDepth++;
          else if (formula[j] === ')') innerDepth--;
          else if (formula[j] === ',' && innerDepth === 0) argIndex++;
        }

        return { funcName, argIndex };
      } else {
        depth--;
      }
    }
    i--;
  }

  return null;
}

const FormulaHint: Component<FormulaHintProps> = (props) => {
  const hintData = () => {
    if (!props.visible) return null;
    const result = findEnclosingFunction(props.inputValue, props.cursorPos);
    if (!result) return null;

    // Look up the function in FORMULA_FUNCTIONS
    const func = FORMULA_FUNCTIONS.find(
      (f) => f.name === result.funcName,
    );
    if (!func) return null;

    return { signature: func.signature, argIndex: result.argIndex };
  };

  return (
    <Show when={hintData()}>
      {(data) => (
        <div
          class="formula-hint-tooltip"
          style={{
            left: `${props.left}px`,
            top: `${props.top}px`,
          }}
        >
          {data().signature}
        </div>
      )}
    </Show>
  );
};

export default FormulaHint;
