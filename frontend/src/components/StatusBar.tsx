import type { Component } from 'solid-js';
import { Show } from 'solid-js';

export interface StatusBarProps {
  message: string;
  mode: 'Ready' | 'Edit';
  selectionSummary: string;
  zoom: number;
  onZoomChange: (zoom: number) => void;
}

const StatusBar: Component<StatusBarProps> = (props) => {
  const handleZoomSlider = (e: InputEvent) => {
    const val = parseInt((e.target as HTMLInputElement).value, 10);
    props.onZoomChange(val / 100);
  };

  const zoomPercent = () => Math.round(props.zoom * 100);

  return (
    <div class="status-bar">
      <div class="status-bar-left">
        <span class={`status-mode ${props.mode === 'Edit' ? 'editing' : ''}`}>
          {props.mode}
        </span>
        <span class="status-message">{props.message}</span>
      </div>
      <div class="status-bar-center">
        <Show when={props.selectionSummary}>
          <span class="status-selection-summary">{props.selectionSummary}</span>
        </Show>
      </div>
      <div class="status-bar-right">
        <span class="status-zoom-label">{zoomPercent()}%</span>
        <input
          class="status-zoom-slider"
          type="range"
          min="50"
          max="200"
          step="10"
          value={zoomPercent()}
          onInput={handleZoomSlider}
          title={`Zoom: ${zoomPercent()}%`}
        />
      </div>
    </div>
  );
};

export default StatusBar;
