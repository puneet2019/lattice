import type { Component } from 'solid-js';
import { createSignal, For, onMount, Show } from 'solid-js';
import { getColumnValues, applyColumnFilter } from '../bridge/tauri';
import type { FilterInfo } from '../bridge/tauri';

export interface FilterDropdownProps {
  activeSheet: string;
  col: number;
  x: number;
  y: number;
  onClose: () => void;
  onApply: (info: FilterInfo) => void;
}

const FilterDropdown: Component<FilterDropdownProps> = (props) => {
  const [allValues, setAllValues] = createSignal<string[]>([]);
  const [checkedValues, setCheckedValues] = createSignal<Set<string>>(new Set());
  const [searchText, setSearchText] = createSignal('');
  const [loading, setLoading] = createSignal(true);

  onMount(async () => {
    try {
      const values = await getColumnValues(props.activeSheet, props.col);
      setAllValues(values);
      setCheckedValues(new Set(values)); // all checked by default
    } catch {
      // Fallback
    }
    setLoading(false);
  });

  const filteredValues = () => {
    const search = searchText().toLowerCase();
    if (!search) return allValues();
    return allValues().filter((v) => v.toLowerCase().includes(search));
  };

  const handleToggle = (value: string) => {
    const current = new Set(checkedValues());
    if (current.has(value)) {
      current.delete(value);
    } else {
      current.add(value);
    }
    setCheckedValues(current);
  };

  const handleSelectAll = () => {
    setCheckedValues(new Set(allValues()));
  };

  const handleClearAll = () => {
    setCheckedValues(new Set());
  };

  const handleApply = async () => {
    const selected = Array.from(checkedValues());
    try {
      const info = await applyColumnFilter(props.activeSheet, props.col, selected);
      props.onApply(info);
    } catch {
      props.onClose();
    }
  };

  return (
    <div class="format-dialog-backdrop" onClick={props.onClose} style={{ background: 'transparent' }}>
      <div
        class="filter-dropdown"
        style={{
          left: `${props.x}px`,
          top: `${props.y}px`,
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div class="filter-dropdown-header">
          <input
            class="format-dialog-input"
            type="text"
            placeholder="Search..."
            value={searchText()}
            onInput={(e) => setSearchText(e.currentTarget.value)}
            autofocus
          />
        </div>

        <div class="filter-dropdown-actions">
          <button class="filter-dropdown-action-btn" onClick={handleSelectAll}>
            Select all
          </button>
          <button class="filter-dropdown-action-btn" onClick={handleClearAll}>
            Clear
          </button>
        </div>

        <div class="filter-dropdown-list">
          <Show when={loading()}>
            <div class="filter-dropdown-item" style={{ color: 'var(--header-text)' }}>
              Loading...
            </div>
          </Show>
          <For each={filteredValues()}>
            {(value) => (
              <label class="filter-dropdown-item">
                <input
                  type="checkbox"
                  checked={checkedValues().has(value)}
                  onChange={() => handleToggle(value)}
                />
                <span>{value || '(Blanks)'}</span>
              </label>
            )}
          </For>
        </div>

        <div class="filter-dropdown-footer">
          <button class="chart-dialog-btn" onClick={props.onClose}>
            Cancel
          </button>
          <button class="chart-dialog-btn chart-dialog-btn-primary" onClick={handleApply}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
};

export default FilterDropdown;
