import type { Component } from 'solid-js';
import { createSignal, For, Show } from 'solid-js';

export interface SheetTabsProps {
  sheets: string[];
  activeSheet: string;
  onSelectSheet: (name: string) => void;
  onAddSheet: () => void;
  onRenameSheet: (oldName: string, newName: string) => void;
  onDeleteSheet: (name: string) => void;
  onDuplicateSheet: (name: string) => void;
}

const SheetTabs: Component<SheetTabsProps> = (props) => {
  const [contextMenu, setContextMenu] = createSignal<{ x: number; y: number; sheet: string } | null>(null);
  const [renamingSheet, setRenamingSheet] = createSignal<string | null>(null);
  const [renameValue, setRenameValue] = createSignal('');

  let renameInputRef: HTMLInputElement | undefined;

  const handleContextMenu = (e: MouseEvent, name: string) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, sheet: name });
  };

  const closeContextMenu = () => setContextMenu(null);

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
    closeContextMenu();

    switch (action) {
      case 'rename':
        handleDoubleClick(sheet);
        break;
      case 'delete':
        if (props.sheets.length > 1) {
          props.onDeleteSheet(sheet);
        }
        break;
      case 'duplicate':
        props.onDuplicateSheet(sheet);
        break;
      default:
        break;
    }
  };

  return (
    <div class="sheet-tabs" onClick={closeContextMenu}>
      <button class="sheet-tab-add" title="Add sheet" onClick={props.onAddSheet}>
        +
      </button>
      <div class="sheet-tabs-list">
        <For each={props.sheets}>
          {(name) => (
            <div
              class={`sheet-tab ${name === props.activeSheet ? 'active' : ''}`}
              onClick={() => {
                if (renamingSheet() !== name) {
                  props.onSelectSheet(name);
                }
              }}
              onDblClick={() => handleDoubleClick(name)}
              onContextMenu={(e) => handleContextMenu(e, name)}
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
          )}
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
