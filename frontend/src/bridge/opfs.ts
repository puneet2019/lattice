/**
 * OPFS autosave — Origin Private File System persistence for the web build.
 *
 * In the WASM build the workbook lives only in tab memory, so a refresh or
 * crash would lose unsaved work. This module debounces edits and writes the
 * serialized `.xlsx` bytes into OPFS, and on startup offers to restore them.
 *
 * No-op on the Tauri build (desktop already autosaves to disk).
 */

import { IS_WASM_BACKEND, getWasmModule } from './backend';

/** File name used inside the origin-private directory. */
const AUTOSAVE_FILE = 'autosave.xlsx';

/** Debounce window — write at most once this long after the last edit. */
const DEBOUNCE_MS = 3000;

/** Whether OPFS is usable in this environment. */
function hasOpfs(): boolean {
  return (
    IS_WASM_BACKEND &&
    typeof navigator !== 'undefined' &&
    !!navigator.storage &&
    typeof navigator.storage.getDirectory === 'function'
  );
}

/** Resolve the origin-private root directory. */
async function opfsRoot(): Promise<FileSystemDirectoryHandle> {
  return navigator.storage.getDirectory();
}

let pending: ReturnType<typeof setTimeout> | null = null;

/**
 * Schedule an autosave. Repeated calls within {@link DEBOUNCE_MS} collapse
 * into a single write {@link DEBOUNCE_MS} after the last call.
 */
export function scheduleAutosave(): void {
  if (!hasOpfs()) return;
  if (pending) clearTimeout(pending);
  pending = setTimeout(() => {
    pending = null;
    void writeAutosave();
  }, DEBOUNCE_MS);
}

/** Immediately serialize the workbook and write it to OPFS. */
export async function writeAutosave(): Promise<void> {
  if (!hasOpfs()) return;
  try {
    const mod = await getWasmModule();
    const bytes = mod.save_xlsx();
    const root = await opfsRoot();
    const handle = await root.getFileHandle(AUTOSAVE_FILE, { create: true });
    const writable = await handle.createWritable();
    await writable.write(new Blob([bytes as BlobPart]));
    await writable.close();
  } catch (e) {
    // Autosave is best-effort; never surface as a hard failure.
    console.warn('OPFS autosave failed:', e);
  }
}

/**
 * Read the autosave file if one exists.
 *
 * Returns the raw `.xlsx` bytes, or `null` if there is no autosave.
 */
export async function readAutosave(): Promise<Uint8Array | null> {
  if (!hasOpfs()) return null;
  try {
    const root = await opfsRoot();
    const handle = await root.getFileHandle(AUTOSAVE_FILE);
    const file = await handle.getFile();
    if (file.size === 0) return null;
    return new Uint8Array(await file.arrayBuffer());
  } catch {
    // NotFoundError when no autosave exists — a clean "nothing to restore".
    return null;
  }
}

/** Delete the autosave file (called after an explicit save or new workbook). */
export async function clearAutosave(): Promise<void> {
  if (!hasOpfs()) return;
  try {
    const root = await opfsRoot();
    await root.removeEntry(AUTOSAVE_FILE);
  } catch {
    // Already absent — nothing to do.
  }
}

/**
 * Restore a workbook from the OPFS autosave, if present.
 *
 * Loads the bytes back into the WASM engine via `open_xlsx` and returns the
 * resulting `WorkbookInfo`, or `null` when there is nothing to restore.
 */
export async function restoreAutosave(): Promise<{
  sheets: string[];
  active_sheet: string;
} | null> {
  const bytes = await readAutosave();
  if (!bytes) return null;
  try {
    const mod = await getWasmModule();
    const infoJson = mod.open_xlsx(bytes);
    return JSON.parse(infoJson) as { sheets: string[]; active_sheet: string };
  } catch (e) {
    console.warn('OPFS autosave restore failed:', e);
    return null;
  }
}
