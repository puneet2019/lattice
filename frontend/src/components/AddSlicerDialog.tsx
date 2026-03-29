import type { Component } from 'solid-js';
import { createSignal, For } from 'solid-js';
import { col_to_letter } from '../bridge/tauri_helpers';
import { TOTAL_COLS } from './Grid/constants';

export interface AddSlicerDialogProps {
  onAdd: (col: number, colName: string) => void;
  onClose: () => void;
}

/** Maximum columns to show in the slicer column picker. */
const MAX_COLS_SHOWN = 52; // A-AZ

const AddSlicerDialog: Component<AddSlicerDialogProps> = (props) => {
  const [selectedCol, setSelectedCol] = createSignal(0);

  const columnOptions = () => {
    const limit = Math.min(TOTAL_COLS, MAX_COLS_SHOWN);
    const opts: { value: number; label: string }[] = [];
    for (let i = 0; i < limit; i++) {
      opts.push({ value: i, label: `Column ${col_to_letter(i)}` });
    }
    return opts;
  };

  const handleOk = () => {
    const col = selectedCol();
    props.onAdd(col, col_to_letter(col));
  };

  return (
    <div class="slicer-dialog-backdrop" onClick={props.onClose}>
      <div class="slicer-dialog" onClick={(e) => e.stopPropagation()}>
        <div class="slicer-dialog-header">Add Slicer</div>
        <div class="slicer-dialog-body">
          <label class="slicer-dialog-label">Select column for slicer:</label>
          <select
            class="slicer-dialog-select"
            value={selectedCol()}
            onChange={(e) => setSelectedCol(parseInt(e.currentTarget.value, 10))}
          >
            <For each={columnOptions()}>
              {(opt) => <option value={opt.value}>{opt.label}</option>}
            </For>
          </select>
        </div>
        <div class="slicer-dialog-footer">
          <button class="slicer-dialog-btn" onClick={props.onClose}>
            Cancel
          </button>
          <button
            class="slicer-dialog-btn slicer-dialog-btn-primary"
            onClick={handleOk}
          >
            OK
          </button>
        </div>
      </div>
    </div>
  );
};

export default AddSlicerDialog;
