import type { Component } from 'solid-js';
import { createSignal, For, Show, onMount, onCleanup } from 'solid-js';

export type SaveStatus = 'saved' | 'unsaved' | 'saving';

export interface StatusBarProps {
  message: string;
  mode: 'Ready' | 'Edit';
  selectionSummary: string;
  zoom: number;
  onZoomChange: (zoom: number) => void;
  filterSummary?: string;
  saveStatus?: SaveStatus;
}

const ZOOM_PRESETS = [50, 75, 100, 125, 150, 200];

interface StatToggle {
  key: string;
  label: string;
}

const STAT_OPTIONS: StatToggle[] = [
  { key: 'sum', label: 'Sum' },
  { key: 'average', label: 'Average' },
  { key: 'count', label: 'Count' },
  { key: 'min', label: 'Min' },
  { key: 'max', label: 'Max' },
];

const StatusBar: Component<StatusBarProps> = (props) => {
  const [showZoomDropdown, setShowZoomDropdown] = createSignal(false);
  const [showStatMenu, setShowStatMenu] = createSignal(false);
  const [statMenuPos, setStatMenuPos] = createSignal<{ x: number; y: number }>({ x: 0, y: 0 });
  const [enabledStats, setEnabledStats] = createSignal<Set<string>>(
    new Set(['sum', 'average', 'count', 'min', 'max']),
  );

  let zoomDropdownRef: HTMLDivElement | undefined;
  let statMenuRef: HTMLDivElement | undefined;

  const handleZoomSlider = (e: InputEvent) => {
    const val = parseInt((e.target as HTMLInputElement).value, 10);
    props.onZoomChange(val / 100);
  };

  const zoomPercent = () => Math.round(props.zoom * 100);

  const handleZoomPreset = (percent: number) => {
    props.onZoomChange(percent / 100);
    setShowZoomDropdown(false);
  };

  const toggleStat = (key: string) => {
    const current = new Set(enabledStats());
    if (current.has(key)) {
      current.delete(key);
    } else {
      current.add(key);
    }
    setEnabledStats(current);
  };

  /** Filter the selection summary string to only show enabled stats. */
  const filteredSummary = () => {
    const raw = props.selectionSummary;
    if (!raw) return '';
    // Summary format: "Sum: 123  Average: 45  Count: 3  Min: 10  Max: 100"
    const enabled = enabledStats();
    const parts = raw.split(/\s{2,}/);
    const filtered = parts.filter((part) => {
      const lower = part.toLowerCase();
      for (const key of ['sum', 'average', 'count', 'min', 'max']) {
        if (lower.startsWith(key + ':')) {
          return enabled.has(key);
        }
      }
      return true; // Keep parts that don't match any stat key
    });
    return filtered.join('  ');
  };

  const handleSummaryContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    setStatMenuPos({ x: e.clientX, y: e.clientY });
    setShowStatMenu(true);
  };

  // Close dropdowns on click outside or Escape
  const handleDocClick = (e: MouseEvent) => {
    if (showZoomDropdown() && zoomDropdownRef && !zoomDropdownRef.contains(e.target as Node)) {
      setShowZoomDropdown(false);
    }
    if (showStatMenu() && statMenuRef && !statMenuRef.contains(e.target as Node)) {
      setShowStatMenu(false);
    }
  };

  const handleDocKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      setShowZoomDropdown(false);
      setShowStatMenu(false);
    }
  };

  onMount(() => {
    document.addEventListener('mousedown', handleDocClick);
    document.addEventListener('keydown', handleDocKeyDown);
  });

  onCleanup(() => {
    document.removeEventListener('mousedown', handleDocClick);
    document.removeEventListener('keydown', handleDocKeyDown);
  });

  return (
    <div class="status-bar" role="status" aria-label="Status bar">
      <div class="status-bar-left">
        <span class={`status-mode ${props.mode === 'Edit' ? 'editing' : ''}`}>
          {props.mode}
        </span>
        <span class="status-message">{props.message}</span>
      </div>
      <div class="status-bar-center" onContextMenu={handleSummaryContextMenu}>
        <Show when={props.filterSummary}>
          <span class="status-filter-summary">{props.filterSummary}</span>
        </Show>
        <Show when={filteredSummary()}>
          <span class="status-selection-summary">{filteredSummary()}</span>
        </Show>
      </div>
      <div class="status-bar-right">
        <Show when={props.saveStatus}>
          <span class={`save-status save-status-${props.saveStatus}`}>
            <span class="save-status-dot" />
            {props.saveStatus === 'saved' ? 'All changes saved' :
             props.saveStatus === 'saving' ? 'Saving...' :
             'Unsaved changes'}
          </span>
        </Show>
        <div class="status-zoom-dropdown" ref={zoomDropdownRef} style={{ position: 'relative' }}>
          <span
            class="status-zoom-label status-zoom-clickable"
            onClick={() => setShowZoomDropdown(!showZoomDropdown())}
            title="Click to choose zoom level"
          >
            {zoomPercent()}%
          </span>
          <Show when={showZoomDropdown()}>
            <div class="status-zoom-menu">
              <For each={ZOOM_PRESETS}>
                {(preset) => (
                  <div
                    class={`status-zoom-menu-item ${preset === zoomPercent() ? 'active' : ''}`}
                    onClick={() => handleZoomPreset(preset)}
                  >
                    {preset}%
                  </div>
                )}
              </For>
            </div>
          </Show>
        </div>
        <input
          class="status-zoom-slider"
          type="range"
          min="25"
          max="200"
          step="5"
          value={zoomPercent()}
          onInput={handleZoomSlider}
          title={`Zoom: ${zoomPercent()}%`}
        />
      </div>

      {/* Right-click stat customization menu */}
      <Show when={showStatMenu()}>
        <div
          class="status-stat-menu"
          ref={statMenuRef}
          style={{
            position: 'fixed',
            left: `${statMenuPos().x}px`,
            top: `${statMenuPos().y}px`,
          }}
        >
          <For each={STAT_OPTIONS}>
            {(opt) => (
              <div
                class="status-stat-menu-item"
                onClick={() => toggleStat(opt.key)}
              >
                <span class="status-stat-check">
                  {enabledStats().has(opt.key) ? '\u2713' : ''}
                </span>
                {opt.label}
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};

export default StatusBar;
