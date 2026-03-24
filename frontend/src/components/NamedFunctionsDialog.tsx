import type { Component } from 'solid-js';
import { createSignal, onMount, For, Show } from 'solid-js';
import {
  listNamedFunctions,
  addNamedFunction,
  removeNamedFunction,
} from '../bridge/tauri';
import type { NamedFunctionInfo } from '../bridge/tauri';

export interface NamedFunctionsDialogProps {
  onClose: () => void;
  onStatusChange: (msg: string) => void;
}

const NamedFunctionsDialog: Component<NamedFunctionsDialogProps> = (props) => {
  const [functions, setFunctions] = createSignal<NamedFunctionInfo[]>([]);
  const [showAddForm, setShowAddForm] = createSignal(false);
  const [newName, setNewName] = createSignal('');
  const [newParams, setNewParams] = createSignal('');
  const [newBody, setNewBody] = createSignal('');
  const [newDesc, setNewDesc] = createSignal('');
  const [error, setError] = createSignal('');
  const [testResult, setTestResult] = createSignal<string | null>(null);

  const fetchFunctions = async () => {
    try {
      const list = await listNamedFunctions();
      setFunctions(list);
    } catch {
      setFunctions([]);
    }
  };

  onMount(() => {
    void fetchFunctions();
  });

  const handleAdd = async () => {
    const name = newName().trim();
    const paramsRaw = newParams().trim();
    const body = newBody().trim();
    if (!name) { setError('Name is required'); return; }
    if (!paramsRaw) { setError('At least one parameter is required'); return; }
    if (!body) { setError('Body formula is required'); return; }

    const params = paramsRaw.split(',').map((p) => p.trim()).filter(Boolean);
    if (params.length === 0) { setError('At least one parameter is required'); return; }

    const desc = newDesc().trim() || undefined;
    try {
      await addNamedFunction(name, params, body, desc);
      setShowAddForm(false);
      setNewName('');
      setNewParams('');
      setNewBody('');
      setNewDesc('');
      setError('');
      await fetchFunctions();
      props.onStatusChange(`Added named function "${name}"`);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemove = async (name: string) => {
    try {
      await removeNamedFunction(name);
      await fetchFunctions();
      props.onStatusChange(`Removed named function "${name}"`);
    } catch (e) {
      props.onStatusChange(`Failed to remove: ${e}`);
    }
  };

  const handleTest = (nf: NamedFunctionInfo) => {
    // Show a simple preview of how the function would be called
    const args = nf.params.map((p) => `<${p}>`).join(', ');
    const preview = `=${nf.name}(${args}) => ${nf.body}`;
    setTestResult(preview);
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
      <div class="paste-special-dialog" style={{ "min-width": '480px', "max-height": '560px' }}>
        <div class="paste-special-header">
          <h2>Named Functions</h2>
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

        <div class="paste-special-body" style={{ "max-height": '320px', "overflow-y": 'auto', padding: '0' }}>
          <Show when={functions().length === 0 && !showAddForm()}>
            <div style={{
              "text-align": 'center',
              padding: '20px',
              color: 'var(--header-text, #5f6368)',
              "font-size": '13px',
            }}>
              No named functions defined
            </div>
          </Show>
          <For each={functions()}>
            {(nf) => (
              <div
                style={{
                  padding: '10px 16px',
                  "border-bottom": '1px solid var(--grid-border, #e0e0e0)',
                }}
              >
                <div style={{
                  display: 'flex',
                  "align-items": 'center',
                  "justify-content": 'space-between',
                  "margin-bottom": '4px',
                }}>
                  <div style={{
                    "font-weight": '600',
                    "font-size": '13px',
                    color: 'var(--cell-text, #202124)',
                  }}>
                    {nf.name}({nf.params.join(', ')})
                  </div>
                  <div style={{ display: 'flex', gap: '4px' }}>
                    <button
                      class="chart-dialog-btn"
                      onClick={() => handleTest(nf)}
                      title="Test"
                      style={{ "font-size": '11px', padding: '2px 8px' }}
                    >
                      Test
                    </button>
                    <button
                      class="chart-overlay-close"
                      onClick={() => void handleRemove(nf.name)}
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
                </div>
                <div style={{
                  "font-size": '12px',
                  color: 'var(--header-text, #5f6368)',
                  "font-family": 'monospace',
                  "margin-bottom": nf.description ? '2px' : '0',
                }}>
                  = {nf.body}
                </div>
                <Show when={nf.description}>
                  <div style={{
                    "font-size": '11px',
                    color: 'var(--header-text, #5f6368)',
                    "font-style": 'italic',
                  }}>
                    {nf.description}
                  </div>
                </Show>
              </div>
            )}
          </For>
        </div>

        {/* Test result display */}
        <Show when={testResult()}>
          <div style={{
            padding: '8px 16px',
            "border-top": '1px solid var(--grid-border, #e0e0e0)',
            background: 'var(--header-bg, #f8f9fa)',
            "font-size": '12px',
            "font-family": 'monospace',
            color: 'var(--cell-text, #202124)',
            display: 'flex',
            "align-items": 'center',
            "justify-content": 'space-between',
          }}>
            <span>{testResult()}</span>
            <button
              class="chart-overlay-close"
              onClick={() => setTestResult(null)}
              title="Dismiss"
              style={{ "margin-left": '8px' }}
            >
              <svg width="10" height="10" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
                <line x1="2" y1="2" x2="10" y2="10" />
                <line x1="10" y1="2" x2="2" y2="10" />
              </svg>
            </button>
          </div>
        </Show>

        {/* Add form */}
        <Show when={showAddForm()}>
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
            <div style={{ display: 'flex', gap: '8px', "margin-bottom": '8px' }}>
              <input
                type="text"
                placeholder="Function name"
                value={newName()}
                onInput={(e) => setNewName(e.currentTarget.value)}
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
              <input
                type="text"
                placeholder="Parameters (x, y, ...)"
                value={newParams()}
                onInput={(e) => setNewParams(e.currentTarget.value)}
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
            </div>
            <div style={{ "margin-bottom": '8px' }}>
              <input
                type="text"
                placeholder="Body formula (e.g., x * 2 + y)"
                value={newBody()}
                onInput={(e) => setNewBody(e.currentTarget.value)}
                style={{
                  width: '100%',
                  padding: '4px 8px',
                  border: '1px solid var(--grid-border, #e0e0e0)',
                  "border-radius": '4px',
                  "font-size": '13px',
                  "font-family": 'monospace',
                  color: 'var(--cell-text)',
                  background: 'var(--cell-bg)',
                  "box-sizing": 'border-box',
                }}
              />
            </div>
            <div style={{ display: 'flex', gap: '8px', "align-items": 'center' }}>
              <input
                type="text"
                placeholder="Description (optional)"
                value={newDesc()}
                onInput={(e) => setNewDesc(e.currentTarget.value)}
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

export default NamedFunctionsDialog;
