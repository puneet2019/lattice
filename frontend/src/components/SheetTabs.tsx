import type { Component } from 'solid-js';
import { For } from 'solid-js';

export interface SheetTabsProps {
  sheets: string[];
  activeSheet: string;
  onSelectSheet: (name: string) => void;
  onAddSheet: () => void;
}

const SheetTabs: Component<SheetTabsProps> = (props) => {
  return (
    <div class="sheet-tabs">
      <button class="sheet-tab-add" title="Add sheet" onClick={props.onAddSheet}>
        +
      </button>
      <For each={props.sheets}>
        {(name) => (
          <div
            class={`sheet-tab ${name === props.activeSheet ? 'active' : ''}`}
            onClick={() => props.onSelectSheet(name)}
          >
            {name}
          </div>
        )}
      </For>
    </div>
  );
};

export default SheetTabs;
