/// <reference types="vite/client" />

/**
 * Build-time environment variables consumed by the frontend.
 *
 * `VITE_LATTICE_BACKEND` selects the command backend:
 *   - `'wasm'` → in-browser WebAssembly build
 *   - unset / anything else → Tauri desktop build
 */
interface ImportMetaEnv {
  readonly VITE_LATTICE_BACKEND?: 'wasm' | string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
