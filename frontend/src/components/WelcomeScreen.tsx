import type { Component } from 'solid-js';
import { For, Show } from 'solid-js';
import type { RecentFile } from '../bridge/tauri';

export interface WelcomeScreenProps {
  recentFiles: RecentFile[];
  onNewWorkbook: () => void;
  onOpenFile: () => void;
  onOpenRecent: (path: string) => void;
}

const WelcomeScreen: Component<WelcomeScreenProps> = (props) => {
  return (
    <div style={{
      display: 'flex',
      "flex-direction": 'column',
      "align-items": 'center',
      "justify-content": 'center',
      flex: '1',
      background: 'var(--cell-bg)',
      "min-height": '0',
    }}>
      {/* Logo */}
      <div style={{
        width: '80px',
        height: '80px',
        "margin-bottom": '20px',
        background: 'linear-gradient(135deg, #1a73e8 0%, #4285f4 50%, #8ab4f8 100%)',
        "border-radius": '18px',
        display: 'flex',
        "align-items": 'center',
        "justify-content": 'center',
        "box-shadow": '0 4px 16px rgba(26, 115, 232, 0.25)',
      }}>
        <svg width="44" height="44" viewBox="0 0 36 36" fill="none">
          <rect x="4" y="4" width="12" height="12" rx="1" fill="rgba(255,255,255,0.9)" />
          <rect x="20" y="4" width="12" height="12" rx="1" fill="rgba(255,255,255,0.6)" />
          <rect x="4" y="20" width="12" height="12" rx="1" fill="rgba(255,255,255,0.6)" />
          <rect x="20" y="20" width="12" height="12" rx="1" fill="rgba(255,255,255,0.4)" />
        </svg>
      </div>

      <div style={{
        "font-size": '28px',
        "font-weight": '700',
        color: 'var(--cell-text)',
        "margin-bottom": '6px',
      }}>
        Lattice
      </div>

      <div style={{
        "font-size": '14px',
        color: 'var(--header-text)',
        "margin-bottom": '32px',
      }}>
        AI-Native Spreadsheet for macOS
      </div>

      {/* Action buttons */}
      <div style={{
        display: 'flex',
        gap: '12px',
        "margin-bottom": '32px',
      }}>
        <button
          class="welcome-action-btn welcome-action-primary"
          onClick={() => props.onNewWorkbook()}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <line x1="8" y1="3" x2="8" y2="13" />
            <line x1="3" y1="8" x2="13" y2="8" />
          </svg>
          New blank spreadsheet
        </button>
        <button
          class="welcome-action-btn"
          onClick={() => props.onOpenFile()}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M2 13V5a1 1 0 011-1h4l2 2h4a1 1 0 011 1v6a1 1 0 01-1 1H3a1 1 0 01-1-1z" />
          </svg>
          Open file...
        </button>
      </div>

      {/* Recent files */}
      <Show when={props.recentFiles.length > 0}>
        <div style={{
          width: '360px',
          "max-height": '240px',
          "overflow-y": 'auto',
        }}>
          <div style={{
            "font-size": '11px',
            "font-weight": '600',
            color: 'var(--header-text)',
            "text-transform": 'uppercase',
            "letter-spacing": '0.5px',
            "margin-bottom": '8px',
            "padding-left": '12px',
          }}>
            Recent files
          </div>
          <For each={props.recentFiles}>
            {(file) => {
              const fileName = file.name || file.path.split('/').pop() || file.path;
              const dirPath = file.path.substring(0, file.path.lastIndexOf('/'));
              return (
                <button
                  class="welcome-recent-item"
                  onClick={() => props.onOpenRecent(file.path)}
                  title={file.path}
                >
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="var(--header-text)" stroke-width="1.2">
                    <rect x="3" y="1" width="10" height="14" rx="1" />
                    <line x1="5" y1="4" x2="11" y2="4" />
                    <line x1="5" y1="6.5" x2="11" y2="6.5" />
                    <line x1="5" y1="9" x2="9" y2="9" />
                  </svg>
                  <div style={{ flex: '1', "min-width": '0' }}>
                    <div style={{
                      "font-size": '13px',
                      "font-weight": '500',
                      color: 'var(--cell-text)',
                      overflow: 'hidden',
                      "text-overflow": 'ellipsis',
                      "white-space": 'nowrap',
                    }}>
                      {fileName}
                    </div>
                    <div style={{
                      "font-size": '11px',
                      color: 'var(--header-text)',
                      overflow: 'hidden',
                      "text-overflow": 'ellipsis',
                      "white-space": 'nowrap',
                    }}>
                      {dirPath}
                    </div>
                  </div>
                </button>
              );
            }}
          </For>
        </div>
      </Show>
    </div>
  );
};

export default WelcomeScreen;
