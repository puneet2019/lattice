import type { Component } from 'solid-js';
import { createSignal, onMount, For, Show } from 'solid-js';
import {
  listNamedRanges,
  addNamedRange,
  removeNamedRange,
} from '../bridge/tauri';
import type { NamedRangeInfo } from '../bridge/tauri';

export interface NamedRangesDialogProps {
  activeSheet: string;
  /** Current selection as A1-notation range string for default range input. */
  selectionRange: string;
  onClose: () => void;
  /** Navigate to a range when user double-clicks a named range. */
  onNavigate: (ref: string) => void;
  onStatusChange: (msg: string) => void;
}

const NamedRangesDialog: Component<NamedRangesDialogProps> = (props) => {
  const [ranges, setRanges] = createSignal<NamedRangeInfo[]>([]);
  const [showAddForm, setShowAddForm] = createSignal(false);
  const [newName, setNewName] = createSignal('');
  const [newRange, setNewRange] = createSignal('');
  const [newSheet, setNewSheet] = createSignal('');
  const [error, setError] = createSignal('');

  const fetchRanges = async () => {
    try {
      const list = await listNamedRanges();
      setRanges(list);
    } catch {
      setRanges([]);
    }
  };

  onMount(() => {
    void fetchRanges();
    setNewRange(props.selectionRange);
  });

  const handleAdd = async () => {
    const name = newName().trim();
    const range = newRange().trim();
    if (!name) {
      setError('Name is required');
      return;
    }
    if (!range) {
      setError('Range is required');
      return;
    }
    const sheet = newSheet().trim() || undefined;
    try {
      await addNamedRange(name, range, sheet);
      setShowAddForm(false);
      setNewName('');
      setNewRange(props.selectionRange);
      setNewSheet('');
      setError('');
      await fetchRanges();
      props.onStatusChange(`Added named range "${name}"`);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemove = async (name: string) => {
    try {
      await removeNamedRange(name);
      await fetchRanges();
      props.onStatusChange(`Removed named range "${name}"`);
    } catch (e) {
      props.onStatusChange(`Failed to remove: ${e}`);
    }
  };

  const handleDoubleClick = (nr: NamedRangeInfo) => {
    props.onNavigate(nr.range);
    props.onClose();
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
      <div class="paste-special-dialog" style={{ "min-width": '400px', "max-height": '500px' }}>
        <div class="paste-special-header">
          <h2>Named Ranges</h2>
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

        <div class="paste-special-body" style={{ "max-height": '300px', "overflow-y": 'auto' }}>
          <Show when={ranges().length === 0}>
            <div style={{
              "text-align": 'center',
              padding: '20px',
              color: 'var(--header-text, #5f6368)',
              "font-size": '13px',
            }}>
              No named ranges defined
            </div>
          </Show>
          <For each={ranges()}>
            {(nr) => (
              <div
                style={{
                  display: 'flex',
                  "align-items": 'center',
                  "justify-content": 'space-between',
                  padding: '6px 8px',
                  "border-bottom": '1px solid var(--grid-border, #e0e0e0)',
                  cursor: 'pointer',
                }}
                onDblClick={() => handleDoubleClick(nr)}
                title="Double-click to navigate"
              >
                <div style={{ flex: '1' }}>
                  <div style={{
                    "font-weight": '600',
                    "font-size": '13px',
                    color: 'var(--cell-text, #202124)',
                  }}>
                    {nr.name}
                  </div>
                  <div style={{
                    "font-size": '11px',
                    color: 'var(--header-text, #5f6368)',
                  }}>
                    {nr.sheet ? `${nr.sheet}!${nr.range}` : nr.range}
                  </div>
                </div>
                <button
                  class="chart-overlay-close"
                  onClick={(e) => { e.stopPropagation(); void handleRemove(nr.name); }}
                  title="Delete"
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

        <Show when={showAddForm()}>
          <div style={{
            padding: '12px',
            "border-top": '1px solid var(--grid-border, #e0e0e0)',
          }}>
            <Show when={error()}>
              <div style={{
                color: 'var(--danger-color, #d93025)',
                "font-size": '12px',
                "margin-bottom": '8px',
              }}>
                {error()}
              </div>
            </Show>
            <div style={{ display: 'flex', gap: '8px', "margin-bottom": '8px' }}>
              <input
                type="text"
                placeholder="Name"
                value={newName()}
                onInput={(e) => setNewName(e.currentTarget.value)}
                style={{
                  flex: '1',
                  padding: '4px 8px',
                  border: '1px solid var(--grid-border, #e0e0e0)',
                  "border-radius": '4px',
                  "font-size": '13px',
                }}
              />
              <input
                type="text"
                placeholder="Range (e.g. A1:D10)"
                value={newRange()}
                onInput={(e) => setNewRange(e.currentTarget.value)}
                style={{
                  flex: '1',
                  padding: '4px 8px',
                  border: '1px solid var(--grid-border, #e0e0e0)',
                  "border-radius": '4px',
                  "font-size": '13px',
                }}
              />
            </div>
            <div style={{ display: 'flex', gap: '8px', "align-items": 'center' }}>
              <input
                type="text"
                placeholder="Sheet scope (optional)"
                value={newSheet()}
                onInput={(e) => setNewSheet(e.currentTarget.value)}
                style={{
                  flex: '1',
                  padding: '4px 8px',
                  border: '1px solid var(--grid-border, #e0e0e0)',
                  "border-radius": '4px',
                  "font-size": '13px',
                }}
              />
              <button
                class="chart-dialog-btn chart-dialog-btn-primary"
                onClick={() => void handleAdd()}
                style={{ "font-size": '12px' }}
              >
                Save
              </button>
              <button
                class="chart-dialog-btn"
                onClick={() => { setShowAddForm(false); setError(''); }}
                style={{ "font-size": '12px' }}
              >
                Cancel
              </button>
            </div>
          </div>
        </Show>

        <div class="paste-special-footer">
          <Show when={!showAddForm()}>
            <button
              class="chart-dialog-btn chart-dialog-btn-primary"
              onClick={() => setShowAddForm(true)}
            >
              Add
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
  );
};

export default NamedRangesDialog;
