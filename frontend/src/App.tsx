import type { Component } from 'solid-js';
import { createSignal, onMount, onCleanup, Show } from 'solid-js';
import { listen } from '@tauri-apps/api/event';
import { open as dialogOpen, save as dialogSave } from '@tauri-apps/plugin-dialog';
import Toolbar from './components/Toolbar';
import FormulaBar from './components/FormulaBar';
import FindBar from './components/FindBar';
import VirtualGrid from './components/Grid/VirtualGrid';
import SheetTabs from './components/SheetTabs';
import StatusBar from './components/StatusBar';
import type { SaveStatus } from './components/StatusBar';
import ChartContainer from './components/Charts/ChartContainer';
import type { ChartOverlay } from './components/Charts/ChartContainer';
import ChartDialog from './components/Charts/ChartDialog';
import PasteSpecialDialog from './components/PasteSpecialDialog';
import type { PasteMode } from './components/PasteSpecialDialog';
import FormatCellsDialog from './components/FormatCellsDialog';
import DataValidationDialog from './components/DataValidationDialog';
import FilterDropdown from './components/FilterDropdown';
import ConditionalFormatDialog from './components/ConditionalFormatDialog';
import SortDialog from './components/SortDialog';
import NamedRangesDialog from './components/NamedRangesDialog';
import KeyboardShortcutsDialog from './components/KeyboardShortcutsDialog';
import PrintPreviewDialog from './components/PrintPreviewDialog';
import DataCleanupDialog from './components/DataCleanupDialog';
import TextToColumnsDialog from './components/TextToColumnsDialog';
import AboutDialog from './components/AboutDialog';
import FilterViewDropdown from './components/FilterViewDropdown';
import NamedFunctionsDialog from './components/NamedFunctionsDialog';
import PivotDialog from './components/PivotDialog';
import ColumnStatsPanel from './components/ColumnStatsPanel';
import SlicerContainer from './components/SlicerWidget';
import type { SlicerState } from './components/SlicerWidget';
import AddSlicerDialog from './components/AddSlicerDialog';
import WelcomeScreen from './components/WelcomeScreen';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  listSheets,
  addSheet,
  setActiveSheet,
  setCell,
  getCell,
  renameSheet,
  deleteSheet,
  duplicateSheet,
  formatCells,
  openFile,
  openCsv,
  openTsv,
  saveFile,
  exportCsv,
  exportTsv,
  exportHtml,
  newWorkbook,
  undo as tauriUndo,
  redo as tauriRedo,
  listCharts,
  getChartConfig,
  renderChartSvg,
  setSheetTabColor,
  moveSheet,
  setAutoFilter,
  clearFilter,
  listNamedRanges,
  insertRows,
  insertCols,
  sortRange,
  getRecentFiles,
  addRecentFile,
  mergeCells,
  unmergeCells,
  getMergedRegions,
  setBandedRows,
  getBandedRows,
  textToColumns,
} from './bridge/tauri';
import type { FilterInfo, NamedRangeInfo, RecentFile, MergedRegionData, BandedRowsData, BordersUpdate } from './bridge/tauri';
import type { ChartInfo, ChartTypeStr } from './bridge/tauri';
import { parse_cell_ref, col_to_letter } from './bridge/tauri_helpers';
import { TOTAL_ROWS, TOTAL_COLS } from './components/Grid/constants';
import './styles/grid.css';

/** Convert 0-based row, col to A1-style reference. */
function cellRefStr(row: number, col: number): string {
  let result = '';
  let c = col;
  do {
    result = String.fromCharCode(65 + (c % 26)) + result;
    c = Math.floor(c / 26) - 1;
  } while (c >= 0);
  return `${result}${row + 1}`;
}

/** Parse column letters (e.g., "A", "AA") to 0-based column index. */
function parseColLetters(letters: string): number {
  const upper = letters.toUpperCase();
  let col = 0;
  for (let i = 0; i < upper.length; i++) {
    col = col * 26 + (upper.charCodeAt(i) - 64);
  }
  return col - 1;
}

/** Compute name box display string from selection range. */
function nameBoxDisplayStr(
  row: number, col: number,
  minRow: number, minCol: number,
  maxRow: number, maxCol: number,
): string {
  // Single cell
  if (minRow === maxRow && minCol === maxCol) {
    return cellRefStr(row, col);
  }
  // All cells selected
  if (minRow === 0 && minCol === 0 && maxRow >= TOTAL_ROWS - 1 && maxCol >= TOTAL_COLS - 1) {
    return 'All';
  }
  // Full column(s): all rows selected
  if (minRow === 0 && maxRow >= TOTAL_ROWS - 1) {
    const startLetter = col_to_letter(minCol);
    const endLetter = col_to_letter(maxCol);
    return minCol === maxCol ? `${startLetter}:${startLetter}` : `${startLetter}:${endLetter}`;
  }
  // Full row(s): all columns selected
  if (minCol === 0 && maxCol >= TOTAL_COLS - 1) {
    return minRow === maxRow ? `${minRow + 1}:${minRow + 1}` : `${minRow + 1}:${maxRow + 1}`;
  }
  // Range
  return `${cellRefStr(minRow, minCol)}:${cellRefStr(maxRow, maxCol)}`;
}

const App: Component = () => {
  const [sheets, setSheets] = createSignal<string[]>(['Sheet1']);
  const [activeSheetName, setActiveSheetLocal] = createSignal('Sheet1');
  const [selectedCell, setSelectedCell] = createSignal<[number, number]>([0, 0]);
  // Track the full selection range (minRow, minCol, maxRow, maxCol) for multi-cell operations.
  const [selRange, setSelRange] = createSignal<[number, number, number, number]>([0, 0, 0, 0]);
  const [formulaContent, setFormulaContent] = createSignal('');
  const [nameBoxDisplay, setNameBoxDisplay] = createSignal('A1');
  // Navigation request signal: [row, col, anchorRow, anchorCol, endRow, endCol]
  const [navigateTo, setNavigateTo] = createSignal<[number, number, number, number, number, number] | null>(null);
  const [statusMessage, setStatusMessage] = createSignal('Ready');
  const [mode, setMode] = createSignal<'Ready' | 'Edit'>('Ready');
  const [selectionSummary, setSelectionSummary] = createSignal('');
  const [zoom, setZoom] = createSignal(1.0);
  const [refreshTrigger, setRefreshTrigger] = createSignal(0);
  const [boldActive, setBoldActive] = createSignal(false);
  const [italicActive, setItalicActive] = createSignal(false);
  const [underlineActive, setUnderlineActive] = createSignal(false);
  const [strikethroughActive, setStrikethroughActive] = createSignal(false);
  const [paintFormatActive, setPaintFormatActive] = createSignal(false);
  const [paintFormatData, setPaintFormatData] = createSignal<Record<string, unknown> | null>(null);
  const [currentFontFamily, setCurrentFontFamily] = createSignal('Arial');
  const [currentFontSize, setCurrentFontSize] = createSignal(11);
  const [currentHAlign, setCurrentHAlign] = createSignal('left');
  const [currentVAlign, setCurrentVAlign] = createSignal('bottom');
  const [currentFilePath, setCurrentFilePath] = createSignal<string | null>(null);
  const [tabColors, setTabColors] = createSignal<Record<string, string>>({});
  const [frozenRows, setFrozenRows] = createSignal(0);
  const [frozenCols, setFrozenCols] = createSignal(0);
  const [splitRow, setSplitRow] = createSignal(0);
  const [splitCol, setSplitCol] = createSignal(0);
  const [filterActive, setFilterActive] = createSignal(false);
  const [filterInfo, setFilterInfo] = createSignal<FilterInfo | null>(null);
  const [showFilterDropdown, setShowFilterDropdown] = createSignal(false);
  const [filterDropdownCol, setFilterDropdownCol] = createSignal(0);
  const [filterDropdownX, setFilterDropdownX] = createSignal(0);
  const [filterDropdownY, setFilterDropdownY] = createSignal(0);

  const [isDirty, setIsDirty] = createSignal(false);
  const [saveStatus, setSaveStatus] = createSignal<SaveStatus>('saved');
  const [showGridlines, setShowGridlines] = createSignal(true);
  const [showFormulas, setShowFormulas] = createSignal(false);
  const [recentFiles, setRecentFilesState] = createSignal<RecentFile[]>([]);
  const [workbookLoaded, setWorkbookLoaded] = createSignal(false);

  /** Mark the workbook as having unsaved changes. */
  function markDirty() {
    if (!isDirty()) {
      setIsDirty(true);
      updateWindowTitle(currentFilePath(), true);
    }
    setSaveStatus('unsaved');
  }

  // Spreadsheet file filter for open/save dialogs.
  const fileFilters = [
    { name: 'Spreadsheet', extensions: ['xlsx', 'lattice', 'csv', 'tsv'] },
    { name: 'All Files', extensions: ['*'] },
  ];

  // -------------------------------------------------------------------
  // File operations (triggered by menu events)
  // -------------------------------------------------------------------

  /** Update the Tauri window title based on current file and dirty state. */
  function updateWindowTitle(filePath: string | null, dirty: boolean) {
    let name = 'Untitled';
    if (filePath) {
      const parts = filePath.split('/');
      name = parts[parts.length - 1] || filePath;
    }
    const title = dirty ? `${name}* \u2014 Lattice` : `${name} \u2014 Lattice`;
    try {
      void getCurrentWindow().setTitle(title);
    } catch {
      // Tauri not available in browser dev mode
    }
  }

  /** Apply workbook info from open/new result to local state. */
  function applyWorkbookInfo(info: { sheets: string[]; active_sheet: string }) {
    setSheets(info.sheets);
    setActiveSheetLocal(info.active_sheet);
    setRefreshTrigger((n) => n + 1);
    setSelectedCell([0, 0]);
    setSelRange([0, 0, 0, 0]);
    setFormulaContent('');
    setIsDirty(false);
    setWorkbookLoaded(true);
  }

  const handleFileNew = async () => {
    try {
      const info = await newWorkbook();
      applyWorkbookInfo(info);
      setCurrentFilePath(null);
      updateWindowTitle(null, false);
      setSaveStatus('saved');
      setStatusMessage('New workbook created');
    } catch (e) {
      setStatusMessage(`New workbook failed: ${e}`);
    }
  };

  const handleFileOpen = async () => {
    try {
      const selected = await dialogOpen({
        title: 'Open Spreadsheet',
        filters: fileFilters,
        multiple: false,
        directory: false,
      });
      if (!selected) return; // user cancelled
      const path = typeof selected === 'string' ? selected : selected[0];
      if (!path) return;
      // Determine format by extension and call the right open command
      const lower = path.toLowerCase();
      let info;
      if (lower.endsWith('.csv')) {
        info = await openCsv(path);
      } else if (lower.endsWith('.tsv') || lower.endsWith('.tab')) {
        info = await openTsv(path);
      } else {
        info = await openFile(path);
      }
      applyWorkbookInfo(info);
      setCurrentFilePath(path);
      updateWindowTitle(path, false);
      setSaveStatus('saved');
      // Track in recent files
      const fileName = path.split('/').pop() || path;
      void addRecentFile(path, fileName).catch(() => {});
      setStatusMessage(`Opened: ${path}`);
    } catch (e) {
      setStatusMessage(`Open failed: ${e}`);
    }
  };

  const handleOpenRecentFile = async (path: string) => {
    try {
      const lower = path.toLowerCase();
      let info;
      if (lower.endsWith('.csv')) {
        info = await openCsv(path);
      } else if (lower.endsWith('.tsv') || lower.endsWith('.tab')) {
        info = await openTsv(path);
      } else {
        info = await openFile(path);
      }
      applyWorkbookInfo(info);
      setCurrentFilePath(path);
      updateWindowTitle(path, false);
      setSaveStatus('saved');
      const fileName = path.split('/').pop() || path;
      void addRecentFile(path, fileName).catch(() => {});
      setStatusMessage(`Opened: ${path}`);
    } catch (e) {
      setStatusMessage(`Open failed: ${e}`);
    }
  };

  const handleFileSave = async () => {
    const path = currentFilePath();
    if (!path) {
      // No path yet — fall through to Save As.
      await handleFileSaveAs();
      return;
    }
    try {
      setSaveStatus('saving');
      await saveFile(path);
      setIsDirty(false);
      updateWindowTitle(path, false);
      setSaveStatus('saved');
      setStatusMessage(`Saved: ${path}`);
    } catch (e) {
      setSaveStatus('unsaved');
      setStatusMessage(`Save failed: ${e}`);
    }
  };

  const handleFileSaveAs = async () => {
    try {
      const path = await dialogSave({
        title: 'Save Spreadsheet',
        filters: fileFilters,
      });
      if (!path) return; // user cancelled
      setSaveStatus('saving');
      await saveFile(path);
      setCurrentFilePath(path);
      setIsDirty(false);
      updateWindowTitle(path, false);
      setSaveStatus('saved');
      setStatusMessage(`Saved: ${path}`);
    } catch (e) {
      setSaveStatus('unsaved');
      setStatusMessage(`Save As failed: ${e}`);
    }
  };

  const handleExportCsv = async () => {
    try {
      const path = await dialogSave({
        title: 'Download as CSV',
        filters: [
          { name: 'CSV', extensions: ['csv'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });
      if (!path) return;
      await exportCsv(activeSheetName(), path);
      setStatusMessage(`Exported CSV: ${path}`);
    } catch (e) {
      setStatusMessage(`CSV export failed: ${e}`);
    }
  };

  const handleExportTsv = async () => {
    try {
      const path = await dialogSave({
        title: 'Download as TSV',
        filters: [
          { name: 'TSV', extensions: ['tsv'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });
      if (!path) return;
      await exportTsv(activeSheetName(), path);
      setStatusMessage(`Exported TSV: ${path}`);
    } catch (e) {
      setStatusMessage(`TSV export failed: ${e}`);
    }
  };

  const handleExportPdf = async () => {
    try {
      const html = await exportHtml(activeSheetName());
      // Write HTML to a temp file and open in browser for print-to-PDF
      const blob = new Blob([html], { type: 'text/html' });
      const url = URL.createObjectURL(blob);
      window.open(url, '_blank');
      setStatusMessage('PDF export: print from browser window');
    } catch (e) {
      setStatusMessage(`PDF export failed: ${e}`);
    }
  };

  // -------------------------------------------------------------------
  // Menu event listener
  // -------------------------------------------------------------------

  const menuActions: Record<string, () => void> = {
    // -- File ---------------------------------------------------------------
    file_new: handleFileNew,
    file_open: handleFileOpen,
    file_save: handleFileSave,
    file_save_as: handleFileSaveAs,
    file_export_csv: handleExportCsv,
    file_export_tsv: handleExportTsv,
    file_export_pdf: handleExportPdf,
    file_print: () => { setShowPrintPreview(true); },

    // -- Edit ---------------------------------------------------------------
    edit_paste_special: () => { setShowPasteSpecial(true); },
    edit_find: () => { setFindBarReplace(false); setShowFindBar(true); },
    edit_find_replace: () => { setFindBarReplace(true); setShowFindBar(true); },
    edit_select_all: () => {
      // Select all cells — set selection range to full grid
      setSelRange([0, 0, TOTAL_ROWS - 1, TOTAL_COLS - 1]);
      setNameBoxDisplay('All');
      setStatusMessage('All cells selected');
    },
    edit_undo: () => {
      void tauriUndo()
        .then(() => {
          setRefreshTrigger((n) => n + 1);
          setStatusMessage('Undo');
        })
        .catch(() => setStatusMessage('Nothing to undo'));
    },
    edit_redo: () => {
      void tauriRedo()
        .then(() => {
          setRefreshTrigger((n) => n + 1);
          setStatusMessage('Redo');
        })
        .catch(() => setStatusMessage('Nothing to redo'));
    },

    // -- View > Freeze ------------------------------------------------------
    view_freeze_1row: () => { setFrozenRows(1); setFrozenCols(0); setStatusMessage('Frozen: 1 row'); },
    view_freeze_2rows: () => { setFrozenRows(2); setFrozenCols(0); setStatusMessage('Frozen: 2 rows'); },
    view_freeze_1col: () => { setFrozenRows(0); setFrozenCols(1); setStatusMessage('Frozen: 1 column'); },
    view_freeze_2cols: () => { setFrozenRows(0); setFrozenCols(2); setStatusMessage('Frozen: 2 columns'); },
    view_freeze_none: () => { setFrozenRows(0); setFrozenCols(0); setStatusMessage('Freeze panes removed'); },

    // -- View > Show / Zoom -------------------------------------------------
    view_show_formulas: () => {
      setShowFormulas(!showFormulas());
      setRefreshTrigger((n) => n + 1);
      setStatusMessage(showFormulas() ? 'Formula view on' : 'Formula view off');
    },
    view_toggle_gridlines: () => {
      setShowGridlines(!showGridlines());
      setStatusMessage(showGridlines() ? 'Gridlines shown' : 'Gridlines hidden');
    },
    view_zoom_in: handleZoomIn,
    view_zoom_out: handleZoomOut,
    view_zoom_reset: handleZoomReset,
    view_page_break_preview: () => {
      setPageBreakPreview(!pageBreakPreview());
      setStatusMessage(pageBreakPreview() ? 'Page break preview enabled' : 'Page break preview disabled');
    },

    // -- Insert -------------------------------------------------------------
    insert_row_above: () => {
      const [row] = selectedCell();
      void insertRows(activeSheetName(), row, 1)
        .then(() => { setRefreshTrigger((n) => n + 1); markDirty(); setStatusMessage('Row inserted above'); })
        .catch((e) => setStatusMessage(`Insert row failed: ${e}`));
    },
    insert_row_below: () => {
      const [row] = selectedCell();
      void insertRows(activeSheetName(), row + 1, 1)
        .then(() => { setRefreshTrigger((n) => n + 1); markDirty(); setStatusMessage('Row inserted below'); })
        .catch((e) => setStatusMessage(`Insert row failed: ${e}`));
    },
    insert_col_left: () => {
      const [, col] = selectedCell();
      void insertCols(activeSheetName(), col, 1)
        .then(() => { setRefreshTrigger((n) => n + 1); markDirty(); setStatusMessage('Column inserted left'); })
        .catch((e) => setStatusMessage(`Insert column failed: ${e}`));
    },
    insert_col_right: () => {
      const [, col] = selectedCell();
      void insertCols(activeSheetName(), col + 1, 1)
        .then(() => { setRefreshTrigger((n) => n + 1); markDirty(); setStatusMessage('Column inserted right'); })
        .catch((e) => setStatusMessage(`Insert column failed: ${e}`));
    },
    insert_chart: handleInsertChart,
    insert_note: () => {
      // Dispatch a custom event so VirtualGrid can open the inline comment
      // editor at the selected cell (same as Shift+F2 / context menu > Insert note).
      window.dispatchEvent(new CustomEvent('lattice:insert-note'));
    },
    insert_checkbox: () => {
      const [row, col] = selectedCell();
      void setCell(activeSheetName(), row, col, 'FALSE')
        .then(() => {
          setRefreshTrigger((n) => n + 1);
          markDirty();
          setStatusMessage(`Checkbox inserted at ${cellRefStr(row, col)}`);
        })
        .catch((e) => setStatusMessage(`Insert checkbox failed: ${e}`));
    },
    insert_named_range: () => { setShowNamedRanges(true); },
    insert_named_function: () => { setShowNamedFunctions(true); },

    // -- Format > Number ----------------------------------------------------
    format_num_general: () => { handleNumberFormat('General'); },
    format_num_number: () => { handleNumberFormat('#,##0.00'); },
    format_num_currency: () => { handleNumberFormat('$#,##0.00'); },
    format_num_percentage: () => { handleNumberFormat('0.00%'); },
    format_num_date: () => { handleNumberFormat('yyyy-mm-dd'); },
    format_num_time: () => { handleNumberFormat('hh:mm:ss'); },
    format_num_scientific: () => { handleNumberFormat('0.00E+00'); },

    // -- Format > Text styling ----------------------------------------------
    format_bold: handleBold,
    format_italic: handleItalic,
    format_underline: handleUnderline,
    format_strikethrough: () => {
      void applyFormat({ strikethrough: true });
      setStatusMessage('Strikethrough applied');
    },

    // -- Format > Font size -------------------------------------------------
    format_size_increase: () => { handleFontSize(14); },
    format_size_decrease: () => { handleFontSize(10); },

    // -- Format > Colors & alignment ----------------------------------------
    format_text_color: () => { setStatusMessage('Text color (use toolbar color picker)'); },
    format_fill_color: () => { setStatusMessage('Fill color (use toolbar color picker)'); },
    format_align_left: () => { handleAlign('left'); },
    format_align_center: () => { handleAlign('center'); },
    format_align_right: () => { handleAlign('right'); },

    // -- Format > Merge, conditional, clear ---------------------------------
    format_merge: handleMerge,
    format_conditional: () => { setShowConditionalFormat(true); },
    format_alternating: handleAlternatingColors,
    format_clear: () => {
      void applyFormat({
        bold: false, italic: false, underline: false, strikethrough: false,
        font_color: '', bg_color: '', number_format: 'General',
        h_align: 'left', font_size: 11, font_family: 'Arial',
      });
      setBoldActive(false);
      setItalicActive(false);
      setUnderlineActive(false);
      setStatusMessage('Formatting cleared');
    },

    // -- Data > Sort --------------------------------------------------------
    data_sort_az: () => {
      const [, col] = selectedCell();
      void sortRange(activeSheetName(), null, [{ col, direction: 'asc' }])
        .then(() => { setRefreshTrigger((n) => n + 1); setStatusMessage('Sorted A \u2192 Z'); })
        .catch((e) => setStatusMessage(`Sort failed: ${e}`));
    },
    data_sort_za: () => {
      const [, col] = selectedCell();
      void sortRange(activeSheetName(), null, [{ col, direction: 'desc' }])
        .then(() => { setRefreshTrigger((n) => n + 1); setStatusMessage('Sorted Z \u2192 A'); })
        .catch((e) => setStatusMessage(`Sort failed: ${e}`));
    },
    data_sort_custom: () => { setShowSortDialog(true); },

    // -- Data > Filter, validation, etc. ------------------------------------
    data_create_filter: handleFilterToggle,
    data_filter_views: () => { setShowFilterViews(true); },
    data_named_ranges: () => { setShowNamedRanges(true); },
    data_validation: () => { setShowDataValidation(true); },
    data_remove_duplicates: () => { setShowDataCleanup(true); },
    data_text_to_columns: () => { setShowTextToColumns(true); },
    data_pivot_table: () => { setShowPivotDialog(true); },
    data_column_stats: () => {
      setColumnStatsCol(selectedCell()[1]);
      setShowColumnStats(true);
    },
    data_add_slicer: () => { setShowAddSlicer(true); },

    // -- Help ---------------------------------------------------------------
    help_shortcuts: () => { setShowKeyboardShortcuts(true); },
    help_about: () => { setShowAbout(true); },
  };

  // Load sheets on mount and subscribe to menu events.
  onMount(async () => {
    try {
      const sheetList = await listSheets();
      setSheets(sheetList.map((s) => s.name));
      const active = sheetList.find((s) => s.is_active);
      if (active) {
        setActiveSheetLocal(active.name);
      }
      // A workbook already exists in the backend — mark as loaded so the
      // grid renders instead of staying stuck on the WelcomeScreen.
      setWorkbookLoaded(true);
    } catch {
      // If Tauri is not available (e.g. running in browser for dev), use defaults.
      // workbookLoaded stays false so the WelcomeScreen is shown.
      console.warn('Tauri not available, using default state');
    }

    // Load named ranges
    try {
      const nrs = await listNamedRanges();
      setNamedRanges(nrs);
    } catch {
      // Named ranges not available
    }

    // Load recent files
    try {
      const recent = await getRecentFiles();
      setRecentFilesState(recent);
    } catch {
      // Recent files not available
    }

    // Load merged regions and banded rows for initial sheet
    void refreshMergedRegions();
    void refreshBandedRows();

    // Set initial window title
    updateWindowTitle(currentFilePath(), false);

    // Listen for macOS menu bar events emitted by the Tauri backend.
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listen<string>('menu-event', (event) => {
        const action = menuActions[event.payload];
        if (action) {
          action();
        }
      });
    } catch {
      // Tauri event system not available (browser dev mode).
    }

    // Listen for MCP bridge workbook changes.
    // When Claude writes to the workbook via the MCP socket bridge,
    // the backend emits this event so the frontend repaints the grid.
    let unlistenWorkbookChanged: (() => void) | undefined;
    try {
      unlistenWorkbookChanged = await listen('workbook-changed', () => {
        setRefreshTrigger((n) => n + 1);
      });
    } catch {
      // Tauri event system not available (browser dev mode).
    }

    // Auto-save every 60 seconds if there are unsaved changes and a file path exists.
    const autoSaveInterval = setInterval(() => {
      if (isDirty() && currentFilePath()) {
        void handleFileSave();
      }
    }, 60_000);

    // Warn before closing with unsaved changes.
    const beforeUnloadHandler = (e: BeforeUnloadEvent) => {
      if (isDirty()) {
        e.preventDefault();
        e.returnValue = '';
      }
    };
    window.addEventListener('beforeunload', beforeUnloadHandler);

    onCleanup(() => {
      unlisten?.();
      unlistenWorkbookChanged?.();
      clearInterval(autoSaveInterval);
      window.removeEventListener('beforeunload', beforeUnloadHandler);
    });
  });

  const handleSelectSheet = async (name: string) => {
    try {
      await setActiveSheet(name);
    } catch {
      // Ignore in browser dev mode.
    }
    setActiveSheetLocal(name);
    // Reset selection when switching sheets to avoid stale range targeting
    setSelectedCell([0, 0]);
    setSelRange([0, 0, 0, 0]);
    setFormulaContent('');
    // Refresh merged regions and banded rows for the new sheet
    void refreshMergedRegions();
    void refreshBandedRows();
  };

  const handleNextSheet = () => {
    const list = sheets();
    const idx = list.indexOf(activeSheetName());
    if (idx < list.length - 1) {
      void handleSelectSheet(list[idx + 1]);
    }
  };

  const handlePrevSheet = () => {
    const list = sheets();
    const idx = list.indexOf(activeSheetName());
    if (idx > 0) {
      void handleSelectSheet(list[idx - 1]);
    }
  };

  const handleAddSheet = async () => {
    const existing = sheets();
    let newName = `Sheet${existing.length + 1}`;
    let i = existing.length + 1;
    while (existing.includes(newName)) {
      i++;
      newName = `Sheet${i}`;
    }
    try {
      await addSheet(newName);
    } catch {
      // Ignore in browser dev mode.
    }
    setSheets([...existing, newName]);
    handleSelectSheet(newName);
  };

  const handleRenameSheet = async (oldName: string, newName: string) => {
    try {
      await renameSheet(oldName, newName);
      setSheets(sheets().map((s) => (s === oldName ? newName : s)));
      if (activeSheetName() === oldName) {
        setActiveSheetLocal(newName);
      }
    } catch (e) {
      setStatusMessage(`Rename failed: ${e}`);
    }
  };

  const handleDeleteSheet = async (name: string) => {
    try {
      await deleteSheet(name);
      const remaining = sheets().filter((s) => s !== name);
      setSheets(remaining);
      if (activeSheetName() === name && remaining.length > 0) {
        handleSelectSheet(remaining[0]);
      }
    } catch (e) {
      setStatusMessage(`Delete failed: ${e}`);
    }
  };

  const handleDuplicateSheet = async (name: string) => {
    const newName = `${name} (Copy)`;
    try {
      await duplicateSheet(name, newName);
      setSheets([...sheets(), newName]);
      handleSelectSheet(newName);
    } catch (e) {
      setStatusMessage(`Duplicate failed: ${e}`);
    }
  };

  const handleTabColorChange = async (name: string, color: string | null) => {
    try {
      await setSheetTabColor(name, color);
    } catch {
      // Backend may not support this yet
    }
    if (color) {
      setTabColors({ ...tabColors(), [name]: color });
    } else {
      const next = { ...tabColors() };
      delete next[name];
      setTabColors(next);
    }
  };

  const handleMoveSheet = async (name: string, toIndex: number) => {
    try {
      await moveSheet(name, toIndex);
    } catch {
      // Backend may not support this yet
    }
    // Reorder locally
    const current = sheets().filter((s) => s !== name);
    const clamped = Math.max(0, Math.min(toIndex, current.length));
    current.splice(clamped, 0, name);
    setSheets(current);
  };

  const handleSelectionChange = (row: number, col: number, minRow?: number, minCol?: number, maxRow?: number, maxCol?: number) => {
    setSelectedCell([row, col]);
    // Update the full selection range (defaults to single cell if no range provided).
    const mR = minRow ?? row;
    const mC = minCol ?? col;
    const xR = maxRow ?? row;
    const xC = maxCol ?? col;
    setSelRange([mR, mC, xR, xC]);
    setNameBoxDisplay(nameBoxDisplayStr(row, col, mR, mC, xR, xC));
    setStatusMessage(`Cell ${cellRefStr(row, col)}`);
    // Sync toolbar format state from the selected cell
    getCell(activeSheetName(), row, col)
      .then((cell) => {
        setBoldActive(cell?.bold ?? false);
        setItalicActive(cell?.italic ?? false);
        setUnderlineActive(cell?.underline ?? false);
        setStrikethroughActive(cell?.strikethrough ?? false);
        if (cell?.font_family) setCurrentFontFamily(cell.font_family);
        setCurrentFontSize(cell?.font_size ?? 11);
        setCurrentHAlign(cell?.h_align ?? 'left');
        setCurrentVAlign(cell?.v_align ?? 'bottom');

        // Paint format: apply copied format to clicked cell then clear
        const pf = paintFormatData();
        if (pf) {
          applyFormat(pf);
          setPaintFormatData(null);
          setPaintFormatActive(false);
          setStatusMessage('Format painted');
        }
      })
      .catch(() => {
        setBoldActive(false);
        setItalicActive(false);
        setUnderlineActive(false);
        setStrikethroughActive(false);
      });
  };

  const handleFormulaCommit = async (value: string) => {
    const [row, col] = selectedCell();
    let formula: string | undefined;
    if (value.startsWith('=')) {
      formula = value.slice(1);
    }
    try {
      await setCell(activeSheetName(), row, col, value, formula);
    } catch {
      // Ignore in browser dev mode.
    }
    setFormulaContent(value);
    setRefreshTrigger((n) => n + 1);
    markDirty();
  };

  const handleFormulaCancel = () => {
    // No-op: formula bar resets itself.
  };

  const handleFormulaNavigate = (ref: string) => {
    // Check for range notation (e.g., "A1:C5")
    if (ref.includes(':')) {
      const parts = ref.split(':');
      if (parts.length === 2) {
        // Full row notation (e.g., "3:5")
        const rowOnly = /^(\d+)$/;
        const m1 = parts[0].match(rowOnly);
        const m2 = parts[1].match(rowOnly);
        if (m1 && m2) {
          const r1 = parseInt(m1[1], 10) - 1;
          const r2 = parseInt(m2[1], 10) - 1;
          const minR = Math.min(r1, r2);
          const maxR = Math.max(r1, r2);
          setSelectedCell([minR, 0]);
          setSelRange([minR, 0, maxR, TOTAL_COLS - 1]);
          setNameBoxDisplay(nameBoxDisplayStr(minR, 0, minR, 0, maxR, TOTAL_COLS - 1));
          setNavigateTo([minR, 0, minR, 0, maxR, TOTAL_COLS - 1]);
          setStatusMessage(`Row ${minR + 1}:${maxR + 1}`);
          setFormulaContent('');
          return;
        }
        // Full column notation (e.g., "A:C")
        const colOnly = /^([A-Za-z]+)$/;
        const c1 = parts[0].match(colOnly);
        const c2 = parts[1].match(colOnly);
        if (c1 && c2) {
          const col1 = parseColLetters(c1[1]);
          const col2 = parseColLetters(c2[1]);
          const minC = Math.min(col1, col2);
          const maxC = Math.max(col1, col2);
          setSelectedCell([0, minC]);
          setSelRange([0, minC, TOTAL_ROWS - 1, maxC]);
          setNameBoxDisplay(nameBoxDisplayStr(0, minC, 0, minC, TOTAL_ROWS - 1, maxC));
          setNavigateTo([0, minC, 0, minC, TOTAL_ROWS - 1, maxC]);
          setStatusMessage(`Column ${col_to_letter(minC)}:${col_to_letter(maxC)}`);
          setFormulaContent('');
          return;
        }
        // Cell range notation (e.g., "A1:C5")
        const p1 = parse_cell_ref(parts[0].trim());
        const p2 = parse_cell_ref(parts[1].trim());
        if (p1 && p2) {
          const minR = Math.min(p1.row, p2.row);
          const minC = Math.min(p1.col, p2.col);
          const maxR = Math.max(p1.row, p2.row);
          const maxC = Math.max(p1.col, p2.col);
          setSelectedCell([minR, minC]);
          setSelRange([minR, minC, maxR, maxC]);
          setNameBoxDisplay(nameBoxDisplayStr(minR, minC, minR, minC, maxR, maxC));
          setNavigateTo([minR, minC, minR, minC, maxR, maxC]);
          setStatusMessage(`Range ${cellRefStr(minR, minC)}:${cellRefStr(maxR, maxC)}`);
          // Fetch the top-left cell content for formula bar
          getCell(activeSheetName(), minR, minC)
            .then((cell) => {
              setFormulaContent(cell?.formula ? `=${cell.formula}` : cell?.value ?? '');
            })
            .catch(() => setFormulaContent(''));
          return;
        }
      }
    }
    // Single cell reference
    const parsed = parse_cell_ref(ref);
    if (parsed) {
      setSelectedCell([parsed.row, parsed.col]);
      setSelRange([parsed.row, parsed.col, parsed.row, parsed.col]);
      setNameBoxDisplay(cellRefStr(parsed.row, parsed.col));
      setNavigateTo([parsed.row, parsed.col, parsed.row, parsed.col, parsed.row, parsed.col]);
      setStatusMessage(`Cell ${cellRefStr(parsed.row, parsed.col)}`);
      // Fetch the navigated cell's content for the formula bar
      getCell(activeSheetName(), parsed.row, parsed.col)
        .then((cell) => {
          setFormulaContent(cell?.formula ? `=${cell.formula}` : cell?.value ?? '');
        })
        .catch(() => {
          setFormulaContent('');
        });
    }
  };

  const handleContentChange = (content: string) => {
    setFormulaContent(content);
  };

  const handleCellCommit = (_row: number, _col: number, _value: string) => {
    // Grid handles the actual Tauri call.
    markDirty();
  };

  // Toolbar format actions.
  const applyFormat = async (format: Record<string, unknown>) => {
    const [minR, minC, maxR, maxC] = selRange();
    try {
      await formatCells(activeSheetName(), minR, minC, maxR, maxC, format);
      // Refresh the grid so the canvas re-fetches and renders the new format.
      setRefreshTrigger((n) => n + 1);
      markDirty();
    } catch (e) {
      setStatusMessage(`Format failed: ${e}`);
    }
  };

  function handleBold() {
    setBoldActive(!boldActive());
    applyFormat({ bold: boldActive() });
    setStatusMessage(boldActive() ? 'Bold on' : 'Bold off');
  }

  function handleItalic() {
    setItalicActive(!italicActive());
    applyFormat({ italic: italicActive() });
    setStatusMessage(italicActive() ? 'Italic on' : 'Italic off');
  }

  function handleUnderline() {
    setUnderlineActive(!underlineActive());
    applyFormat({ underline: underlineActive() });
    setStatusMessage(underlineActive() ? 'Underline on' : 'Underline off');
  }

  const handleStrikethrough = () => {
    setStrikethroughActive(!strikethroughActive());
    applyFormat({ strikethrough: strikethroughActive() });
    setStatusMessage(strikethroughActive() ? 'Strikethrough on' : 'Strikethrough off');
  };

  const handleTextWrap = (wrap: 'Overflow' | 'Wrap' | 'Clip') => {
    applyFormat({ text_wrap: wrap });
    setStatusMessage(`Text wrap: ${wrap}`);
  };

  const handleTextRotation = (degrees: number) => {
    applyFormat({ text_rotation: degrees });
    setStatusMessage(degrees === 0 ? 'Text rotation: normal' : `Text rotation: ${degrees}\u00B0`);
  };

  const handleIndent = async () => {
    const [row, col] = selectedCell();
    try {
      const cellData = await getCell(activeSheetName(), row, col);
      const current = cellData?.indent ?? 0;
      applyFormat({ indent: Math.min(current + 1, 15) });
      setStatusMessage(`Indent: ${current + 1}`);
    } catch {
      applyFormat({ indent: 1 });
    }
  };

  const handleOutdent = async () => {
    const [row, col] = selectedCell();
    try {
      const cellData = await getCell(activeSheetName(), row, col);
      const current = cellData?.indent ?? 0;
      if (current > 0) {
        applyFormat({ indent: current - 1 });
        setStatusMessage(`Indent: ${current - 1}`);
      }
    } catch {
      // ignore
    }
  };

  const handlePaintFormat = () => {
    if (paintFormatActive()) {
      // Cancel paint mode
      setPaintFormatActive(false);
      setPaintFormatData(null);
      setStatusMessage('Paint format cancelled');
      return;
    }
    // Copy current cell's format
    const [row, col] = selectedCell();
    getCell(activeSheetName(), row, col)
      .then((cell) => {
        if (!cell) return;
        const fmt: Record<string, unknown> = {};
        if (cell.bold) fmt.bold = true;
        if (cell.italic) fmt.italic = true;
        if (cell.underline) fmt.underline = true;
        if (cell.strikethrough) fmt.strikethrough = true;
        if (cell.font_size && cell.font_size !== 11) fmt.font_size = cell.font_size;
        if (cell.font_family && cell.font_family !== 'Arial') fmt.font_family = cell.font_family;
        if (cell.font_color) fmt.font_color = cell.font_color;
        if (cell.bg_color) fmt.bg_color = cell.bg_color;
        if (cell.h_align && cell.h_align !== 'left') fmt.h_align = cell.h_align;
        if (cell.number_format) fmt.number_format = cell.number_format;
        if (cell.text_wrap && cell.text_wrap !== 'Overflow') fmt.text_wrap = cell.text_wrap;
        setPaintFormatData(fmt);
        setPaintFormatActive(true);
        setStatusMessage('Paint format: click a cell to apply');
      })
      .catch(() => {
        setStatusMessage('Failed to read cell format');
      });
  };

  // Format clipboard for Cmd+Option+C / Cmd+Option+V shortcuts.
  const [formatClipboard, setFormatClipboard] = createSignal<Record<string, unknown> | null>(null);

  const handleCopyFormat = () => {
    const [row, col] = selectedCell();
    getCell(activeSheetName(), row, col)
      .then((cell) => {
        if (!cell) return;
        const fmt: Record<string, unknown> = {};
        if (cell.bold) fmt.bold = true;
        if (cell.italic) fmt.italic = true;
        if (cell.underline) fmt.underline = true;
        if (cell.strikethrough) fmt.strikethrough = true;
        if (cell.font_size && cell.font_size !== 11) fmt.font_size = cell.font_size;
        if (cell.font_family && cell.font_family !== 'Arial') fmt.font_family = cell.font_family;
        if (cell.font_color) fmt.font_color = cell.font_color;
        if (cell.bg_color) fmt.bg_color = cell.bg_color;
        if (cell.h_align && cell.h_align !== 'left') fmt.h_align = cell.h_align;
        if (cell.number_format) fmt.number_format = cell.number_format;
        if (cell.text_wrap && cell.text_wrap !== 'Overflow') fmt.text_wrap = cell.text_wrap;
        setFormatClipboard(fmt);
        setStatusMessage('Format copied');
      })
      .catch(() => {
        setStatusMessage('Failed to copy cell format');
      });
  };

  const handlePasteFormat = () => {
    const fmt = formatClipboard();
    if (!fmt) {
      setStatusMessage('No format copied');
      return;
    }
    applyFormat(fmt);
    setStatusMessage('Format pasted');
  };

  const handleInsertFunction = (fn: string) => {
    const [row, col] = selectedCell();
    const value = `=${fn}(`;
    setFormulaContent(value);
    // Write to cell and let VirtualGrid enter edit mode via content change
    setCell(activeSheetName(), row, col, value, `${fn}(`)
      .catch(() => {});
    setRefreshTrigger((n) => n + 1);
    setStatusMessage(`Inserted ${fn} function`);
  };

  const handleFontFamily = (family: string) => {
    setCurrentFontFamily(family);
    applyFormat({ font_family: family });
    setStatusMessage(`Font family: ${family}`);
  };

  const handleFontSize = (size: number) => {
    applyFormat({ font_size: size });
    setStatusMessage(`Font size: ${size}`);
  };

  const handleFontColor = (color: string) => {
    applyFormat({ font_color: color });
    setStatusMessage(`Text color: ${color}`);
  };

  const handleBgColor = (color: string) => {
    // Send empty string to clear bg_color (backend treats "" as "remove fill").
    // A non-empty color string sets the fill color.
    applyFormat({ bg_color: color });
    setStatusMessage(color ? `Fill color: ${color}` : 'Fill removed');
  };

  const handleBorders = (borders: BordersUpdate) => {
    applyFormat({ borders });
    setStatusMessage('Borders applied');
  };

  const handleAlign = (align: 'left' | 'center' | 'right') => {
    applyFormat({ h_align: align });
    setStatusMessage(`Align: ${align}`);
  };

  const handleVAlign = (align: 'top' | 'middle' | 'bottom') => {
    applyFormat({ v_align: align });
    setStatusMessage(`Vertical align: ${align}`);
  };

  const handleNumberFormat = (fmt: string) => {
    applyFormat({ number_format: fmt });
    setStatusMessage(`Number format: ${fmt}`);
  };

  const handleUndo = async () => {
    try {
      await tauriUndo();
      setRefreshTrigger((n) => n + 1);
      setStatusMessage('Undo');
    } catch {
      setStatusMessage('Nothing to undo');
    }
  };

  const handleRedo = async () => {
    try {
      await tauriRedo();
      setRefreshTrigger((n) => n + 1);
      setStatusMessage('Redo');
    } catch {
      setStatusMessage('Nothing to redo');
    }
  };

  const handleFreezeToggle = () => {
    if (frozenRows() > 0 || frozenCols() > 0) {
      // Unfreeze
      setFrozenRows(0);
      setFrozenCols(0);
      setStatusMessage('Freeze panes removed');
    } else {
      // Freeze at current selection (freeze rows above and columns to the left)
      const [row, col] = selectedCell();
      setFrozenRows(row > 0 ? row : 1);
      setFrozenCols(col > 0 ? col : 1);
      setStatusMessage(`Frozen: ${row > 0 ? row : 1} rows, ${col > 0 ? col : 1} columns`);
    }
  };

  const handleSplitToggle = () => {
    if (splitRow() > 0 || splitCol() > 0) {
      // Remove split
      setSplitRow(0);
      setSplitCol(0);
      setStatusMessage('Split panes removed');
    } else {
      // Split at current selection
      const [row, col] = selectedCell();
      setSplitRow(row > 0 ? row : 1);
      setSplitCol(col > 0 ? col : 1);
      setStatusMessage(`Split: row ${row > 0 ? row : 1}, col ${col > 0 ? col : 1}`);
    }
  };

  async function handleFilterToggle() {
    if (filterActive()) {
      // Remove filter
      try {
        await clearFilter(activeSheetName());
        setFilterActive(false);
        setFilterInfo(null);
        setRefreshTrigger((n) => n + 1);
        setStatusMessage('Filter removed');
      } catch (e) {
        setStatusMessage(`Failed to clear filter: ${e}`);
      }
    } else {
      // Create filter
      try {
        const info = await setAutoFilter(activeSheetName());
        setFilterActive(true);
        setFilterInfo(info);
        setRefreshTrigger((n) => n + 1);
        setStatusMessage('Filter created');
      } catch (e) {
        setStatusMessage(`Failed to create filter: ${e}`);
      }
    }
  }

  const handleFilterColumnClick = (col: number, x: number, y: number) => {
    setFilterDropdownCol(col);
    setFilterDropdownX(x);
    setFilterDropdownY(y);
    setShowFilterDropdown(true);
  };

  const handleFilterApply = (info: FilterInfo) => {
    setFilterInfo(info);
    setShowFilterDropdown(false);
    setRefreshTrigger((n) => n + 1);
    setStatusMessage(`Filter applied: ${info.visible_rows} of ${info.total_rows} rows`);
  };

  const handleZoomChange = (z: number) => {
    setZoom(Math.max(0.25, Math.min(2.0, z)));
  };

  function handleZoomIn() {
    handleZoomChange(Math.round((zoom() + 0.1) * 10) / 10);
  }

  function handleZoomOut() {
    handleZoomChange(Math.round((zoom() - 0.1) * 10) / 10);
  }

  function handleZoomReset() {
    handleZoomChange(1.0);
  }

  // Find bar state
  const [showFindBar, setShowFindBar] = createSignal(false);
  const [findBarReplace, setFindBarReplace] = createSignal(false);
  const [findMatches, setFindMatches] = createSignal<{ row: number; col: number }[]>([]);
  const [findActiveIndex, setFindActiveIndex] = createSignal(-1);

  const handleFindOpen = () => {
    setFindBarReplace(false);
    setShowFindBar(true);
  };

  const handleFindReplaceOpen = () => {
    setFindBarReplace(true);
    setShowFindBar(true);
  };

  const handleFindClose = () => {
    setShowFindBar(false);
    setFindMatches([]);
    setFindActiveIndex(-1);
  };

  // -------------------------------------------------------------------
  // Chart state
  // -------------------------------------------------------------------

  const [chartOverlays, setChartOverlays] = createSignal<ChartOverlay[]>([]);
  const [showChartDialog, setShowChartDialog] = createSignal(false);
  const [editChartId, setEditChartId] = createSignal<string | undefined>(undefined);
  const [editChartType, setEditChartType] = createSignal<ChartTypeStr | undefined>(undefined);
  const [editDataRange, setEditDataRange] = createSignal<string | undefined>(undefined);
  const [editTitle, setEditTitle] = createSignal<string | undefined>(undefined);

  // -------------------------------------------------------------------
  // Slicer state
  // -------------------------------------------------------------------

  const [slicers, setSlicers] = createSignal<SlicerState[]>([]);
  const [showAddSlicer, setShowAddSlicer] = createSignal(false);

  let slicerIdCounter = 0;

  const handleAddSlicer = (col: number, colName: string) => {
    slicerIdCounter += 1;
    const offset = slicers().length * 30;
    const newSlicer: SlicerState = {
      id: `slicer-${slicerIdCounter}-${Date.now()}`,
      sheet: activeSheetName(),
      col,
      colName,
      x: 150 + offset,
      y: 100 + offset,
      width: 220,
      height: 320,
    };
    setSlicers([...slicers(), newSlicer]);
    setShowAddSlicer(false);
    setStatusMessage(`Slicer added for column ${colName}`);
  };

  const handleSlicerClose = (id: string) => {
    setSlicers(slicers().filter((s) => s.id !== id));
    setStatusMessage('Slicer removed');
  };

  const handleSlicerMove = (id: string, x: number, y: number) => {
    setSlicers(slicers().map((s) => (s.id === id ? { ...s, x, y } : s)));
  };

  const handleSlicerResize = (id: string, width: number, height: number) => {
    setSlicers(slicers().map((s) => (s.id === id ? { ...s, width, height } : s)));
  };

  // -------------------------------------------------------------------
  // Merge / Unmerge cells
  // -------------------------------------------------------------------

  const [mergedRegions, setMergedRegions] = createSignal<MergedRegionData[]>([]);

  /** Fetch merged regions for the active sheet. */
  const refreshMergedRegions = async () => {
    try {
      const regions = await getMergedRegions(activeSheetName());
      setMergedRegions(regions);
    } catch {
      setMergedRegions([]);
    }
  };

  async function handleMerge() {
    const [minR, minC, maxR, maxC] = selRange();
    // Need at least 2 cells to merge
    if (minR === maxR && minC === maxC) {
      setStatusMessage('Select more than one cell to merge');
      return;
    }
    try {
      await mergeCells(activeSheetName(), minR, minC, maxR, maxC);
      await refreshMergedRegions();
      setRefreshTrigger((n) => n + 1);
      markDirty();
      setStatusMessage('Cells merged');
    } catch (e) {
      setStatusMessage(`Merge failed: ${e}`);
    }
  }

  const handleUnmerge = async () => {
    const [row, col] = selectedCell();
    try {
      const unmerged = await unmergeCells(activeSheetName(), row, col);
      if (unmerged) {
        await refreshMergedRegions();
        setRefreshTrigger((n) => n + 1);
        markDirty();
        setStatusMessage('Cells unmerged');
      } else {
        setStatusMessage('No merged region at current cell');
      }
    } catch (e) {
      setStatusMessage(`Unmerge failed: ${e}`);
    }
  };

  // -------------------------------------------------------------------
  // Banded (alternating) rows
  // -------------------------------------------------------------------

  const [bandedRows, setBandedRowsState] = createSignal<BandedRowsData | null>(null);

  /** Fetch banded row config for the active sheet. */
  const refreshBandedRows = async () => {
    try {
      const banded = await getBandedRows(activeSheetName());
      setBandedRowsState(banded);
    } catch {
      setBandedRowsState(null);
    }
  };

  async function handleAlternatingColors() {
    const current = bandedRows();
    if (current && current.enabled) {
      // Toggle off
      try {
        await setBandedRows(activeSheetName(), false, '', '', null);
        setBandedRowsState(null);
        setRefreshTrigger((n) => n + 1);
        markDirty();
        setStatusMessage('Alternating colors removed');
      } catch (e) {
        setStatusMessage(`Failed to remove alternating colors: ${e}`);
      }
    } else {
      // Toggle on with default colors
      try {
        await setBandedRows(activeSheetName(), true, '#F3F3F3', '#FFFFFF', '#E8EAED');
        await refreshBandedRows();
        setRefreshTrigger((n) => n + 1);
        markDirty();
        setStatusMessage('Alternating colors applied');
      } catch (e) {
        setStatusMessage(`Failed to apply alternating colors: ${e}`);
      }
    }
  }

  function handleInsertChart() {
    setEditChartId(undefined);
    setEditChartType(undefined);
    setEditDataRange(undefined);
    setEditTitle(undefined);
    setShowChartDialog(true);
  }

  const handleEditChart = async (chartId: string) => {
    try {
      const config = await getChartConfig(chartId);
      setEditChartId(chartId);
      setEditChartType(config.chart_type as ChartTypeStr);
      setEditDataRange(config.data_range);
      setEditTitle(config.title ?? '');
      setShowChartDialog(true);
    } catch (e) {
      setStatusMessage(`Failed to load chart config: ${e}`);
    }
  };

  const handleChartInserted = (chartId: string) => {
    setShowChartDialog(false);

    if (editChartId()) {
      // Edit mode: refresh the overlay info and re-render SVG.
      void (async () => {
        try {
          const config = await getChartConfig(chartId);
          await renderChartSvg(chartId);
          setChartOverlays(
            chartOverlays().map((c) =>
              c.info.id === chartId
                ? { ...c, info: config }
                : c,
            ),
          );
          setStatusMessage('Chart updated');
        } catch {
          setStatusMessage('Chart updated');
        }
      })();
      setEditChartId(undefined);
      setEditChartType(undefined);
      setEditDataRange(undefined);
      setEditTitle(undefined);
    } else {
      // Create mode: fetch the new chart info and add it as an overlay.
      void loadChartOverlay(chartId);
      setStatusMessage('Chart inserted');
    }
  };

  const loadChartOverlay = async (chartId: string) => {
    try {
      const charts = await listCharts(activeSheetName());
      const info = charts.find((c: ChartInfo) => c.id === chartId);
      if (info) {
        // Position new charts with a slight offset so they don't stack exactly.
        const offset = chartOverlays().length * 30;
        const overlay: ChartOverlay = {
          info,
          x: 120 + offset,
          y: 80 + offset,
          width: info.width,
          height: info.height,
        };
        setChartOverlays([...chartOverlays(), overlay]);
      }
    } catch {
      // Ignore in browser dev mode.
    }
  };

  const handleChartDelete = (chartId: string) => {
    setChartOverlays(chartOverlays().filter((c) => c.info.id !== chartId));
    setStatusMessage('Chart deleted');
  };

  const handleChartMove = (chartId: string, x: number, y: number) => {
    setChartOverlays(
      chartOverlays().map((c) =>
        c.info.id === chartId ? { ...c, x, y } : c,
      ),
    );
  };

  const handleChartResize = (chartId: string, width: number, height: number) => {
    setChartOverlays(
      chartOverlays().map((c) =>
        c.info.id === chartId ? { ...c, width, height } : c,
      ),
    );
  };

  const handleChartDialogClose = () => {
    setShowChartDialog(false);
    setEditChartId(undefined);
    setEditChartType(undefined);
    setEditDataRange(undefined);
    setEditTitle(undefined);
  };

  // -------------------------------------------------------------------
  // Paste special state
  // -------------------------------------------------------------------

  const [showFormatCells, setShowFormatCells] = createSignal(false);
  const [showDataValidation, setShowDataValidation] = createSignal(false);
  const [showConditionalFormat, setShowConditionalFormat] = createSignal(false);
  const [showSortDialog, setShowSortDialog] = createSignal(false);
  const [showNamedRanges, setShowNamedRanges] = createSignal(false);
  const [namedRanges, setNamedRanges] = createSignal<NamedRangeInfo[]>([]);
  const [showPasteSpecial, setShowPasteSpecial] = createSignal(false);
  const [pasteSpecialMode, setPasteSpecialMode] = createSignal<PasteMode | null>(null);
  const [showKeyboardShortcuts, setShowKeyboardShortcuts] = createSignal(false);
  const [showPrintPreview, setShowPrintPreview] = createSignal(false);
  const [pageBreakPreview, setPageBreakPreview] = createSignal(false);
  const [showDataCleanup, setShowDataCleanup] = createSignal(false);
  const [showTextToColumns, setShowTextToColumns] = createSignal(false);
  const [showAbout, setShowAbout] = createSignal(false);
  const [showFilterViews, setShowFilterViews] = createSignal(false);
  const [showNamedFunctions, setShowNamedFunctions] = createSignal(false);
  const [showPivotDialog, setShowPivotDialog] = createSignal(false);
  const [showColumnStats, setShowColumnStats] = createSignal(false);
  const [columnStatsCol, setColumnStatsCol] = createSignal(0);

  const handlePasteSpecialOpen = () => {
    setShowPasteSpecial(true);
  };

  const handlePasteSpecialClose = () => {
    setShowPasteSpecial(false);
  };

  const handlePasteSpecialPaste = (mode: PasteMode) => {
    setShowPasteSpecial(false);
    setPasteSpecialMode(mode);
  };

  const handlePasteSpecialDone = () => {
    setPasteSpecialMode(null);
  };

  // Load existing charts on mount.
  onMount(async () => {
    try {
      const charts = await listCharts(activeSheetName());
      const overlays: ChartOverlay[] = charts.map((info: ChartInfo, i: number) => ({
        info,
        x: 120 + i * 30,
        y: 80 + i * 30,
        width: info.width,
        height: info.height,
      }));
      setChartOverlays(overlays);
    } catch {
      // Ignore in browser dev mode.
    }
  });

  return (
    <div class="app-container">
      <Show
        when={workbookLoaded()}
        fallback={
          <WelcomeScreen
            recentFiles={recentFiles()}
            onNewWorkbook={handleFileNew}
            onOpenFile={handleFileOpen}
            onOpenRecent={(path) => void handleOpenRecentFile(path)}
          />
        }
      >
      <Toolbar
        onBold={handleBold}
        onItalic={handleItalic}
        onUnderline={handleUnderline}
        onStrikethrough={handleStrikethrough}
        onFontSize={handleFontSize}
        onFontFamily={handleFontFamily}
        onFontColor={handleFontColor}
        onBgColor={handleBgColor}
        onBorders={handleBorders}
        onAlign={handleAlign}
        onVAlign={handleVAlign}
        onTextWrap={handleTextWrap}
        onNumberFormat={handleNumberFormat}
        onUndo={handleUndo}
        onRedo={handleRedo}
        onFreezeToggle={handleFreezeToggle}
        onSplitToggle={handleSplitToggle}
        onInsertChart={handleInsertChart}
        onFilterToggle={handleFilterToggle}
        onConditionalFormat={() => setShowConditionalFormat(true)}
        onPaintFormat={handlePaintFormat}
        onIndent={handleIndent}
        onOutdent={handleOutdent}
        onTextRotation={handleTextRotation}
        onMerge={handleMerge}
        onUnmerge={handleUnmerge}
        onInsertFunction={handleInsertFunction}
        boldActive={boldActive()}
        italicActive={italicActive()}
        underlineActive={underlineActive()}
        strikethroughActive={strikethroughActive()}
        freezeActive={frozenRows() > 0 || frozenCols() > 0}
        splitActive={splitRow() > 0 || splitCol() > 0}
        filterActive={filterActive()}
        paintFormatActive={paintFormatActive()}
        currentFontFamily={currentFontFamily()}
        currentFontSize={currentFontSize()}
        currentHAlign={currentHAlign()}
        currentVAlign={currentVAlign()}
        onAddSlicer={() => setShowAddSlicer(true)}
      />
      <FormulaBar
        cellRef={nameBoxDisplay()}
        content={formulaContent()}
        onCommit={handleFormulaCommit}
        onCancel={handleFormulaCancel}
        onNavigate={handleFormulaNavigate}
        onContentChange={handleContentChange}
        namedRanges={namedRanges()}
      />
      <Show when={showFindBar()}>
        <FindBar
          activeSheet={activeSheetName()}
          showReplace={findBarReplace()}
          onClose={handleFindClose}
          onNavigateToCell={(row, col) => {
            setSelectedCell([row, col]);
            setSelRange([row, col, row, col]);
            setNameBoxDisplay(cellRefStr(row, col));
            setNavigateTo([row, col, row, col, row, col]);
            setStatusMessage(`Cell ${cellRefStr(row, col)}`);
            // Sync formula bar with navigated cell
            getCell(activeSheetName(), row, col)
              .then((cell) => {
                setFormulaContent(cell?.formula ? `=${cell.formula}` : cell?.value ?? '');
              })
              .catch(() => setFormulaContent(''));
          }}
          onStatusChange={setStatusMessage}
          onDataChanged={() => setRefreshTrigger((n) => n + 1)}
          onMatchesChange={(matches, activeIndex) => {
            setFindMatches(matches);
            setFindActiveIndex(activeIndex);
          }}
        />
      </Show>
      <div style={{ position: 'relative', flex: '1', overflow: 'hidden', display: 'flex', "flex-direction": 'column' }}>
        <VirtualGrid
          activeSheet={activeSheetName()}
          refreshTrigger={refreshTrigger()}
          frozenRows={frozenRows()}
          frozenCols={frozenCols()}
          splitRow={splitRow()}
          splitCol={splitCol()}
          zoom={zoom()}
          onSelectionChange={handleSelectionChange}
          onSelectionSummary={setSelectionSummary}
          onContentChange={handleContentChange}
          onCellCommit={handleCellCommit}
          onStatusChange={setStatusMessage}
          onModeChange={setMode}
          onBoldToggle={handleBold}
          onItalicToggle={handleItalic}
          onUnderlineToggle={handleUnderline}
          onFindOpen={handleFindOpen}
          onFindReplaceOpen={handleFindReplaceOpen}
          onZoomIn={handleZoomIn}
          onZoomOut={handleZoomOut}
          onZoomReset={handleZoomReset}
          onPasteSpecialOpen={handlePasteSpecialOpen}
          pasteSpecialMode={pasteSpecialMode()}
          onPasteSpecialDone={handlePasteSpecialDone}
          findMatches={findMatches()}
          findActiveIndex={findActiveIndex()}
          onFormatCellsOpen={() => setShowFormatCells(true)}
          onDataValidationOpen={() => setShowDataValidation(true)}
          filterActive={filterActive()}
          filterStartCol={filterInfo()?.start_col}
          filterEndCol={filterInfo()?.end_col}
          onFilterColumnClick={handleFilterColumnClick}
          onSortDialogOpen={() => setShowSortDialog(true)}
          onNamedRangesOpen={() => setShowNamedRanges(true)}
          showGridlines={showGridlines()}
          showFormulas={showFormulas()}
          onToggleFormulas={(v: boolean) => {
            setShowFormulas(v);
            setStatusMessage(v ? 'Formula view on' : 'Formula view off');
          }}
          mergedRegions={mergedRegions()}
          bandedRows={bandedRows()}
          onNextSheet={handleNextSheet}
          onPrevSheet={handlePrevSheet}
          onKeyboardShortcutsOpen={() => setShowKeyboardShortcuts(true)}
          onPrintPreviewOpen={() => setShowPrintPreview(true)}
          pageBreakPreview={pageBreakPreview()}
          pageBreakPaperSize="letter"
          pageBreakOrientation="portrait"
          navigateTo={navigateTo()}
          onCopyFormat={handleCopyFormat}
          onPasteFormat={handlePasteFormat}
        />
        <ChartContainer
          charts={chartOverlays()}
          onDelete={handleChartDelete}
          onMove={handleChartMove}
          onResize={handleChartResize}
          onEditChart={(id) => void handleEditChart(id)}
        />
        <SlicerContainer
          slicers={slicers()}
          activeSheet={activeSheetName()}
          onClose={handleSlicerClose}
          onMove={handleSlicerMove}
          onResize={handleSlicerResize}
          onStatusChange={setStatusMessage}
          onRefresh={() => setRefreshTrigger((n) => n + 1)}
        />
      </div>
      <Show when={showChartDialog()}>
        <ChartDialog
          activeSheet={activeSheetName()}
          onInsert={handleChartInserted}
          onClose={handleChartDialogClose}
          editChartId={editChartId()}
          initialChartType={editChartType()}
          initialDataRange={editDataRange()}
          initialTitle={editTitle()}
        />
      </Show>
      <Show when={showFormatCells()}>
        <FormatCellsDialog
          cellValue={formulaContent().startsWith('=') ? '' : formulaContent()}
          onApply={(format) => {
            const [minR, minC, maxR, maxC] = selRange();
            formatCells(activeSheetName(), minR, minC, maxR, maxC, format)
              .then(() => {
                setRefreshTrigger((n) => n + 1);
                setStatusMessage('Format applied');
              })
              .catch((e) => {
                setStatusMessage(`Format failed: ${e}`);
              });
            setShowFormatCells(false);
          }}
          onClose={() => setShowFormatCells(false)}
        />
      </Show>
      <Show when={showDataValidation()}>
        <DataValidationDialog
          activeSheet={activeSheetName()}
          row={selectedCell()[0]}
          col={selectedCell()[1]}
          cellRef={cellRefStr(selectedCell()[0], selectedCell()[1])}
          onClose={() => setShowDataValidation(false)}
          onSaved={() => {
            setShowDataValidation(false);
            setRefreshTrigger((n) => n + 1);
            setStatusMessage('Validation saved');
          }}
        />
      </Show>
      <Show when={showFilterDropdown()}>
        <FilterDropdown
          activeSheet={activeSheetName()}
          col={filterDropdownCol()}
          x={filterDropdownX()}
          y={filterDropdownY()}
          onClose={() => setShowFilterDropdown(false)}
          onApply={handleFilterApply}
        />
      </Show>
      <Show when={showConditionalFormat()}>
        <ConditionalFormatDialog
          activeSheet={activeSheetName()}
          selRange={selRange()}
          onClose={() => setShowConditionalFormat(false)}
          onStatusChange={setStatusMessage}
          onRefresh={() => setRefreshTrigger((n) => n + 1)}
        />
      </Show>
      <Show when={showSortDialog()}>
        <SortDialog
          activeSheet={activeSheetName()}
          defaultCol={selectedCell()[1]}
          maxCol={25}
          onClose={() => setShowSortDialog(false)}
          onSorted={() => setRefreshTrigger((n) => n + 1)}
          onStatusChange={setStatusMessage}
        />
      </Show>
      <Show when={showNamedRanges()}>
        <NamedRangesDialog
          activeSheet={activeSheetName()}
          selectionRange={(() => {
            const [minR, minC, maxR, maxC] = selRange();
            const start = cellRefStr(minR, minC);
            const end = cellRefStr(maxR, maxC);
            return start === end ? start : `${start}:${end}`;
          })()}
          onClose={() => {
            setShowNamedRanges(false);
            // Refresh named ranges after dialog closes
            void listNamedRanges().then(setNamedRanges).catch(() => {});
          }}
          onNavigate={handleFormulaNavigate}
          onStatusChange={setStatusMessage}
        />
      </Show>
      <Show when={showPasteSpecial()}>
        <PasteSpecialDialog
          onPaste={handlePasteSpecialPaste}
          onClose={handlePasteSpecialClose}
        />
      </Show>
      <Show when={showKeyboardShortcuts()}>
        <KeyboardShortcutsDialog onClose={() => setShowKeyboardShortcuts(false)} />
      </Show>
      <Show when={showPrintPreview()}>
        <PrintPreviewDialog
          activeSheet={activeSheetName()}
          sheets={sheets()}
          onClose={() => setShowPrintPreview(false)}
          onStatusChange={setStatusMessage}
        />
      </Show>
      <Show when={showDataCleanup()}>
        <DataCleanupDialog
          activeSheet={activeSheetName()}
          selectionRange={selRange()}
          onClose={() => setShowDataCleanup(false)}
          onDataChanged={() => { setRefreshTrigger((n) => n + 1); markDirty(); }}
          onStatusChange={setStatusMessage}
        />
      </Show>
      <Show when={showTextToColumns()}>
        <TextToColumnsDialog
          onApply={(delimiter: string) => {
            const [, col] = selectedCell();
            const [minRow, , maxRow] = selRange();
            void textToColumns(activeSheetName(), col, delimiter, minRow, maxRow)
              .then((maxCols) => {
                setShowTextToColumns(false);
                setRefreshTrigger((n) => n + 1);
                markDirty();
                setStatusMessage(`Text split into ${maxCols} columns`);
              })
              .catch((e) => setStatusMessage(`Text to columns failed: ${e}`));
          }}
          onClose={() => setShowTextToColumns(false)}
        />
      </Show>
      <Show when={showNamedFunctions()}>
        <NamedFunctionsDialog
          onClose={() => setShowNamedFunctions(false)}
          onStatusChange={setStatusMessage}
        />
      </Show>
      <Show when={showPivotDialog()}>
        <PivotDialog
          activeSheet={activeSheetName()}
          selectionRange={selRange()}
          onClose={() => setShowPivotDialog(false)}
          onCreated={(targetSheet) => {
            setShowPivotDialog(false);
            void listSheets().then((sl) => {
              setSheets(sl.map((s) => s.name));
              void handleSelectSheet(targetSheet);
            }).catch(() => {});
            setRefreshTrigger((n) => n + 1);
            markDirty();
          }}
          onStatusChange={setStatusMessage}
        />
      </Show>
      <Show when={showColumnStats()}>
        <ColumnStatsPanel
          activeSheet={activeSheetName()}
          col={columnStatsCol()}
          totalCols={TOTAL_COLS}
          onClose={() => setShowColumnStats(false)}
          onColumnChange={(c) => setColumnStatsCol(c)}
        />
      </Show>
      <Show when={showFilterViews()}>
        <FilterViewDropdown
          activeSheet={activeSheetName()}
          onClose={() => setShowFilterViews(false)}
          onStatusChange={setStatusMessage}
          onRefresh={() => setRefreshTrigger((n) => n + 1)}
        />
      </Show>
      <Show when={showAbout()}>
        <AboutDialog onClose={() => setShowAbout(false)} />
      </Show>
      <Show when={showAddSlicer()}>
        <AddSlicerDialog
          onAdd={handleAddSlicer}
          onClose={() => setShowAddSlicer(false)}
        />
      </Show>
      <SheetTabs
        sheets={sheets()}
        activeSheet={activeSheetName()}
        onSelectSheet={handleSelectSheet}
        onAddSheet={handleAddSheet}
        onRenameSheet={handleRenameSheet}
        onDeleteSheet={handleDeleteSheet}
        onDuplicateSheet={handleDuplicateSheet}
        onTabColorChange={handleTabColorChange}
        onMoveSheet={handleMoveSheet}
        tabColors={tabColors()}
      />
      <StatusBar
        message={statusMessage()}
        mode={mode()}
        selectionSummary={selectionSummary()}
        zoom={zoom()}
        onZoomChange={handleZoomChange}
        filterSummary={filterInfo() ? `${filterInfo()!.visible_rows} of ${filterInfo()!.total_rows} rows displayed` : undefined}
        saveStatus={saveStatus()}
      />
      </Show>
    </div>
  );
};

export default App;
