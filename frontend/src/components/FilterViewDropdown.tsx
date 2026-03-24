import type { Component } from 'solid-js';
import { createSignal, onMount, For, Show } from 'solid-js';
import {
  listFilterViews,
  saveFilterView,
  applyFilterView,
  deleteFilterView,
  clearFilter,
} from '../bridge/tauri';
import type { FilterViewInfo } from '../bridge/tauri';

export interface FilterViewDropdownProps {
  activeSheet: string;
  onClose: () => void;
  onStatusChange: (msg: string) => void;
  onRefresh: () => void;
}

const FilterViewDropdown: Component<FilterViewDropdownProps> = (props) => {
  const [views, setViews] = createSignal<FilterViewInfo[]>([]);
  const [showSaveForm, setShowSaveForm] = createSignal(false);
  const [newName, setNewName] = createSignal('');
  const [error, setError] = createSignal('');

  const fetchViews = async () => {
    try {
      const list = await listFilterViews();
      setViews(list);
    } catch {
      setViews([]);
    }
  };

  onMount(() => {
    void fetchViews();
  });

  const handleApply = async (name: string) => {
    try {
      const hidden = await applyFilterView(props.activeSheet, name);
      props.onStatusChange(`Filter view "${name}" applied (${hidden} rows hidden)`);
      props.onRefresh();
      props.onClose();
    } catch (e) {
      props.onStatusChange(`Apply filter view failed: ${e}`);
    }
  };

  const handleSave = async () => {
    const name = newName().trim();
    if (!name) {
      setError('Name is required');
      return;
    }
    try {
      // Save current filter state as a new view (empty column_filters for now --
      // the backend captures the active filter state).
      await saveFilterView(name, {});
      setShowSaveForm(false);
      setNewName('');
      setError('');
      await fetchViews();
      props.onStatusChange(`Filter view "${name}" saved`);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleClear = async () => {
    try {
      await clearFilter(props.activeSheet);
      props.onStatusChange('Filter cleared');
      props.onRefresh();
      props.onClose();
    } catch (e) {
      props.onStatusChange(`Clear filter failed: ${e}`);
    }
  };

  const handleDelete = async (name: string) => {
    try {
      await deleteFilterView(name);
      await fetchViews();
      props.onStatusChange(`Filter view "${name}" deleted`);
    } catch (e) {
      props.onStatusChange(`Delete filter view failed: ${e}`);
    }
  };

  const handleBackdropClick = (e: MouseEvent) => {
    if ((e.target as HTMLElement).classList.contains('paste-special-backdrop')) {
      props.onClose();
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      props.onClose();
    }
  };

  return (
    <div
      class="paste-special-backdrop"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
    >
      <div class="paste-special-dialog" style={{ "min-width": '320px' }}>
        <div class="paste-special-header">
          <h2>Filter Views</h2>
          <button
            class="chart-overlay-close"
            onClick={() => props.onClose()}
            title="Close"
          >
            <svg
              width="12"
              height="12"
              viewBox="0 0 12 12"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            >
              <line x1="2" y1="2" x2="10" y2="10" />
              <line x1="10" y1="2" x2="2" y2="10" />
            </svg>
          </button>
        </div>

        <div class="paste-special-body" style={{ "max-height": '280px', "overflow-y": 'auto', padding: '0' }}>
          <Show when={views().length === 0 && !showSaveForm()}>
            <div style={{
              "text-align": 'center',
              padding: '20px',
              color: 'var(--header-text, #5f6368)',
              "font-size": '13px',
            }}>
              No saved filter views
            </div>
          </Show>
          <For each={views()}>
            {(view) => (
              <div
                style={{
                  display: 'flex',
                  "align-items": 'center',
                  "justify-content": 'space-between',
                  padding: '8px 16px',
                  "border-bottom": '1px solid var(--grid-border, #e0e0e0)',
                  cursor: 'pointer',
                }}
                onClick={() => void handleApply(view.name)}
                title={`Apply "${view.name}"`}
              >
                <div style={{
                  flex: '1',
                  "font-size": '13px',
                  color: 'var(--cell-text, #202124)',
                }}>
                  {view.name}
                </div>
                <button
                  class="chart-overlay-close"
                  onClick={(e) => { e.stopPropagation(); void handleDelete(view.name); }}
                  title="Delete view"
                >
                  <svg
                    width="10"
                    height="10"
                    viewBox="0 0 12 12"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                  >
                    <line x1="2" y1="2" x2="10" y2="10" />
                    <line x1="10" y1="2" x2="2" y2="10" />
                  </svg>
                </button>
              </div>
            )}
          </For>
        </div>

        <Show when={showSaveForm()}>
          <div style={{
            padding: '12px 16px',
            "border-top": '1px solid var(--grid-border, #e0e0e0)',
          }}>
            <Show when={error()}>
              <div style={{
                color: '#d93025',
                "font-size": '12px',
                "margin-bottom": '8px',
              }}>
                {error()}
              </div>
            </Show>
            <div style={{ display: 'flex', gap: '8px' }}>
              <input
                type="text"
                placeholder="View name"
                value={newName()}
                onInput={(e) => setNewName(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void handleSave();
                }}
                style={{
                  flex: '1',
                  padding: '4px 8px',
                  border: '1px solid var(--grid-border, #e0e0e0)',
                  "border-radius": '4px',
                  "font-size": '13px',
                  "font-family": 'var(--font-family)',
                  color: 'var(--cell-text)',
                  background: 'var(--cell-bg)',
                }}
              />
              <button
                class="chart-dialog-btn chart-dialog-btn-primary"
                onClick={() => void handleSave()}
                style={{ "font-size": '12px' }}
              >
                Save
              </button>
              <button
                class="chart-dialog-btn"
                onClick={() => { setShowSaveForm(false); setError(''); }}
                style={{ "font-size": '12px' }}
              >
                Cancel
              </button>
            </div>
          </div>
        </Show>

        <div class="paste-special-footer" style={{ "justify-content": 'space-between' }}>
          <div style={{ display: 'flex', gap: '8px' }}>
            <button
              class="chart-dialog-btn"
              onClick={() => void handleClear()}
            >
              Clear filter
            </button>
          </div>
          <div style={{ display: 'flex', gap: '8px' }}>
            <Show when={!showSaveForm()}>
              <button
                class="chart-dialog-btn chart-dialog-btn-primary"
                onClick={() => setShowSaveForm(true)}
              >
                Save current filter
              </button>
            </Show>
            <button
              class="chart-dialog-btn"
              onClick={() => props.onClose()}
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default FilterViewDropdown;
