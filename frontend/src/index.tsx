/* @refresh reload */
import { render } from 'solid-js/web';
import { ErrorBoundary } from 'solid-js';
import App from './App';

// Catch unhandled errors that could cause a white screen.
window.addEventListener('error', (e) => {
  console.error('Uncaught error:', e.error ?? e.message);
});
window.addEventListener('unhandledrejection', (e) => {
  console.error('Unhandled promise rejection:', e.reason);
});

const root = document.getElementById('root');

render(
  () => (
    <ErrorBoundary
      fallback={(err) => (
        <div style={{
          padding: '32px',
          "font-family": '-apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif',
          color: '#c00',
        }}>
          <h2>Something went wrong</h2>
          <pre style={{ "white-space": 'pre-wrap', "word-break": 'break-word' }}>
            {err instanceof Error ? err.message : String(err)}
          </pre>
          <pre style={{ "font-size": '12px', color: '#888', "margin-top": '8px', "white-space": 'pre-wrap' }}>
            {err instanceof Error ? err.stack : ''}
          </pre>
          <button
            onClick={() => window.location.reload()}
            style={{ "margin-top": '16px', padding: '8px 16px', cursor: 'pointer' }}
          >
            Reload
          </button>
        </div>
      )}
    >
      <App />
    </ErrorBoundary>
  ),
  root!,
);
