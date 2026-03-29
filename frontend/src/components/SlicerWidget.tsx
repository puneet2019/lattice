import type { Component } from 'solid-js';
import { createSignal, createEffect, onCleanup, For, Show, onMount } from 'solid-js';
import { getColumnValues, applyColumnFilter } from '../bridge/tauri';

export interface SlicerState {
  id: string;
  sheet: string;
  col: number;
  colName: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface SlicerWidgetProps {
  slicer: SlicerState;
  activeSheet: string;
  onClose: (id: string) => void;
  onMove: (id: string, x: number, y: number) => void;
  onResize: (id: string, width: number, height: number) => void;
  onStatusChange?: (msg: string) => void;
  onRefresh?: () => void;
}

const SlicerPanel: Component<SlicerWidgetProps> = (props) => {
  const [allValues, setAllValues] = createSignal<string[]>([]);
  const [checkedValues, setCheckedValues] = createSignal<Set<string>>(new Set());
  const [searchText, setSearchText] = createSignal('');
  const [loading, setLoading] = createSignal(true);
  const [dragging, setDragging] = createSignal(false);
  const [resizing, setResizing] = createSignal(false);
  const [dragOffset, setDragOffset] = createSignal({ x: 0, y: 0 });

  /** Fetch unique values for this slicer's column. */
  const fetchValues = async () => {
    setLoading(true);
    try {
      const values: string[] = await getColumnValues(props.slicer.sheet, props.slicer.col);
      setAllValues(values);
      setCheckedValues(new Set<string>(values));
    } catch {
      setAllValues([]);
      setCheckedValues(new Set<string>());
    }
    setLoading(false);
  };

  onMount(() => {
    void fetchValues();
  });

  // Re-fetch values when the active sheet or column changes.
  createEffect(() => {
    void props.slicer.sheet;
    void props.slicer.col;
    void fetchValues();
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
    setCheckedValues(new Set<string>());
  };

  const handleApply = async () => {
    const selected = Array.from(checkedValues());
    try {
      // If nothing is checked, clear the filter for this column (show all)
      if (selected.length === 0) {
        await applyColumnFilter(props.slicer.sheet, props.slicer.col, allValues());
        props.onStatusChange?.('Slicer: showing all rows');
      } else {
        await applyColumnFilter(props.slicer.sheet, props.slicer.col, selected);
        const total = allValues().length;
        props.onStatusChange?.(
          `Slicer: ${selected.length} of ${total} values selected`,
        );
      }
      props.onRefresh?.();
    } catch (e) {
      props.onStatusChange?.(`Slicer filter failed: ${e}`);
    }
  };

  const handleClose = () => {
    // Clear the filter for this column before closing
    void applyColumnFilter(props.slicer.sheet, props.slicer.col, allValues())
      .then(() => props.onRefresh?.())
      .catch(() => {});
    props.onClose(props.slicer.id);
  };

  // -- Drag logic --
  const handleDragStart = (e: MouseEvent) => {
    e.preventDefault();
    setDragging(true);
    setDragOffset({
      x: e.clientX - props.slicer.x,
      y: e.clientY - props.slicer.y,
    });
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (dragging()) {
      const offset = dragOffset();
      props.onMove(
        props.slicer.id,
        e.clientX - offset.x,
        e.clientY - offset.y,
      );
    }
    if (resizing()) {
      const newWidth = Math.max(180, e.clientX - props.slicer.x);
      const newHeight = Math.max(200, e.clientY - props.slicer.y);
      props.onResize(props.slicer.id, newWidth, newHeight);
    }
  };

  const handleMouseUp = () => {
    setDragging(false);
    setResizing(false);
  };

  const handleResizeStart = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setResizing(true);
  };

  // Attach global listeners when dragging/resizing.
  createEffect(() => {
    if (dragging() || resizing()) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
    } else {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    }
  });

  onCleanup(() => {
    window.removeEventListener('mousemove', handleMouseMove);
    window.removeEventListener('mouseup', handleMouseUp);
  });

  const checkedCount = () => checkedValues().size;
  const totalCount = () => allValues().length;

  return (
    <div
      class="slicer-widget"
      style={{
        left: `${props.slicer.x}px`,
        top: `${props.slicer.y}px`,
        width: `${props.slicer.width}px`,
        height: `${props.slicer.height}px`,
      }}
    >
      {/* Colored accent line at top */}
      <div class="slicer-accent" />

      {/* Header bar (drag handle) */}
      <div class="slicer-header" onMouseDown={handleDragStart}>
        <span class="slicer-title">{props.slicer.colName}</span>
        <button
          class="slicer-close"
          onClick={handleClose}
          title="Remove slicer"
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

      {/* Search input */}
      <div class="slicer-search">
        <input
          class="slicer-search-input"
          type="text"
          placeholder="Search..."
          value={searchText()}
          onInput={(e) => setSearchText(e.currentTarget.value)}
        />
      </div>

      {/* Select All / Clear actions */}
      <div class="slicer-actions">
        <button class="slicer-action-btn" onClick={handleSelectAll}>
          Select all
        </button>
        <button class="slicer-action-btn" onClick={handleClearAll}>
          Clear
        </button>
        <span class="slicer-count">
          {checkedCount()}/{totalCount()}
        </span>
      </div>

      {/* Scrollable checkbox list */}
      <div class="slicer-list">
        <Show when={loading()}>
          <div class="slicer-item" style={{ color: 'var(--header-text)' }}>
            Loading...
          </div>
        </Show>
        <For each={filteredValues()}>
          {(value) => (
            <label class="slicer-item">
              <input
                type="checkbox"
                checked={checkedValues().has(value)}
                onChange={() => handleToggle(value)}
              />
              <span>{value || '(Blanks)'}</span>
            </label>
          )}
        </For>
        <Show when={!loading() && filteredValues().length === 0}>
          <div class="slicer-item" style={{ color: 'var(--header-text)' }}>
            No values found
          </div>
        </Show>
      </div>

      {/* Apply button */}
      <div class="slicer-footer">
        <button class="slicer-apply-btn" onClick={() => void handleApply()}>
          Apply
        </button>
      </div>

      {/* Resize handle (bottom-right corner) */}
      <div class="slicer-resize" onMouseDown={handleResizeStart} />
    </div>
  );
};

export interface SlicerContainerProps {
  slicers: SlicerState[];
  activeSheet: string;
  onClose: (id: string) => void;
  onMove: (id: string, x: number, y: number) => void;
  onResize: (id: string, width: number, height: number) => void;
  onStatusChange?: (msg: string) => void;
  onRefresh?: () => void;
}

/** Renders all active slicer widgets as floating overlays on the grid. */
const SlicerContainer: Component<SlicerContainerProps> = (props) => {
  return (
    <For each={props.slicers}>
      {(slicer) => (
        <SlicerPanel
          slicer={slicer}
          activeSheet={props.activeSheet}
          onClose={props.onClose}
          onMove={props.onMove}
          onResize={props.onResize}
          onStatusChange={props.onStatusChange}
          onRefresh={props.onRefresh}
        />
      )}
    </For>
  );
};

export default SlicerContainer;
