/**
 * Backend selector — picks the command/IPC backend at build time.
 *
 * The frontend talks to the engine through a single chokepoint: `invoke()`.
 * On the desktop build that is Tauri IPC; on the web build it is a direct
 * synchronous call into the `lattice-wasm` module. Both expose the *same*
 * async `invoke(command, args?)` signature so the ~95 typed wrappers in
 * `tauri.ts` never need to know which backend they run on.
 *
 * The build-time switch is `import.meta.env.VITE_LATTICE_BACKEND`:
 *   - `'wasm'`  → in-browser WebAssembly build
 *   - anything else (default) → Tauri desktop build
 */

/** Whether this bundle targets the in-browser WASM backend. */
export const IS_WASM_BACKEND = import.meta.env.VITE_LATTICE_BACKEND === 'wasm';

// ---------------------------------------------------------------------------
// Window stub — the web app has no native window.
// ---------------------------------------------------------------------------

/** Minimal subset of the Tauri window API the frontend actually uses. */
export interface AppWindow {
  setTitle(title: string): Promise<void>;
}

/** A no-op window for the web build (document.title is set separately). */
const STUB_WINDOW: AppWindow = {
  async setTitle(title: string): Promise<void> {
    // The browser tab title is the closest analogue to a native title.
    if (typeof document !== 'undefined') {
      document.title = title;
    }
  },
};

// ---------------------------------------------------------------------------
// Event listener type — matches @tauri-apps/api/event `listen`.
// ---------------------------------------------------------------------------

/** Event payload shape passed to `listen` handlers. */
export interface BackendEvent<T> {
  event: string;
  payload: T;
}

/** Unsubscribe handle returned by `listen`. */
export type UnlistenFn = () => void;

// ===========================================================================
// WASM backend
// ===========================================================================

/**
 * Lazily loads the `lattice-wasm` module exactly once.
 *
 * wasm-pack's *default* export is the async loader (it fetches and
 * instantiates `lattice_wasm_bg.wasm`); the *named* `init` installs the
 * Rust panic hook. Both must run once, default-export first.
 */
let wasmReady: Promise<{
  invoke: (command: string, argsJson: string) => string;
  open_xlsx: (bytes: Uint8Array) => string;
  open_csv: (bytes: Uint8Array) => string;
  open_tsv: (bytes: Uint8Array) => string;
  save_xlsx: () => Uint8Array;
  export_csv_bytes: (sheet: string) => Uint8Array;
  export_tsv_bytes: (sheet: string) => Uint8Array;
  /** MCP JSON-RPC 2.0 entry point used by `frontend/src/mcp/server.ts`. */
  mcp_request: (request: string) => string | undefined;
}> | null = null;

/** Resolve (and memoize) the initialized WASM module. */
async function loadWasm() {
  if (!wasmReady) {
    wasmReady = (async () => {
      const mod = await import('lattice-wasm');
      // Default export = async loader; must run before any named export.
      await mod.default();
      // Named `init` = install panic hook (idempotent).
      mod.init();
      return mod;
    })();
  }
  return wasmReady;
}

/** Eagerly trigger WASM loading (optional — speeds up first invoke). */
export function preloadBackend(): void {
  if (IS_WASM_BACKEND) {
    void loadWasm();
  }
}

/**
 * Get the raw WASM module — used by the file I/O layer for the byte-buffer
 * file commands (`open_xlsx`, `save_xlsx`, …) that bypass `invoke`.
 */
export async function getWasmModule() {
  if (!IS_WASM_BACKEND) {
    throw new Error('getWasmModule() is only available in the WASM build');
  }
  return loadWasm();
}

/** WASM `invoke`: sync wasm call wrapped so it returns a Promise. */
async function wasmInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const mod = await loadWasm();
  // `mod.invoke` is synchronous and throws a JsError on failure; awaiting
  // inside an async function turns a throw into a rejected promise.
  const resultJson = mod.invoke(command, JSON.stringify(args ?? {}));
  return JSON.parse(resultJson) as T;
}

// ===========================================================================
// Tauri backend
// ===========================================================================

/** Tauri `invoke`: delegates straight to `@tauri-apps/api/core`. */
async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

/** Tauri `listen`: delegates to `@tauri-apps/api/event`. */
async function tauriListen<T>(
  event: string,
  handler: (e: BackendEvent<T>) => void,
): Promise<UnlistenFn> {
  const { listen } = await import('@tauri-apps/api/event');
  return listen<T>(event, handler);
}

/** Tauri window: delegates to `@tauri-apps/api/window`. */
async function tauriWindow(): Promise<AppWindow> {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  return getCurrentWindow();
}

// ===========================================================================
// Public API — uniform across both backends.
// ===========================================================================

/**
 * Dispatch a backend command.
 *
 * Same signature on both backends: async, args is a plain object with
 * camelCase keys, resolves with the parsed result, rejects on error.
 */
export function invoke<T = unknown>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return IS_WASM_BACKEND ? wasmInvoke<T>(command, args) : tauriInvoke<T>(command, args);
}

/**
 * Subscribe to a backend event.
 *
 * On the WASM build there is no event bus — this is a no-op that returns
 * an unlisten function so callers can use it uniformly.
 */
export function listen<T>(
  event: string,
  handler: (e: BackendEvent<T>) => void,
): Promise<UnlistenFn> {
  if (IS_WASM_BACKEND) {
    // No native event bus in the browser; nothing to subscribe to.
    void event;
    void handler;
    return Promise.resolve(() => {});
  }
  return tauriListen<T>(event, handler);
}

/**
 * Get the current application window.
 *
 * On the WASM build this is a stub that maps `setTitle` onto `document.title`.
 */
export function getCurrentWindow(): AppWindow {
  if (IS_WASM_BACKEND) {
    return STUB_WINDOW;
  }
  // The Tauri window API is async to import; return a thin async-backed proxy
  // so callers keep the synchronous `getCurrentWindow().setTitle(...)` shape.
  return {
    async setTitle(title: string): Promise<void> {
      const win = await tauriWindow();
      await win.setTitle(title);
    },
  };
}
