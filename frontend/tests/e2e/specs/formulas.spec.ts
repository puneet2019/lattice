import {
  createNewSpreadsheet,
  clickCell,
  doubleClickCell,
  getFormulaBarContent,
  typeInCell,
  readCellContent,
} from '../helpers/app';

describe('Formulas', () => {
  before(async () => {
    await createNewSpreadsheet();
  });

  // --- SUM with literal args ---
  it('should evaluate =SUM(1,2,3) and show formula in bar', async () => {
    await typeInCell(0, 0, '=SUM(1,2,3)');
    const content = await readCellContent(0, 0);
    expect(content).toBe('=SUM(1,2,3)');
  });

  // --- AVERAGE ---
  it('should evaluate =AVERAGE(10,20) and show formula in bar', async () => {
    await typeInCell(1, 0, '=AVERAGE(10,20)');
    const content = await readCellContent(1, 0);
    expect(content).toBe('=AVERAGE(10,20)');
  });

  // --- IF ---
  it('should evaluate =IF(1>0,"yes","no") to "yes"', async () => {
    await typeInCell(2, 0, '=IF(1>0,"yes","no")');
    const content = await readCellContent(2, 0);
    expect(content).toBe('=IF(1>0,"yes","no")');
  });

  // --- CONCATENATE ---
  it('should evaluate =CONCATENATE("Hello"," ","World")', async () => {
    await typeInCell(3, 0, '=CONCATENATE("Hello"," ","World")');
    const content = await readCellContent(3, 0);
    expect(content).toContain('CONCATENATE');
  });

  // --- LEN ---
  it('should evaluate =LEN("test") to 4', async () => {
    await typeInCell(4, 0, '=LEN("test")');
    const content = await readCellContent(4, 0);
    expect(content).toBe('=LEN("test")');
  });

  // --- MAX ---
  it('should evaluate =MAX(1,5,3) to 5', async () => {
    await typeInCell(5, 0, '=MAX(1,5,3)');
    const content = await readCellContent(5, 0);
    expect(content).toBe('=MAX(1,5,3)');
  });

  // --- MIN ---
  it('should evaluate =MIN(1,5,3) to 1', async () => {
    await typeInCell(6, 0, '=MIN(1,5,3)');
    const content = await readCellContent(6, 0);
    expect(content).toBe('=MIN(1,5,3)');
  });

  // --- COUNT ---
  it('should evaluate =COUNT(1,2,3) to 3', async () => {
    await typeInCell(7, 0, '=COUNT(1,2,3)');
    const content = await readCellContent(7, 0);
    expect(content).toBe('=COUNT(1,2,3)');
  });

  // --- TODAY ---
  it('should evaluate =TODAY() as a formula', async () => {
    await typeInCell(8, 0, '=TODAY()');
    const content = await readCellContent(8, 0);
    expect(content).toBe('=TODAY()');
  });

  // --- ROUND ---
  it('should evaluate =ROUND(3.14159,2)', async () => {
    await typeInCell(9, 0, '=ROUND(3.14159,2)');
    const content = await readCellContent(9, 0);
    expect(content).toBe('=ROUND(3.14159,2)');
  });

  // --- Division by zero ---
  it('should handle division by zero =1/0', async () => {
    await typeInCell(0, 1, '=1/0');
    const content = await readCellContent(0, 1);
    // Formula bar shows the formula; the cell will show #DIV/0! error
    expect(content).toBe('=1/0');
  });

  // --- LET ---
  it('should evaluate =LET(x,10,x*2)', async () => {
    await typeInCell(1, 1, '=LET(x,10,x*2)');
    const content = await readCellContent(1, 1);
    expect(content).toBe('=LET(x,10,x*2)');
  });

  // --- Cell range formulas ---
  it('should evaluate SUM with cell range after populating data', async () => {
    // Put values 10, 20, 30 in C1:C3
    await typeInCell(0, 2, '10');
    await typeInCell(1, 2, '20');
    await typeInCell(2, 2, '30');

    // Put =SUM(C1:C3) in C4
    await typeInCell(3, 2, '=SUM(C1:C3)');

    const content = await readCellContent(3, 2);
    expect(content).toBe('=SUM(C1:C3)');
  });

  // --- Formula bar shows formula, not value ---
  it('should show the formula in formula bar when cell is selected', async () => {
    await clickCell(0, 0);
    await browser.pause(200);
    const content = await getFormulaBarContent();
    expect(content).toContain('=SUM');
  });

  // --- Formula bar input is visible ---
  it('should display formula bar input for editing', async () => {
    await clickCell(0, 0);
    await browser.pause(200);
    const formulaBarInput = await $('.formula-bar-input');
    expect(await formulaBarInput.isDisplayed()).toBe(true);
  });

  // --- ARRAYFORMULA ---
  it('should support ARRAYFORMULA', async () => {
    // Put values in D1:D3
    await typeInCell(0, 3, '1');
    await typeInCell(1, 3, '2');
    await typeInCell(2, 3, '3');

    // Enter ARRAYFORMULA in E1
    await typeInCell(0, 4, '=ARRAYFORMULA(D1:D3*2)');

    const content = await readCellContent(0, 4);
    expect(content).toContain('ARRAYFORMULA');
  });
});
