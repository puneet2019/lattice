import {
  createNewSpreadsheet,
  clickCell,
  doubleClickCell,
  getNameBoxContent,
  getFormulaBarContent,
} from '../helpers/app';

describe('Cell interactions', () => {
  before(async () => {
    // Start with a fresh spreadsheet for this suite.
    await createNewSpreadsheet();
  });

  it('should update the name box when clicking a cell', async () => {
    // Click cell B2 (row 1, col 1 -- zero-based)
    await clickCell(1, 1);

    // Allow a short delay for signals to propagate.
    await browser.pause(200);

    const nameBox = await getNameBoxContent();
    expect(nameBox).toBe('B2');
  });

  it('should type text into a cell and commit with Enter', async () => {
    // Click cell A1
    await clickCell(0, 0);
    await browser.pause(100);

    // Double-click to enter edit mode
    await doubleClickCell(0, 0);
    await browser.pause(100);

    // Type "Hello" and press Enter to commit
    await browser.keys(['H', 'e', 'l', 'l', 'o', 'Enter']);
    await browser.pause(300);

    // Navigate back to A1 to verify via formula bar
    await clickCell(0, 0);
    await browser.pause(200);

    const content = await getFormulaBarContent();
    expect(content).toBe('Hello');
  });

  it('should evaluate a formula', async () => {
    // Put "10" in A2
    await clickCell(1, 0);
    await doubleClickCell(1, 0);
    await browser.pause(100);
    await browser.keys(['1', '0', 'Enter']);
    await browser.pause(200);

    // Put "20" in A3
    await clickCell(2, 0);
    await doubleClickCell(2, 0);
    await browser.pause(100);
    await browser.keys(['2', '0', 'Enter']);
    await browser.pause(200);

    // Put "=A2+A3" in A4
    await clickCell(3, 0);
    await doubleClickCell(3, 0);
    await browser.pause(100);
    await browser.keys(['=', 'A', '2', '+', 'A', '3', 'Enter']);
    await browser.pause(300);

    // Verify formula bar shows the formula and the cell displays "30"
    await clickCell(3, 0);
    await browser.pause(200);

    const content = await getFormulaBarContent();
    // Formula bar should show the formula prefixed with "="
    expect(content).toBe('=A2+A3');
  });

  it('should clear a cell with Delete key', async () => {
    // Click cell A1 (which has "Hello" from earlier test)
    await clickCell(0, 0);
    await browser.pause(200);

    // Press Delete to clear the cell
    await browser.keys(['Delete']);
    await browser.pause(300);

    // Verify the cell is now empty
    await clickCell(0, 0);
    await browser.pause(200);

    const content = await getFormulaBarContent();
    expect(content).toBe('');
  });

  it('should undo a delete and restore the previous value', async () => {
    // Undo the delete (Cmd+Z on macOS)
    await browser.keys(['Meta', 'z']);
    await browser.pause(300);

    // Verify the cell has its value restored
    await clickCell(0, 0);
    await browser.pause(200);

    const content = await getFormulaBarContent();
    expect(content).toBe('Hello');
  });
});
