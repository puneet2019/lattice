import type { Component } from 'solid-js';
import { createSignal, onMount } from 'solid-js';
import Toolbar from './components/Toolbar';
import FormulaBar from './components/FormulaBar';
import VirtualGrid from './components/Grid/VirtualGrid';
import SheetTabs from './components/SheetTabs';
import StatusBar from './components/StatusBar';
import {
  listSheets,
  addSheet,
  setActiveSheet,
  setCell,
  renameSheet,
  deleteSheet,
  duplicateSheet,
  formatCells,
  undo as tauriUndo,
  redo as tauriRedo,
} from './bridge/tauri';
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
  const [boldActive, setBoldActive] = createSignal(false);
  const [italicActive, setItalicActive] = createSignal(false);
  const [underlineActive, setUnderlineActive] = createSignal(false);

  // Load sheets on mount.
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
      setStatusMessage('Undo');
    } catch {
      setStatusMessage('Nothing to undo');
    }
  };

  const handleRedo = async () => {
    try {
      await tauriRedo();
      setStatusMessage('Redo');
    } catch {
      setStatusMessage('Nothing to redo');
    }
  };

  const handleZoomChange = (z: number) => {
    setZoom(z);
  };

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
        boldActive={boldActive()}
        italicActive={italicActive()}
        underlineActive={underlineActive()}
      />
      <FormulaBar
        cellRef={cellRefStr(selectedCell()[0], selectedCell()[1])}
        content={formulaContent()}
        onCommit={handleFormulaCommit}
        onCancel={handleFormulaCancel}
        onNavigate={handleFormulaNavigate}
        onContentChange={handleContentChange}
      />
      <VirtualGrid
        activeSheet={activeSheetName()}
        onSelectionChange={handleSelectionChange}
        onContentChange={handleContentChange}
        onCellCommit={handleCellCommit}
        onStatusChange={setStatusMessage}
        onModeChange={setMode}
        onBoldToggle={handleBold}
        onItalicToggle={handleItalic}
        onUnderlineToggle={handleUnderline}
      />
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
