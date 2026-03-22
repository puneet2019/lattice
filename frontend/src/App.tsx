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
import ChartContainer from './components/Charts/ChartContainer';
import type { ChartOverlay } from './components/Charts/ChartContainer';
import ChartDialog from './components/Charts/ChartDialog';
import {
  listSheets,
  addSheet,
  setActiveSheet,
  setCell,
  renameSheet,
  deleteSheet,
  duplicateSheet,
  formatCells,
  openFile,
  saveFile,
  newWorkbook,
  undo as tauriUndo,
  redo as tauriRedo,
  listCharts,
} from './bridge/tauri';
import type { ChartInfo } from './bridge/tauri';
import { parse_cell_ref } from './bridge/tauri_helpers';
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

const App: Component = () => {
  const [sheets, setSheets] = createSignal<string[]>(['Sheet1']);
  const [activeSheetName, setActiveSheetLocal] = createSignal('Sheet1');
  const [selectedCell, setSelectedCell] = createSignal<[number, number]>([0, 0]);
  const [formulaContent, setFormulaContent] = createSignal('');
  const [statusMessage, setStatusMessage] = createSignal('Ready');
  const [mode, setMode] = createSignal<'Ready' | 'Edit'>('Ready');
  const [selectionSummary] = createSignal('');
  const [zoom, setZoom] = createSignal(1.0);
  const [refreshTrigger, setRefreshTrigger] = createSignal(0);
  const [boldActive, setBoldActive] = createSignal(false);
  const [italicActive, setItalicActive] = createSignal(false);
  const [underlineActive, setUnderlineActive] = createSignal(false);
  const [currentFilePath, setCurrentFilePath] = createSignal<string | null>(null);
  const [frozenRows, setFrozenRows] = createSignal(0);
  const [frozenCols, setFrozenCols] = createSignal(0);

  // Spreadsheet file filter for open/save dialogs.
  const fileFilters = [
    { name: 'Spreadsheet', extensions: ['xlsx', 'lattice'] },
    { name: 'All Files', extensions: ['*'] },
  ];

  // -------------------------------------------------------------------
  // File operations (triggered by menu events)
  // -------------------------------------------------------------------

  /** Apply workbook info from open/new result to local state. */
  function applyWorkbookInfo(info: { sheets: string[]; active_sheet: string }) {
    setSheets(info.sheets);
    setActiveSheetLocal(info.active_sheet);
    setRefreshTrigger((n) => n + 1);
    setSelectedCell([0, 0]);
    setFormulaContent('');
  }

  const handleFileNew = async () => {
    try {
      const info = await newWorkbook();
      applyWorkbookInfo(info);
      setCurrentFilePath(null);
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
      const info = await openFile(path);
      applyWorkbookInfo(info);
      setCurrentFilePath(path);
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
      await saveFile(path);
      setStatusMessage(`Saved: ${path}`);
    } catch (e) {
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
      await saveFile(path);
      setCurrentFilePath(path);
      setStatusMessage(`Saved: ${path}`);
    } catch (e) {
      setStatusMessage(`Save As failed: ${e}`);
    }
  };

  // -------------------------------------------------------------------
  // Menu event listener
  // -------------------------------------------------------------------

  const menuActions: Record<string, () => void> = {
    file_new: handleFileNew,
    file_open: handleFileOpen,
    file_save: handleFileSave,
    file_save_as: handleFileSaveAs,
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
    } catch {
      // If Tauri is not available (e.g. running in browser for dev), use defaults.
      console.warn('Tauri not available, using default state');
    }

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
    onCleanup(() => {
      unlisten?.();
    });
  });

  const handleSelectSheet = async (name: string) => {
    try {
      await setActiveSheet(name);
    } catch {
      // Ignore in browser dev mode.
    }
    setActiveSheetLocal(name);
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

  const handleSelectionChange = (row: number, col: number) => {
    setSelectedCell([row, col]);
    setStatusMessage(`Cell ${cellRefStr(row, col)}`);
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
  };

  const handleFormulaCancel = () => {
    // No-op: formula bar resets itself.
  };

  const handleFormulaNavigate = (ref: string) => {
    const parsed = parse_cell_ref(ref);
    if (parsed) {
      setSelectedCell([parsed.row, parsed.col]);
      setStatusMessage(`Cell ${cellRefStr(parsed.row, parsed.col)}`);
    }
  };

  const handleContentChange = (content: string) => {
    setFormulaContent(content);
  };

  const handleCellCommit = (_row: number, _col: number, _value: string) => {
    // Grid handles the actual Tauri call.
  };

  // Toolbar format actions.
  const applyFormat = async (format: Record<string, unknown>) => {
    const [row, col] = selectedCell();
    try {
      await formatCells(activeSheetName(), row, col, row, col, format);
    } catch {
      // Ignore in browser dev mode — command may not exist yet.
    }
  };

  const handleBold = () => {
    setBoldActive(!boldActive());
    applyFormat({ bold: boldActive() });
    setStatusMessage(boldActive() ? 'Bold on' : 'Bold off');
  };

  const handleItalic = () => {
    setItalicActive(!italicActive());
    applyFormat({ italic: italicActive() });
    setStatusMessage(italicActive() ? 'Italic on' : 'Italic off');
  };

  const handleUnderline = () => {
    setUnderlineActive(!underlineActive());
    applyFormat({ underline: underlineActive() });
    setStatusMessage(underlineActive() ? 'Underline on' : 'Underline off');
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
    applyFormat({ bg_color: color || null });
    setStatusMessage(color ? `Fill color: ${color}` : 'Fill removed');
  };

  const handleAlign = (align: 'left' | 'center' | 'right') => {
    applyFormat({ h_align: align });
    setStatusMessage(`Align: ${align}`);
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

  const handleZoomChange = (z: number) => {
    setZoom(Math.max(0.25, Math.min(2.0, z)));
  };

  const handleZoomIn = () => {
    handleZoomChange(Math.round((zoom() + 0.1) * 10) / 10);
  };

  const handleZoomOut = () => {
    handleZoomChange(Math.round((zoom() - 0.1) * 10) / 10);
  };

  const handleZoomReset = () => {
    handleZoomChange(1.0);
  };

  // Find bar state
  const [showFindBar, setShowFindBar] = createSignal(false);
  const [findBarReplace, setFindBarReplace] = createSignal(false);

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
  };

  // -------------------------------------------------------------------
  // Chart state
  // -------------------------------------------------------------------

  const [chartOverlays, setChartOverlays] = createSignal<ChartOverlay[]>([]);
  const [showChartDialog, setShowChartDialog] = createSignal(false);

  const handleInsertChart = () => {
    setShowChartDialog(true);
  };

  const handleChartInserted = (chartId: string) => {
    setShowChartDialog(false);
    // Fetch the new chart info and add it as an overlay.
    void loadChartOverlay(chartId);
    setStatusMessage('Chart inserted');
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
      <Toolbar
        onBold={handleBold}
        onItalic={handleItalic}
        onUnderline={handleUnderline}
        onFontSize={handleFontSize}
        onFontColor={handleFontColor}
        onBgColor={handleBgColor}
        onAlign={handleAlign}
        onUndo={handleUndo}
        onRedo={handleRedo}
        onFreezeToggle={handleFreezeToggle}
        onInsertChart={handleInsertChart}
        boldActive={boldActive()}
        italicActive={italicActive()}
        underlineActive={underlineActive()}
        freezeActive={frozenRows() > 0 || frozenCols() > 0}
      />
      <FormulaBar
        cellRef={cellRefStr(selectedCell()[0], selectedCell()[1])}
        content={formulaContent()}
        onCommit={handleFormulaCommit}
        onCancel={handleFormulaCancel}
        onNavigate={handleFormulaNavigate}
        onContentChange={handleContentChange}
      />
      <Show when={showFindBar()}>
        <FindBar
          activeSheet={activeSheetName()}
          showReplace={findBarReplace()}
          onClose={handleFindClose}
          onNavigateToCell={(row, col) => {
            setSelectedCell([row, col]);
            setStatusMessage(`Cell ${cellRefStr(row, col)}`);
          }}
          onStatusChange={setStatusMessage}
          onDataChanged={() => setRefreshTrigger((n) => n + 1)}
        />
      </Show>
      <div style={{ position: 'relative', flex: '1', overflow: 'hidden', display: 'flex', "flex-direction": 'column' }}>
        <VirtualGrid
          activeSheet={activeSheetName()}
          refreshTrigger={refreshTrigger()}
          frozenRows={frozenRows()}
          frozenCols={frozenCols()}
          zoom={zoom()}
          onSelectionChange={handleSelectionChange}
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
        />
        <ChartContainer
          charts={chartOverlays()}
          onDelete={handleChartDelete}
          onMove={handleChartMove}
          onResize={handleChartResize}
        />
      </div>
      <Show when={showChartDialog()}>
        <ChartDialog
          activeSheet={activeSheetName()}
          onInsert={handleChartInserted}
          onClose={handleChartDialogClose}
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
      />
      <StatusBar
        message={statusMessage()}
        mode={mode()}
        selectionSummary={selectionSummary()}
        zoom={zoom()}
        onZoomChange={handleZoomChange}
      />
    </div>
  );
};

export default App;
