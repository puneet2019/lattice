import type { Component } from 'solid-js';
import { createSignal, onMount, For, Show } from 'solid-js';
import { saveVersion, listVersions, restoreVersion } from '../bridge/tauri';
import type { VersionInfo } from '../bridge/tauri';

export interface VersionHistoryDialogProps {
  onClose: () => void;
  /** Called after a version is restored so App can reload workbook state. */
  onRestore: (info: { sheets: string[]; active_sheet: string }) => void;
  onStatusChange: (msg: string) => void;
}

/** Format a unix timestamp (seconds) as a human-readable date/time string. */
function formatTimestamp(ts: number): string {
  if (ts === 0) return 'Unknown';
  const d = new Date(ts * 1000);
  return d.toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** Format file size in human-readable format. */
function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const VersionHistoryDialog: Component<VersionHistoryDialogProps> = (props) => {
  const [versions, setVersions] = createSignal<VersionInfo[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [showSaveForm, setShowSaveForm] = createSignal(false);
  const [description, setDescription] = createSignal('');
  const [selectedIndex, setSelectedIndex] = createSignal<number | null>(null);

  const fetchVersions = async () => {
    try {
      const list = await listVersions();
      setVersions(list);
    } catch {
      setVersions([]);
    }
  };

  onMount(() => {
    void fetchVersions();
  });

  const handleSave = async () => {
    const desc = description().trim() || 'Manual save';
    setLoading(true);
    try {
      await saveVersion(desc);
      setDescription('');
      setShowSaveForm(false);
      await fetchVersions();
      props.onStatusChange(`Version saved: ${desc}`);
    } catch (e) {
      props.onStatusChange(`Save version failed: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleRestore = async (index: number) => {
    setLoading(true);
    try {
      const info = await restoreVersion(index);
      props.onStatusChange('Version restored');
      props.onRestore(info);
      props.onClose();
    } catch (e) {
      props.onStatusChange(`Restore version failed: ${e}`);
    } finally {
      setLoading(false);
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
      <div class="paste-special-dialog" style={{ "min-width": '440px', "max-height": '520px' }}>
        <div class="paste-special-header">
          <h2>Version History</h2>
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

        <div style={{ padding: '8px 16px', "border-bottom": '1px solid var(--grid-border, #e0e0e0)' }}>
          <Show when={!showSaveForm()}>
            <button
              class="paste-special-button"
              onClick={() => setShowSaveForm(true)}
              disabled={loading()}
              style={{ width: '100%' }}
            >
              Save Current Version
            </button>
          </Show>
          <Show when={showSaveForm()}>
            <div style={{ display: 'flex', gap: '8px' }}>
              <input
                type="text"
                value={description()}
                onInput={(e) => setDescription(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    void handleSave();
                  }
                }}
                placeholder="Version description..."
                style={{
                  flex: '1',
                  padding: '6px 8px',
                  border: '1px solid var(--grid-border, #e0e0e0)',
                  "border-radius": '4px',
                  "font-size": '13px',
                  background: 'var(--cell-bg, #fff)',
                  color: 'var(--cell-text, #202124)',
                }}
              />
              <button
                class="paste-special-button primary"
                onClick={() => void handleSave()}
                disabled={loading()}
              >
                Save
              </button>
              <button
                class="paste-special-button"
                onClick={() => { setShowSaveForm(false); setDescription(''); }}
              >
                Cancel
              </button>
            </div>
          </Show>
        </div>

        <div class="paste-special-body" style={{ "max-height": '340px', "overflow-y": 'auto' }}>
          <Show when={versions().length === 0}>
            <div style={{
              "text-align": 'center',
              padding: '20px',
              color: 'var(--header-text, #5f6368)',
              "font-size": '13px',
            }}>
              No saved versions
            </div>
          </Show>
          <For each={versions()}>
            {(v) => (
              <div
                style={{
                  display: 'flex',
                  "align-items": 'center',
                  "justify-content": 'space-between',
                  padding: '8px 12px',
                  "border-bottom": '1px solid var(--grid-border, #e0e0e0)',
                  cursor: 'pointer',
                  background: selectedIndex() === v.index ? 'var(--selection-bg, rgba(26,115,232,0.08))' : 'transparent',
                }}
                onClick={() => setSelectedIndex(v.index)}
              >
                <div style={{ flex: '1' }}>
                  <div style={{
                    "font-weight": '600',
                    "font-size": '13px',
                    color: 'var(--cell-text, #202124)',
                  }}>
                    {v.description || 'Untitled version'}
                  </div>
                  <div style={{
                    "font-size": '11px',
                    color: 'var(--header-text, #5f6368)',
                    "margin-top": '2px',
                  }}>
                    {formatTimestamp(v.timestamp)} &middot; {formatSize(v.size)}
                  </div>
                </div>
                <Show when={selectedIndex() === v.index}>
                  <button
                    class="paste-special-button primary"
                    onClick={(e) => {
                      e.stopPropagation();
                      void handleRestore(v.index);
                    }}
                    disabled={loading()}
                    style={{ "margin-left": '8px', "white-space": 'nowrap' }}
                  >
                    Restore
                  </button>
                </Show>
              </div>
            )}
          </For>
        </div>

        <div class="paste-special-footer">
          <button class="paste-special-button" onClick={() => props.onClose()}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
};

export default VersionHistoryDialog;
