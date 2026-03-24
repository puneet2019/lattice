import type { Component } from 'solid-js';

export interface AboutDialogProps {
  onClose: () => void;
}

const AboutDialog: Component<AboutDialogProps> = (props) => {
  const handleBackdropClick = (e: MouseEvent) => {
    if ((e.target as HTMLElement).classList.contains('paste-special-backdrop')) {
      props.onClose();
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      props.onClose();
    }
  };

  return (
    <div
      class="paste-special-backdrop"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
    >
      <div class="paste-special-dialog" style={{ width: '340px', "text-align": 'center' }}>
        <div class="paste-special-header" style={{ "justify-content": 'flex-end', "border-bottom": 'none' }}>
          <button
            class="chart-overlay-close"
            onClick={() => props.onClose()}
            title="Close"
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

        <div class="paste-special-body" style={{ "padding-top": '0', "padding-bottom": '24px' }}>
          {/* Logo */}
          <div style={{
            width: '64px',
            height: '64px',
            margin: '0 auto 16px',
            background: 'linear-gradient(135deg, #1a73e8 0%, #4285f4 50%, #8ab4f8 100%)',
            "border-radius": '14px',
            display: 'flex',
            "align-items": 'center',
            "justify-content": 'center',
          }}>
            <svg width="36" height="36" viewBox="0 0 36 36" fill="none">
              {/* Grid icon representing a spreadsheet */}
              <rect x="4" y="4" width="12" height="12" rx="1" fill="rgba(255,255,255,0.9)" />
              <rect x="20" y="4" width="12" height="12" rx="1" fill="rgba(255,255,255,0.6)" />
              <rect x="4" y="20" width="12" height="12" rx="1" fill="rgba(255,255,255,0.6)" />
              <rect x="20" y="20" width="12" height="12" rx="1" fill="rgba(255,255,255,0.4)" />
            </svg>
          </div>

          <div style={{
            "font-size": '20px',
            "font-weight": '700',
            color: 'var(--cell-text)',
            "margin-bottom": '4px',
          }}>
            Lattice
          </div>

          <div style={{
            "font-size": '13px',
            color: 'var(--header-text)',
            "margin-bottom": '12px',
          }}>
            Version 0.1.0
          </div>

          <div style={{
            "font-size": '13px',
            color: 'var(--cell-text)',
            "margin-bottom": '4px',
          }}>
            AI-Native Spreadsheet for macOS
          </div>

          <div style={{
            "font-size": '12px',
            color: 'var(--header-text)',
            "margin-bottom": '16px',
          }}>
            Built with Rust + Tauri + SolidJS
          </div>

          <a
            href="https://github.com/nicholasgasior/lattice"
            target="_blank"
            rel="noopener noreferrer"
            style={{
              "font-size": '12px',
              color: 'var(--selection-border)',
              "text-decoration": 'none',
            }}
          >
            View on GitHub
          </a>
        </div>

        <div class="paste-special-footer">
          <button
            class="chart-dialog-btn chart-dialog-btn-primary"
            onClick={() => props.onClose()}
          >
            OK
          </button>
        </div>
      </div>
    </div>
  );
};

export default AboutDialog;
