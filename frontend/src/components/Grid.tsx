import type { Component } from 'solid-js';
import { For, createSignal, createEffect, onMount, onCleanup } from 'solid-js';
import { getCell, setCell } from '../bridge/tauri';
import type { CellData } from '../bridge/tauri';

const NUM_ROWS = 50;
const NUM_COLS = 26;

/** Convert a 0-based column index to a letter (A-Z). */
function colToLetter(col: number): string {
  let result = '';
  let c = col;
  do {
    result = String.fromCharCode(65 + (c % 26)) + result;
    c = Math.floor(c / 26) - 1;
  } while (c >= 0);
  return result;
}

export interface GridProps {
  /** The active sheet name. */
  activeSheet: string;
  /** The currently selected cell [row, col] (0-based). */
  selectedCell: [number, number];
  /** Called when the user selects a cell. */
  onSelectCell: (row: number, col: number) => void;
  /** Called when a cell value is committed (after editing). */
  onCellCommit: (row: number, col: number, value: string) => void;
  /** The current cell content to display in the formula bar (set by parent). */
  onContentChange: (content: string) => void;
}

const Grid: Component<GridProps> = (props) => {
  // Local cell cache: Map<"row,col", CellData | null>
  const [cellCache, setCellCache] = createSignal<Map<string, CellData | null>>(new Map());
  const [editingCell, setEditingCell] = createSignal<[number, number] | null>(null);
  const [editValue, setEditValue] = createSignal('');

  let gridRef: HTMLDivElement | undefined;

  const cellKey = (row: number, col: number) => `${row},${col}`;

  // Load visible cells when sheet changes.
  const loadCells = async () => {
    const cache = new Map<string, CellData | null>();
    // Load in batches — for MVP, load all 50x26 cells.
    // In production, this would use virtual scrolling and only load visible cells.
    const promises: Promise<void>[] = [];
    for (let r = 0; r < NUM_ROWS; r++) {
      for (let c = 0; c < NUM_COLS; c++) {
        const row = r;
        const col = c;
        promises.push(
          getCell(props.activeSheet, row, col)
            .then((data) => {
              cache.set(cellKey(row, col), data);
            })
            .catch(() => {
              cache.set(cellKey(row, col), null);
            }),
        );
      }
    }
    await Promise.all(promises);
    setCellCache(cache);
  };

  // Reload cells when the active sheet changes.
  createEffect(() => {
    void props.activeSheet;
    loadCells();
  });

  // Update formula bar content when selection changes.
  createEffect(() => {
    const [row, col] = props.selectedCell;
    const data = cellCache().get(cellKey(row, col));
    if (data) {
      props.onContentChange(data.formula ? `=${data.formula}` : data.value);
    } else {
      props.onContentChange('');
    }
  });

  const handleCellClick = (row: number, col: number) => {
    // If we were editing a different cell, commit it first.
    const currentEditing = editingCell();
    if (currentEditing) {
      commitEdit(currentEditing[0], currentEditing[1]);
    }
    props.onSelectCell(row, col);
  };

  const handleCellDoubleClick = (row: number, col: number) => {
    startEditing(row, col);
  };

  const startEditing = (row: number, col: number) => {
    const data = cellCache().get(cellKey(row, col));
    const value = data ? (data.formula ? `=${data.formula}` : data.value) : '';
    setEditValue(value);
    setEditingCell([row, col]);
  };

  const commitEdit = async (row: number, col: number) => {
    const value = editValue();
    setEditingCell(null);

    let formula: string | undefined;
    let cellValue = value;

    // If the value starts with '=', treat it as a formula.
    if (value.startsWith('=')) {
      formula = value.slice(1);
      cellValue = value; // The backend will parse the formula.
    }

    try {
      await setCell(props.activeSheet, row, col, cellValue, formula);
      // Reload the cell.
      const updated = await getCell(props.activeSheet, row, col);
      const newCache = new Map(cellCache());
      newCache.set(cellKey(row, col), updated);
      setCellCache(newCache);
      props.onCellCommit(row, col, cellValue);
    } catch (e) {
      console.error('Failed to set cell:', e);
    }
  };

  const cancelEdit = () => {
    setEditingCell(null);
  };

  // Handle keyboard navigation on the grid.
  const handleKeyDown = (e: KeyboardEvent) => {
    const currentEditing = editingCell();

    if (currentEditing) {
      // While editing a cell.
      if (e.key === 'Enter') {
        e.preventDefault();
        commitEdit(currentEditing[0], currentEditing[1]);
        // Move down.
        const nextRow = Math.min(currentEditing[0] + 1, NUM_ROWS - 1);
        props.onSelectCell(nextRow, currentEditing[1]);
      } else if (e.key === 'Tab') {
        e.preventDefault();
        commitEdit(currentEditing[0], currentEditing[1]);
        // Move right.
        const nextCol = Math.min(currentEditing[1] + 1, NUM_COLS - 1);
        props.onSelectCell(currentEditing[0], nextCol);
      } else if (e.key === 'Escape') {
        e.preventDefault();
        cancelEdit();
      }
      return;
    }

    // Navigation when not editing.
    const [row, col] = props.selectedCell;

    switch (e.key) {
      case 'ArrowUp':
        e.preventDefault();
        props.onSelectCell(Math.max(row - 1, 0), col);
        break;
      case 'ArrowDown':
        e.preventDefault();
        props.onSelectCell(Math.min(row + 1, NUM_ROWS - 1), col);
        break;
      case 'ArrowLeft':
        e.preventDefault();
        props.onSelectCell(row, Math.max(col - 1, 0));
        break;
      case 'ArrowRight':
        e.preventDefault();
        props.onSelectCell(row, Math.min(col + 1, NUM_COLS - 1));
        break;
      case 'Tab':
        e.preventDefault();
        if (e.shiftKey) {
          props.onSelectCell(row, Math.max(col - 1, 0));
        } else {
          props.onSelectCell(row, Math.min(col + 1, NUM_COLS - 1));
        }
        break;
      case 'Enter':
        e.preventDefault();
        startEditing(row, col);
        break;
      case 'Delete':
      case 'Backspace':
        e.preventDefault();
        // Clear cell.
        setCell(props.activeSheet, row, col, '').then(() => {
          const newCache = new Map(cellCache());
          newCache.set(cellKey(row, col), null);
          setCellCache(newCache);
          props.onContentChange('');
        });
        break;
      case 'F2':
        e.preventDefault();
        startEditing(row, col);
        break;
      default:
        // If a printable character is typed, start editing.
        if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
          setEditValue(e.key);
          setEditingCell([row, col]);
        }
        break;
    }
  };

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
  });

  onCleanup(() => {
    window.removeEventListener('keydown', handleKeyDown);
  });

  const getCellDisplay = (row: number, col: number): { value: string; bold: boolean; italic: boolean } => {
    const data = cellCache().get(cellKey(row, col));
    if (!data) return { value: '', bold: false, italic: false };
    return { value: data.value, bold: data.bold, italic: data.italic };
  };

  const isSelected = (row: number, col: number): boolean => {
    return props.selectedCell[0] === row && props.selectedCell[1] === col;
  };

  const isEditing = (row: number, col: number): boolean => {
    const e = editingCell();
    return e !== null && e[0] === row && e[1] === col;
  };

  return (
    <div class="grid-container" ref={gridRef}>
      <table class="grid-table">
        <thead>
          <tr>
            <th class="grid-corner" />
            <For each={Array.from({ length: NUM_COLS }, (_, i) => i)}>
              {(col) => (
                <th
                  class={`grid-col-header ${props.selectedCell[1] === col ? 'selected' : ''}`}
                >
                  {colToLetter(col)}
                </th>
              )}
            </For>
          </tr>
        </thead>
        <tbody>
          <For each={Array.from({ length: NUM_ROWS }, (_, i) => i)}>
            {(row) => (
              <tr>
                <td
                  class={`grid-row-number ${props.selectedCell[0] === row ? 'selected' : ''}`}
                >
                  {row + 1}
                </td>
                <For each={Array.from({ length: NUM_COLS }, (_, i) => i)}>
                  {(col) => {
                    const display = () => getCellDisplay(row, col);
                    return (
                      <td
                        class={`grid-cell ${isSelected(row, col) ? 'selected' : ''} ${isEditing(row, col) ? 'editing' : ''}`}
                        onClick={() => handleCellClick(row, col)}
                        onDblClick={() => handleCellDoubleClick(row, col)}
                      >
                        {isEditing(row, col) ? (
                          <input
                            class="grid-cell-input"
                            type="text"
                            value={editValue()}
                            onInput={(e) => setEditValue(e.currentTarget.value)}
                            ref={(el) => {
                              // Auto-focus the input when editing starts.
                              requestAnimationFrame(() => el.focus());
                            }}
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') {
                                e.preventDefault();
                                e.stopPropagation();
                                commitEdit(row, col);
                                props.onSelectCell(Math.min(row + 1, NUM_ROWS - 1), col);
                              } else if (e.key === 'Tab') {
                                e.preventDefault();
                                e.stopPropagation();
                                commitEdit(row, col);
                                if (e.shiftKey) {
                                  props.onSelectCell(row, Math.max(col - 1, 0));
                                } else {
                                  props.onSelectCell(row, Math.min(col + 1, NUM_COLS - 1));
                                }
                              } else if (e.key === 'Escape') {
                                e.preventDefault();
                                e.stopPropagation();
                                cancelEdit();
                              }
                            }}
                            onBlur={() => commitEdit(row, col)}
                          />
                        ) : (
                          <span
                            class={`${display().bold ? 'bold' : ''} ${display().italic ? 'italic' : ''}`}
                          >
                            {display().value}
                          </span>
                        )}
                      </td>
                    );
                  }}
                </For>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </div>
  );
};

export default Grid;
