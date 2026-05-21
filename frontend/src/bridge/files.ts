/**
 * File I/O abstraction — uniform open/save/export across both backends.
 *
 * Desktop (Tauri): native dialogs + path-based engine commands.
 * Web (WASM): File System Access API where available, with an
 * `<input type=file>` / Blob-download fallback for Safari & Firefox.
 *
 * Callers (App.tsx menu handlers) use the same functions regardless of
 * backend and never touch a filesystem path directly.
 */

import { IS_WASM_BACKEND, getWasmModule } from './backend';
import type { WorkbookInfo } from './tauri';
import {
  openFile,
  openCsv,
  openTsv,
  saveFile,
  exportCsv,
  exportTsv,
} from './tauri';

/** Result of opening a workbook through the file layer. */
export interface OpenResult {
  info: WorkbookInfo;
  /** A path (desktop) or a display name (web) — used for the title bar. */
  name: string;
}

/** Result of saving a workbook through the file layer. */
export interface SaveResult {
  /** A path (desktop) or display name (web), or null if the user cancelled. */
  name: string | null;
}

// Spreadsheet file filter shared by the desktop dialogs.
const FILE_FILTERS = [
  { name: 'Spreadsheet', extensions: ['xlsx', 'lattice', 'csv', 'tsv'] },
  { name: 'All Files', extensions: ['*'] },
];

// ---------------------------------------------------------------------------
// Extension dispatch helpers.
// ---------------------------------------------------------------------------

export type FileKind = 'csv' | 'tsv' | 'xlsx';

/**
 * Classify a file name by extension.
 *
 * `.csv` → csv, `.tsv`/`.tab` → tsv, everything else (`.xlsx`, `.lattice`,
 * extensionless) → xlsx. Exported for unit testing.
 */
export function kindOf(name: string): FileKind {
  const lower = name.toLowerCase();
  if (lower.endsWith('.csv')) return 'csv';
  if (lower.endsWith('.tsv') || lower.endsWith('.tab')) return 'tsv';
  return 'xlsx';
}

// ===========================================================================
// WASM (browser) file I/O
// ===========================================================================

/** Feature-detect the File System Access API (Chromium-only today). */
function hasFileSystemAccess(): boolean {
  return typeof (window as unknown as { showOpenFilePicker?: unknown }).showOpenFilePicker ===
    'function';
}

/** Read a `File` (or Blob) into a `Uint8Array`. */
async function fileToBytes(file: Blob): Promise<Uint8Array> {
  return new Uint8Array(await file.arrayBuffer());
}

/** Prompt for a file via a hidden `<input type=file>` (Safari/Firefox path). */
function pickFileViaInput(): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.xlsx,.lattice,.csv,.tsv,.tab';
    input.style.display = 'none';
    let settled = false;
    const done = (f: File | null) => {
      if (settled) return;
      settled = true;
      input.remove();
      resolve(f);
    };
    input.addEventListener('change', () => {
      done(input.files && input.files[0] ? input.files[0] : null);
    });
    // `cancel` fires on supporting browsers when the dialog is dismissed.
    input.addEventListener('cancel', () => done(null));
    document.body.appendChild(input);
    input.click();
  });
}

/** Open a workbook in the WASM build. */
async function wasmOpen(): Promise<OpenResult | null> {
  let file: File | null = null;

  if (hasFileSystemAccess()) {
    try {
      const picker = (window as unknown as {
        showOpenFilePicker: (opts: unknown) => Promise<Array<{ getFile(): Promise<File> }>>;
      }).showOpenFilePicker;
      const [handle] = await picker({
        types: [
          {
            description: 'Spreadsheet',
            accept: {
              'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet': ['.xlsx'],
              'text/csv': ['.csv'],
              'text/tab-separated-values': ['.tsv', '.tab'],
              'application/octet-stream': ['.lattice'],
            },
          },
        ],
        multiple: false,
      });
      if (!handle) return null;
      file = await handle.getFile();
    } catch (e) {
      // AbortError = user cancelled; treat as a clean no-op.
      if (e instanceof DOMException && e.name === 'AbortError') return null;
      throw e;
    }
  } else {
    file = await pickFileViaInput();
  }

  if (!file) return null;

  const bytes = await fileToBytes(file);
  const mod = await getWasmModule();
  const kind = kindOf(file.name);
  let infoJson: string;
  if (kind === 'csv') {
    infoJson = mod.open_csv(bytes);
  } else if (kind === 'tsv') {
    infoJson = mod.open_tsv(bytes);
  } else {
    infoJson = mod.open_xlsx(bytes);
  }
  return { info: JSON.parse(infoJson) as WorkbookInfo, name: file.name };
}

/** Trigger a browser download of `bytes` as `fileName`. */
function downloadBytes(bytes: Uint8Array, fileName: string, mime: string): void {
  const blob = new Blob([bytes as BlobPart], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = fileName;
  a.style.display = 'none';
  document.body.appendChild(a);
  a.click();
  a.remove();
  // Revoke after a tick so the download has a chance to start.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Write `bytes` to disk, preferring the File System Access API. */
async function writeBytesWeb(
  bytes: Uint8Array,
  suggestedName: string,
  mime: string,
  extension: string,
): Promise<string | null> {
  if (hasFileSystemAccess()) {
    try {
      const picker = (window as unknown as {
        showSaveFilePicker: (opts: unknown) => Promise<{
          createWritable(): Promise<{
            write(data: BufferSource | Blob): Promise<void>;
            close(): Promise<void>;
          }>;
          name: string;
        }>;
      }).showSaveFilePicker;
      const handle = await picker({
        suggestedName,
        types: [
          {
            description: 'Spreadsheet',
            accept: { [mime]: [extension] },
          },
        ],
      });
      const writable = await handle.createWritable();
      await writable.write(new Blob([bytes as BlobPart], { type: mime }));
      await writable.close();
      return handle.name;
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return null;
      throw e;
    }
  }
  // Fallback: plain download.
  downloadBytes(bytes, suggestedName, mime);
  return suggestedName;
}

const XLSX_MIME = 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';

/** Save the current workbook as `.xlsx` in the WASM build. */
async function wasmSave(suggestedName: string): Promise<SaveResult> {
  const mod = await getWasmModule();
  const bytes = mod.save_xlsx();
  const name = await writeBytesWeb(bytes, suggestedName, XLSX_MIME, '.xlsx');
  return { name };
}

/** Export a sheet as CSV in the WASM build. */
async function wasmExportCsv(sheet: string, suggestedName: string): Promise<SaveResult> {
  const mod = await getWasmModule();
  const bytes = mod.export_csv_bytes(sheet);
  const name = await writeBytesWeb(bytes, suggestedName, 'text/csv', '.csv');
  return { name };
}

/** Export a sheet as TSV in the WASM build. */
async function wasmExportTsv(sheet: string, suggestedName: string): Promise<SaveResult> {
  const mod = await getWasmModule();
  const bytes = mod.export_tsv_bytes(sheet);
  const name = await writeBytesWeb(bytes, suggestedName, 'text/tab-separated-values', '.tsv');
  return { name };
}

// ===========================================================================
// Tauri (desktop) file I/O — unchanged behavior, just routed through here.
// ===========================================================================

/** Open a workbook in the Tauri build via the native dialog. */
async function tauriOpen(): Promise<OpenResult | null> {
  const { open: dialogOpen } = await import('@tauri-apps/plugin-dialog');
  const selected = await dialogOpen({
    title: 'Open Spreadsheet',
    filters: FILE_FILTERS,
    multiple: false,
    directory: false,
  });
  if (!selected) return null;
  const path = typeof selected === 'string' ? selected : selected[0];
  if (!path) return null;
  const info = await openPath(path);
  return { info, name: path };
}

/** Open a file at a known path (desktop) — dispatches by extension. */
export async function openPath(path: string): Promise<WorkbookInfo> {
  const kind = kindOf(path);
  if (kind === 'csv') return openCsv(path);
  if (kind === 'tsv') return openTsv(path);
  return openFile(path);
}

/** Save a workbook in the Tauri build via the native dialog. */
async function tauriSave(): Promise<SaveResult> {
  const { save: dialogSave } = await import('@tauri-apps/plugin-dialog');
  const path = await dialogSave({ title: 'Save Spreadsheet', filters: FILE_FILTERS });
  if (!path) return { name: null };
  await saveFile(path);
  return { name: path };
}

/** Export a sheet as CSV in the Tauri build via the native dialog. */
async function tauriExportCsv(sheet: string): Promise<SaveResult> {
  const { save: dialogSave } = await import('@tauri-apps/plugin-dialog');
  const path = await dialogSave({
    title: 'Download as CSV',
    filters: [
      { name: 'CSV', extensions: ['csv'] },
      { name: 'All Files', extensions: ['*'] },
    ],
  });
  if (!path) return { name: null };
  await exportCsv(sheet, path);
  return { name: path };
}

/** Export a sheet as TSV in the Tauri build via the native dialog. */
async function tauriExportTsv(sheet: string): Promise<SaveResult> {
  const { save: dialogSave } = await import('@tauri-apps/plugin-dialog');
  const path = await dialogSave({
    title: 'Download as TSV',
    filters: [
      { name: 'TSV', extensions: ['tsv'] },
      { name: 'All Files', extensions: ['*'] },
    ],
  });
  if (!path) return { name: null };
  await exportTsv(sheet, path);
  return { name: path };
}

// ===========================================================================
// Public API — backend-agnostic.
// ===========================================================================

/** Open a workbook from a file. Returns null if the user cancelled. */
export function openWorkbook(): Promise<OpenResult | null> {
  return IS_WASM_BACKEND ? wasmOpen() : tauriOpen();
}

/** Save the current workbook. `suggestedName` is used by the web build. */
export function saveWorkbook(suggestedName = 'workbook.xlsx'): Promise<SaveResult> {
  return IS_WASM_BACKEND ? wasmSave(suggestedName) : tauriSave();
}

/** Export a sheet as CSV. */
export function exportSheetCsv(sheet: string, suggestedName?: string): Promise<SaveResult> {
  return IS_WASM_BACKEND
    ? wasmExportCsv(sheet, suggestedName ?? `${sheet || 'sheet'}.csv`)
    : tauriExportCsv(sheet);
}

/** Export a sheet as TSV. */
export function exportSheetTsv(sheet: string, suggestedName?: string): Promise<SaveResult> {
  return IS_WASM_BACKEND
    ? wasmExportTsv(sheet, suggestedName ?? `${sheet || 'sheet'}.tsv`)
    : tauriExportTsv(sheet);
}
