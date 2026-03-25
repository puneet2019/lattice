import {
  createNewSpreadsheet,
  clickCell,
  typeInCell,
  readCellContent,
  getNameBoxContent,
} from '../helpers/app';

describe('Autofill / Fill down', () => {
  before(async () => {
    await createNewSpreadsheet();
  });

  // --- Populate A1:A3 with 1,2,3 ---
  it('should populate A1:A3 with sequential values', async () => {
    await typeInCell(0, 0, '1');
    await typeInCell(1, 0, '2');
    await typeInCell(2, 0, '3');

    expect(await readCellContent(0, 0)).toBe('1');
    expect(await readCellContent(1, 0)).toBe('2');
    expect(await readCellContent(2, 0)).toBe('3');
  });

  // --- Cmd+D fill down from A1 into A2:A6 ---
  it('should fill down with Cmd+D from A1 to A6', async () => {
    // Put a value in A1 for fill down
    await typeInCell(0, 0, 'FillValue');

    // Select A1:A6 (A2:A6 are empty or have old values)
    await clickCell(0, 0);
    await browser.pause(100);
    for (let i = 0; i < 5; i++) {
      await browser.keys(['Shift', 'ArrowDown']);
      await browser.pause(50);
    }
    await browser.pause(200);

    // Fill down
    await browser.keys(['Meta', 'd']);
    await browser.pause(400);

    // Verify A2-A6 are filled with "FillValue"
    for (let i = 1; i <= 5; i++) {
      const content = await readCellContent(i, 0);
      expect(content).toBe('FillValue');
    }
  });

  // --- Cmd+D with formula fills adjusted references ---
  it('should fill down a formula and adjust cell references', async () => {
    // Put values in B1:B3
    await typeInCell(0, 1, '10');
    await typeInCell(1, 1, '20');
    await typeInCell(2, 1, '30');

    // Put formula =B1*2 in C1
    await typeInCell(0, 2, '=B1*2');

    // Select C1:C3
    await clickCell(0, 2);
    await browser.pause(100);
    await browser.keys(['Shift', 'ArrowDown']);
    await browser.pause(100);
    await browser.keys(['Shift', 'ArrowDown']);
    await browser.pause(200);

    // Fill down
    await browser.keys(['Meta', 'd']);
    await browser.pause(400);

    // C2 should have =B2*2, C3 should have =B3*2
    const c2 = await readCellContent(1, 2);
    expect(c2).toContain('B2');

    const c3 = await readCellContent(2, 2);
    expect(c3).toContain('B3');
  });

  // --- Fill down text ---
  it('should fill down text values', async () => {
    await typeInCell(0, 3, 'Repeat');

    // Select D1:D4
    await clickCell(0, 3);
    await browser.pause(100);
    for (let i = 0; i < 3; i++) {
      await browser.keys(['Shift', 'ArrowDown']);
      await browser.pause(50);
    }
    await browser.pause(200);

    await browser.keys(['Meta', 'd']);
    await browser.pause(400);

    expect(await readCellContent(1, 3)).toBe('Repeat');
    expect(await readCellContent(2, 3)).toBe('Repeat');
    expect(await readCellContent(3, 3)).toBe('Repeat');
  });
});
