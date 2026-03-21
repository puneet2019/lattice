import type { Component } from 'solid-js';

export interface ToolbarProps {
  onBold: () => void;
  onItalic: () => void;
  onUnderline: () => void;
}

const Toolbar: Component<ToolbarProps> = (props) => {
  return (
    <div class="toolbar">
      <button
        class="toolbar-btn"
        title="Bold (Cmd+B)"
        onClick={props.onBold}
      >
        <strong>B</strong>
      </button>
      <button
        class="toolbar-btn"
        title="Italic (Cmd+I)"
        onClick={props.onItalic}
      >
        <em>I</em>
      </button>
      <button
        class="toolbar-btn"
        title="Underline (Cmd+U)"
        onClick={props.onUnderline}
      >
        <span style={{ "text-decoration": "underline" }}>U</span>
      </button>
      <div class="toolbar-separator" />
      <span style={{ "font-size": "11px", color: "var(--header-text)" }}>
        Lattice
      </span>
    </div>
  );
};

export default Toolbar;
