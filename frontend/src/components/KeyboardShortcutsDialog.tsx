import type { Component } from 'solid-js';
import { For, onMount, onCleanup } from 'solid-js';

export interface KeyboardShortcutsDialogProps {
  onClose: () => void;
}

interface ShortcutEntry {
  keys: string;
  description: string;
}

interface ShortcutCategory {
  title: string;
  shortcuts: ShortcutEntry[];
}

const SHORTCUT_CATEGORIES: ShortcutCategory[] = [
  {
    title: 'Navigation',
    shortcuts: [
      { keys: 'Arrow keys', description: 'Move one cell' },
      { keys: 'Tab / Shift+Tab', description: 'Move right / left' },
      { keys: 'Enter / Shift+Enter', description: 'Move down / up' },
      { keys: 'Home', description: 'Go to column A in current row' },
      { keys: 'End', description: 'Go to last used column in current row' },
      { keys: 'Cmd+Home', description: 'Go to cell A1' },
      { keys: 'Cmd+End', description: 'Go to last used cell' },
      { keys: 'Cmd+Arrow', description: 'Jump to edge of data region' },
      { keys: 'Page Down / Page Up', description: 'Scroll one page down / up' },
      { keys: 'Cmd+Backspace', description: 'Scroll to active cell' },
      { keys: 'Option+Down', description: 'Switch to next sheet' },
      { keys: 'Option+Up', description: 'Switch to previous sheet' },
    ],
  },
  {
    title: 'Selection',
    shortcuts: [
      { keys: 'Shift+Arrow', description: 'Extend selection by one cell' },
      { keys: 'Cmd+Shift+Arrow', description: 'Extend selection to edge' },
      { keys: 'Cmd+Shift+Home', description: 'Extend selection to A1' },
      { keys: 'Cmd+Shift+End', description: 'Extend selection to last used cell' },
      { keys: 'Shift+Page Down/Up', description: 'Extend selection by one page' },
      { keys: 'Cmd+A', description: 'Select all cells' },
      { keys: 'Ctrl+Space', description: 'Select entire column' },
      { keys: 'Shift+Space', description: 'Select entire row' },
    ],
  },
  {
    title: 'Editing',
    shortcuts: [
      { keys: 'F2', description: 'Enter edit mode' },
      { keys: 'Escape', description: 'Cancel editing' },
      { keys: 'Delete / Backspace', description: 'Clear cell content' },
      { keys: 'Cmd+Z', description: 'Undo' },
      { keys: 'Cmd+Shift+Z', description: 'Redo' },
      { keys: 'Cmd+C', description: 'Copy' },
      { keys: 'Cmd+X', description: 'Cut' },
      { keys: 'Cmd+V', description: 'Paste' },
      { keys: 'Cmd+Shift+V', description: 'Paste values only' },
      { keys: 'Cmd+D', description: 'Fill down' },
      { keys: 'Cmd+R', description: 'Fill right' },
      { keys: 'Cmd+;', description: 'Insert current date' },
      { keys: 'Cmd+Shift+;', description: 'Insert current time' },
      { keys: 'Option+Enter', description: 'Insert line break in cell' },
      { keys: 'Cmd+Enter', description: 'Fill selection with value' },
    ],
  },
  {
    title: 'Formatting',
    shortcuts: [
      { keys: 'Cmd+B', description: 'Bold' },
      { keys: 'Cmd+I', description: 'Italic' },
      { keys: 'Cmd+U', description: 'Underline' },
      { keys: 'Cmd+Shift+K', description: 'Strikethrough' },
      { keys: 'Cmd+Shift+E', description: 'Center align' },
      { keys: 'Cmd+Shift+L', description: 'Left align' },
      { keys: 'Cmd+Shift+R', description: 'Right align' },
      { keys: 'Cmd+Shift+.', description: 'Increase font size' },
      { keys: 'Cmd+Shift+,', description: 'Decrease font size' },
      { keys: 'Cmd+\\', description: 'Clear formatting' },
      { keys: 'Ctrl+Shift+1', description: 'Number format' },
      { keys: 'Ctrl+Shift+3', description: 'Date format' },
      { keys: 'Ctrl+Shift+4', description: 'Currency format' },
      { keys: 'Ctrl+Shift+5', description: 'Percentage format' },
    ],
  },
  {
    title: 'View',
    shortcuts: [
      { keys: 'Cmd+=', description: 'Zoom in' },
      { keys: 'Cmd+-', description: 'Zoom out' },
      { keys: 'Cmd+0', description: 'Reset zoom' },
      { keys: 'Ctrl+`', description: 'Toggle formula view' },
      { keys: 'Cmd+F', description: 'Find' },
      { keys: 'Cmd+H', description: 'Find and replace' },
      { keys: 'Cmd+/', description: 'Keyboard shortcuts help' },
    ],
  },
  {
    title: 'Data',
    shortcuts: [
      { keys: 'Cmd+Option+9', description: 'Hide selected rows' },
      { keys: 'Cmd+Shift+9', description: 'Unhide rows' },
      { keys: 'Cmd+Option+0', description: 'Hide selected columns' },
      { keys: 'Cmd+Shift+0', description: 'Unhide columns' },
      { keys: 'Ctrl+F3', description: 'Named ranges manager' },
      { keys: 'Cmd+K', description: 'Insert hyperlink' },
      { keys: 'Cmd+P', description: 'Print preview' },
    ],
  },
];

const KeyboardShortcutsDialog: Component<KeyboardShortcutsDialogProps> = (props) => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      props.onClose();
    }
  };

  const handleBackdropClick = (e: MouseEvent) => {
    if ((e.target as HTMLElement).classList.contains('kbd-shortcuts-overlay')) {
      props.onClose();
    }
  };

  onMount(() => {
    document.addEventListener('keydown', handleKeyDown);
  });

  onCleanup(() => {
    document.removeEventListener('keydown', handleKeyDown);
  });

  return (
    <div
      class="kbd-shortcuts-overlay"
      onClick={handleBackdropClick}
      role="dialog"
      aria-label="Keyboard shortcuts"
    >
      <div class="kbd-shortcuts-dialog">
        <div class="kbd-shortcuts-header">
          <h2>Keyboard Shortcuts</h2>
          <button
            class="kbd-shortcuts-close"
            onClick={props.onClose}
            aria-label="Close keyboard shortcuts dialog"
          >
            &times;
          </button>
        </div>
        <div class="kbd-shortcuts-body">
          <For each={SHORTCUT_CATEGORIES}>
            {(category) => (
              <div class="kbd-shortcuts-category">
                <h3>{category.title}</h3>
                <table class="kbd-shortcuts-table">
                  <tbody>
                    <For each={category.shortcuts}>
                      {(shortcut) => (
                        <tr>
                          <td class="kbd-shortcuts-keys">
                            <kbd>{shortcut.keys}</kbd>
                          </td>
                          <td class="kbd-shortcuts-desc">{shortcut.description}</td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
};

export default KeyboardShortcutsDialog;
