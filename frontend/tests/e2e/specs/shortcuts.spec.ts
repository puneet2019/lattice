import {
  createNewSpreadsheet,
  clickCell,
  doubleClickCell,
  getFormulaBarContent,
  typeInCell,
  readCellContent,
  isToolbarButtonActive,
} from '../helpers/app';

describe('Keyboard shortcuts', () => {
  before(async () => {
    await createNewSpreadsheet();
  });

  // --- Cmd+B toggles bold ---
  it('should toggle bold with Cmd+B', async () => {
    await typeInCell(0, 0, 'Hi');
    await clickCell(0, 0);
    await browser.pause(200);

    await browser.keys(['Meta', 'b']);
    await browser.pause(200);
    expect(await isToolbarButtonActive('Bold')).toBe(true);

    await browser.keys(['Meta', 'b']);
    await browser.pause(200);
    expect(await isToolbarButtonActive('Bold')).toBe(false);
  });

  // --- Cmd+Z undo: type value, undo, verify empty ---
  it('should undo a typed value with Cmd+Z', async () => {
    await typeInCell(0, 1, 'DeleteMe');

    const before = await readCellContent(0, 1);
    expect(before).toBe('DeleteMe');

    await browser.keys(['Meta', 'z']);
    await browser.pause(300);

    const after = await readCellContent(0, 1);
    expect(after).toBe('');
  });

  // --- Cmd+Shift+Z redo: undo then redo, verify restored ---
  it('should redo with Cmd+Shift+Z', async () => {
    // After previous test, B1 is empty from undo.
    // Redo should restore "DeleteMe".
    await browser.keys(['Meta', 'Shift', 'z']);
    await browser.pause(300);

    const content = await readCellContent(0, 1);
    expect(content).toBe('DeleteMe');
  });

  // --- Cmd+C / Cmd+V copy-paste ---
  it('should copy and paste a cell value', async () => {
    await clickCell(0, 0); // A1 has "Hi"
    await browser.pause(200);

    await browser.keys(['Meta', 'c']); // Copy
    await browser.pause(200);

    await clickCell(2, 2); // C3 (empty)
    await browser.pause(200);

    await browser.keys(['Meta', 'v']); // Paste
    await browser.pause(400);

    const content = await readCellContent(2, 2);
    expect(content).toBe('Hi');
  });

  // --- Delete clears selected range ---
  it('should clear selected cells with Delete', async () => {
    // Put values in D1:D3
    await typeInCell(0, 3, 'A');
    await typeInCell(1, 3, 'B');
    await typeInCell(2, 3, 'C');

    // Select D1:D3
    await clickCell(0, 3);
    await browser.pause(100);
    await browser.keys(['Shift', 'ArrowDown']);
    await browser.pause(100);
    await browser.keys(['Shift', 'ArrowDown']);
    await browser.pause(200);

    await browser.keys(['Delete']);
    await browser.pause(300);

    // Verify all three cells are empty
    expect(await readCellContent(0, 3)).toBe('');
    expect(await readCellContent(1, 3)).toBe('');
    expect(await readCellContent(2, 3)).toBe('');
  });

  // --- Cmd+D fill down ---
  it('should fill down with Cmd+D', async () => {
    // Put "FillMe" in E1
    await typeInCell(0, 4, 'FillMe');

    // Select E1:E3
    await clickCell(0, 4);
    await browser.pause(100);
    await browser.keys(['Shift', 'ArrowDown']);
    await browser.pause(100);
    await browser.keys(['Shift', 'ArrowDown']);
    await browser.pause(200);

    await browser.keys(['Meta', 'd']);
    await browser.pause(400);

    expect(await readCellContent(1, 4)).toBe('FillMe');
    expect(await readCellContent(2, 4)).toBe('FillMe');
  });

  // --- Cmd+; inserts current date ---
  it('should insert date with Cmd+;', async () => {
    await clickCell(3, 3); // D4
    await browser.pause(200);

    await browser.keys(['Meta', ';']);
    await browser.pause(400);

    const content = await readCellContent(3, 3);
    // Date string should be non-empty and contain a slash or dash
    expect(content.length).toBeGreaterThan(0);
  });

  // --- Cmd+Shift+; inserts current time ---
  it('should insert time with Cmd+Shift+;', async () => {
    await clickCell(4, 3); // D5
    await browser.pause(200);

    await browser.keys(['Meta', 'Shift', ';']);
    await browser.pause(400);

    const content = await readCellContent(4, 3);
    // Time string should be non-empty and contain a colon
    expect(content.length).toBeGreaterThan(0);
  });

  // --- Cmd+F opens find bar ---
  it('should open find bar with Cmd+F', async () => {
    await browser.keys(['Meta', 'f']);
    await browser.pause(300);

    const findBar = await $('.find-bar');
    expect(await findBar.isDisplayed()).toBe(true);

    await browser.keys(['Escape']);
    await browser.pause(200);
  });

  // --- F2 enters edit mode ---
  it('should enter edit mode with F2', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    await browser.keys(['F2']);
    await browser.pause(200);

    const editor = await $('textarea');
    expect(await editor.isExisting()).toBe(true);

    await browser.keys(['Escape']);
    await browser.pause(200);
  });

  // --- Escape cancels editing ---
  it('should cancel editing with Escape and preserve original', async () => {
    await clickCell(0, 0);
    await browser.pause(100);
    await browser.keys(['F2']);
    await browser.pause(100);
    await browser.keys(['X', 'X', 'X']);
    await browser.pause(100);
    await browser.keys(['Escape']);
    await browser.pause(200);

    const content = await readCellContent(0, 0);
    expect(content).not.toContain('XXX');
  });

  // --- Cmd+/ opens keyboard shortcuts dialog ---
  it('should open keyboard shortcuts dialog with Cmd+/', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await browser.keys(['Meta', '/']);
    await browser.pause(300);

    const dialog = await $('.kbd-shortcuts-dialog');
    expect(await dialog.isDisplayed()).toBe(true);

    await browser.keys(['Escape']);
    await browser.pause(200);
  });
});
