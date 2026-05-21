import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

/**
 * Tests for the build-time backend selector.
 *
 * `IS_WASM_BACKEND` is evaluated at module load from `import.meta.env`, so
 * each test stubs the env var and re-imports the module fresh via
 * `vi.resetModules()` to exercise the chosen branch.
 */

// Mock the wasm-pack package — node has no DOM/WASM, so we stand in a fake.
const wasmInvoke = vi.fn((command: string, argsJson: string) => {
  if (command === 'boom') {
    throw new Error('engine failure');
  }
  return JSON.stringify({ echoed: command, args: JSON.parse(argsJson) });
});
const wasmInit = vi.fn();
const wasmDefault = vi.fn(async () => undefined);

vi.mock('lattice-wasm', () => ({
  default: wasmDefault,
  init: wasmInit,
  invoke: wasmInvoke,
  open_xlsx: vi.fn(),
  open_csv: vi.fn(),
  open_tsv: vi.fn(),
  save_xlsx: vi.fn(),
  export_csv_bytes: vi.fn(),
  export_tsv_bytes: vi.fn(),
}));

// Mock the Tauri core so the desktop branch is exercisable in node.
const tauriCoreInvoke = vi.fn(async (command: string) => ({ tauri: command }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: tauriCoreInvoke }));

beforeEach(() => {
  vi.resetModules();
  wasmInvoke.mockClear();
  wasmInit.mockClear();
  wasmDefault.mockClear();
  tauriCoreInvoke.mockClear();
});

afterEach(() => {
  vi.unstubAllEnvs();
});

describe('backend selector — WASM build', () => {
  beforeEach(() => {
    vi.stubEnv('VITE_LATTICE_BACKEND', 'wasm');
  });

  it('reports IS_WASM_BACKEND true', async () => {
    const mod = await import('./backend');
    expect(mod.IS_WASM_BACKEND).toBe(true);
  });

  it('loads the wasm module once: default loader then named init', async () => {
    const { invoke } = await import('./backend');
    await invoke('a');
    await invoke('b');
    // Loader + panic-hook init run exactly once, memoized.
    expect(wasmDefault).toHaveBeenCalledTimes(1);
    expect(wasmInit).toHaveBeenCalledTimes(1);
  });

  it('invoke serializes args to JSON and parses the result', async () => {
    const { invoke } = await import('./backend');
    const result = await invoke<{ echoed: string; args: unknown }>('ping', { x: 1 });
    expect(wasmInvoke).toHaveBeenCalledWith('ping', JSON.stringify({ x: 1 }));
    expect(result).toEqual({ echoed: 'ping', args: { x: 1 } });
  });

  it('invoke passes an empty object when args are omitted', async () => {
    const { invoke } = await import('./backend');
    await invoke('noargs');
    expect(wasmInvoke).toHaveBeenCalledWith('noargs', '{}');
  });

  it('a thrown wasm error becomes a rejected promise', async () => {
    const { invoke } = await import('./backend');
    await expect(invoke('boom')).rejects.toThrow('engine failure');
  });

  it('listen is a no-op returning an unlisten function', async () => {
    const { listen } = await import('./backend');
    const unlisten = await listen('menu-event', () => {
      throw new Error('handler should never fire on the wasm build');
    });
    expect(typeof unlisten).toBe('function');
    expect(() => unlisten()).not.toThrow();
  });

  it('getCurrentWindow stub maps setTitle onto document.title', async () => {
    const { getCurrentWindow } = await import('./backend');
    const fakeDoc = { title: '' };
    vi.stubGlobal('document', fakeDoc);
    await getCurrentWindow().setTitle('My Sheet — Lattice');
    expect(fakeDoc.title).toBe('My Sheet — Lattice');
    vi.unstubAllGlobals();
  });
});

describe('backend selector — Tauri build', () => {
  beforeEach(() => {
    vi.stubEnv('VITE_LATTICE_BACKEND', '');
  });

  it('reports IS_WASM_BACKEND false', async () => {
    const mod = await import('./backend');
    expect(mod.IS_WASM_BACKEND).toBe(false);
  });

  it('invoke delegates to the Tauri core invoke', async () => {
    const { invoke } = await import('./backend');
    const result = await invoke<{ tauri: string }>('list_sheets', {});
    expect(tauriCoreInvoke).toHaveBeenCalledWith('list_sheets', {});
    expect(result).toEqual({ tauri: 'list_sheets' });
    // The wasm module must never load on the desktop build.
    expect(wasmDefault).not.toHaveBeenCalled();
  });
});
