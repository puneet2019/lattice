import type { Component } from 'solid-js';

export interface StatusBarProps {
  message: string;
}

const StatusBar: Component<StatusBarProps> = (props) => {
  return (
    <div class="status-bar">
      <div class="status-bar-left">
        <span>{props.message}</span>
      </div>
      <div class="status-bar-right">
        <span>Lattice v0.1.0</span>
      </div>
    </div>
  );
};

export default StatusBar;
