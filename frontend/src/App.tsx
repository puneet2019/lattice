import type { Component } from 'solid-js';
import { createSignal, onMount } from 'solid-js';
import Toolbar from './components/Toolbar';
import FormulaBar from './components/FormulaBar';
import Grid from './components/Grid';
import SheetTabs from './components/SheetTabs';
import StatusBar from './components/StatusBar';
import { listSheets, addSheet, setActiveSheet, setCell } from './bridge/tauri';
import './styles/grid.css';

/** Convert 0-based row,col to A1-style reference. */
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
  const [activeSheet, setActiveSheetLocal] = createSignal('Sheet1');
  const [selectedCell, setSelectedCell] = createSignal<[number, number]>([0, 0]);
  const [formulaContent, setFormulaContent] = createSignal('');
  const [statusMessage, setStatusMessage] = createSignal('Ready');

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

  const handleSelectCell = (row: number, col: number) => {
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
      await setCell(activeSheet(), row, col, value, formula);
    } catch {
      // Ignore in browser dev mode.
    }
    setFormulaContent(value);
  };

  const handleFormulaCancel = () => {
    // No-op: formula bar resets itself.
  };

  const handleCellCommit = (_row: number, _col: number, _value: string) => {
    // Grid handles the actual Tauri call.
  };

  const handleContentChange = (content: string) => {
    setFormulaContent(content);
  };

  // Toolbar actions (stubs for now).
  const handleBold = () => setStatusMessage('Bold toggled (not yet implemented)');
  const handleItalic = () => setStatusMessage('Italic toggled (not yet implemented)');
  const handleUnderline = () => setStatusMessage('Underline toggled (not yet implemented)');

  return (
    <div class="app-container">
      <Toolbar onBold={handleBold} onItalic={handleItalic} onUnderline={handleUnderline} />
      <FormulaBar
        cellRef={cellRefStr(selectedCell()[0], selectedCell()[1])}
        content={formulaContent()}
        onCommit={handleFormulaCommit}
        onCancel={handleFormulaCancel}
      />
      <Grid
        activeSheet={activeSheet()}
        selectedCell={selectedCell()}
        onSelectCell={handleSelectCell}
        onCellCommit={handleCellCommit}
        onContentChange={handleContentChange}
      />
      <SheetTabs
        sheets={sheets()}
        activeSheet={activeSheet()}
        onSelectSheet={handleSelectSheet}
        onAddSheet={handleAddSheet}
      />
      <StatusBar message={statusMessage()} />
    </div>
  );
};

export default App;
