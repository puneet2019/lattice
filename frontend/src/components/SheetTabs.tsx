import type { Component } from 'solid-js';
import { createSignal, For, Show, onMount, onCleanup } from 'solid-js';

export interface SheetTabsProps {
  sheets: string[];
  activeSheet: string;
  onSelectSheet: (name: string) => void;
  onAddSheet: () => void;
  onRenameSheet: (oldName: string, newName: string) => void;
  onDeleteSheet: (name: string) => void;
  onDuplicateSheet: (name: string) => void;
  onTabColorChange?: (name: string, color: string | null) => void;
  onMoveSheet?: (name: string, toIndex: number) => void;
  /** Tab colors keyed by sheet name. */
  tabColors?: Record<string, string>;
}

const TAB_PRESET_COLORS = [
  '#e06666', '#f6b26b', '#ffd966', '#93c47d', '#6fa8dc', '#8e7cc3', '#c27ba0', null,
];

const SheetTabs: Component<SheetTabsProps> = (props) => {
  const [contextMenu, setContextMenu] = createSignal<{ x: number; y: number; sheet: string } | null>(null);
  const [renamingSheet, setRenamingSheet] = createSignal<string | null>(null);
  const [renameValue, setRenameValue] = createSignal('');
  const [showColorPicker, setShowColorPicker] = createSignal(false);

  let renameInputRef: HTMLInputElement | undefined;

  const handleContextMenu = (e: MouseEvent, name: string) => {
    e.preventDefault();
    setShowColorPicker(false);
    setContextMenu({ x: e.clientX, y: e.clientY, sheet: name });
  };

  const closeContextMenu = () => {
    setContextMenu(null);
    setShowColorPicker(false);
  };

  const handleDoubleClick = (name: string) => {
    setRenamingSheet(name);
    setRenameValue(name);
    requestAnimationFrame(() => {
      if (renameInputRef) {
        renameInputRef.focus();
        renameInputRef.select();
      }
    });
  };

  const commitRename = () => {
    const oldName = renamingSheet();
    const newName = renameValue().trim();
    if (oldName && newName && newName !== oldName) {
      props.onRenameSheet(oldName, newName);
    }
    setRenamingSheet(null);
  };

  const handleRenameKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      commitRename();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      setRenamingSheet(null);
    }
  };

  const handleMenuAction = (action: string) => {
    const menu = contextMenu();
    if (!menu) return;
    const sheet = menu.sheet;

    switch (action) {
      case 'rename':
        closeContextMenu();
        handleDoubleClick(sheet);
        break;
      case 'delete':
        closeContextMenu();
        if (props.sheets.length > 1) {
          props.onDeleteSheet(sheet);
        }
        break;
      case 'duplicate':
        closeContextMenu();
        props.onDuplicateSheet(sheet);
        break;
      case 'color':
        setShowColorPicker(!showColorPicker());
        break;
      case 'move-start':
        closeContextMenu();
        props.onMoveSheet?.(sheet, 0);
        break;
      case 'move-end':
        closeContextMenu();
        props.onMoveSheet?.(sheet, props.sheets.length - 1);
        break;
      default:
        break;
    }
  };

  const handleColorSelect = (color: string | null) => {
    const menu = contextMenu();
    if (!menu) return;
    props.onTabColorChange?.(menu.sheet, color);
    closeContextMenu();
  };

  const getTabColor = (name: string): string | undefined => {
    return props.tabColors?.[name];
  };

  // Close context menu on click-outside, Escape, window blur
  const handleDocMouseDown = (e: MouseEvent) => {
    if (!contextMenu()) return;
    const target = e.target as HTMLElement;
    if (target.closest('.sheet-tab-context-menu')) return;
    closeContextMenu();
  };

  const handleDocKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && contextMenu()) {
      closeContextMenu();
      e.preventDefault();
    }
  };

  const handleWinBlur = () => {
    closeContextMenu();
  };

  onMount(() => {
    document.addEventListener('mousedown', handleDocMouseDown);
    document.addEventListener('keydown', handleDocKeyDown);
    window.addEventListener('blur', handleWinBlur);
  });

  onCleanup(() => {
    document.removeEventListener('mousedown', handleDocMouseDown);
    document.removeEventListener('keydown', handleDocKeyDown);
    window.removeEventListener('blur', handleWinBlur);
  });

  return (
    <div class="sheet-tabs" onClick={closeContextMenu} role="tablist" aria-label="Sheet tabs">
      <button class="sheet-tab-add" title="Add sheet" aria-label="Add sheet" onClick={props.onAddSheet}>
        +
      </button>
      <div class="sheet-tabs-list">
        <For each={props.sheets}>
          {(name) => {
            const tabColor = () => getTabColor(name);
            return (
              <div
                class={`sheet-tab ${name === props.activeSheet ? 'active' : ''}`}
                role="tab"
                aria-selected={name === props.activeSheet}
                aria-label={name}
                onClick={() => {
                  if (renamingSheet() !== name) {
                    props.onSelectSheet(name);
                  }
                }}
                onDblClick={() => handleDoubleClick(name)}
                onContextMenu={(e) => handleContextMenu(e, name)}
                style={{
                  ...(tabColor() ? {
                    "border-bottom": `3px solid ${tabColor()}`,
                  } : {}),
                }}
              >
                {renamingSheet() === name ? (
                  <input
                    ref={renameInputRef}
                    class="sheet-tab-rename-input"
                    type="text"
                    value={renameValue()}
                    onInput={(e) => setRenameValue(e.currentTarget.value)}
                    onKeyDown={handleRenameKeyDown}
                    onBlur={commitRename}
                    onClick={(e) => e.stopPropagation()}
                  />
                ) : (
                  <span>{name}</span>
                )}
              </div>
            );
          }}
        </For>
      </div>

      {/* Context Menu */}
      <Show when={contextMenu()}>
        {(menu) => (
          <div
            class="sheet-tab-context-menu"
            style={{
              left: `${menu().x}px`,
              top: `${menu().y}px`,
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div class="context-menu-item" onClick={() => handleMenuAction('rename')}>
              Rename
            </div>
            <div class="context-menu-item" onClick={() => handleMenuAction('duplicate')}>
              Duplicate
            </div>
            <div class="context-menu-separator" />
            <div class="context-menu-item" onClick={() => handleMenuAction('color')}>
              Change color
            </div>
            <Show when={showColorPicker()}>
              <div class="sheet-tab-color-picker">
                <For each={TAB_PRESET_COLORS}>
                  {(color) => (
                    <div
                      class="sheet-tab-color-swatch"
                      style={{ background: color ?? 'transparent' }}
                      onClick={() => handleColorSelect(color)}
                      title={color ?? 'No color'}
                    >
                      {color === null && <span style={{ "font-size": "8px", color: 'var(--header-text)' }}>X</span>}
                    </div>
                  )}
                </For>
              </div>
            </Show>
            <div class="context-menu-separator" />
            <div class="context-menu-item" onClick={() => handleMenuAction('move-start')}>
              Move to beginning
            </div>
            <div class="context-menu-item" onClick={() => handleMenuAction('move-end')}>
              Move to end
            </div>
            <div class="context-menu-separator" />
            <div
              class={`context-menu-item ${props.sheets.length <= 1 ? 'disabled' : 'destructive'}`}
              onClick={() => handleMenuAction('delete')}
            >
              Delete
            </div>
          </div>
        )}
      </Show>
    </div>
  );
};

export default SheetTabs;
