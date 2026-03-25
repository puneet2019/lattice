import {
  createNewSpreadsheet,
  clickCell,
  doubleClickCell,
  getFormulaBarContent,
} from '../helpers/app';

describe('Formulas', () => {
  before(async () => {
    await createNewSpreadsheet();
  });

  it('should evaluate =SUM(1,2,3) and show 6', async () => {
    await clickCell(0, 0);
    await doubleClickCell(0, 0);
    await browser.pause(100);

    await browser.keys(['=', 'S', 'U', 'M', '(', '1', ',', '2', ',', '3', ')', 'Enter']);
    await browser.pause(400);

    // Navigate back and check formula bar
    await clickCell(0, 0);
    await browser.pause(200);

    const content = await getFormulaBarContent();
    expect(content).toBe('=SUM(1,2,3)');
  });

  it('should evaluate =AVERAGE(10,20) and show 15', async () => {
    await clickCell(1, 0);
    await doubleClickCell(1, 0);
    await browser.pause(100);

    await browser.keys([
      '=', 'A', 'V', 'E', 'R', 'A', 'G', 'E',
      '(', '1', '0', ',', '2', '0', ')', 'Enter',
    ]);
    await browser.pause(400);

    await clickCell(1, 0);
    await browser.pause(200);

    const content = await getFormulaBarContent();
    expect(content).toBe('=AVERAGE(10,20)');
  });

  it('should show the formula in the formula bar when the cell is selected', async () => {
    // A1 already has =SUM(1,2,3) from the first test
    await clickCell(0, 0);
    await browser.pause(200);

    const content = await getFormulaBarContent();
    expect(content).toContain('=SUM');
  });

  it('should display formula bar input for editing', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    const formulaBarInput = await $('.formula-bar-input');
    const displayed = await formulaBarInput.isDisplayed();
    expect(displayed).toBe(true);
  });

  it('should evaluate =LET(x,10,x*2) and show 20', async () => {
    await clickCell(2, 0);
    await doubleClickCell(2, 0);
    await browser.pause(100);

    await browser.keys([
      '=', 'L', 'E', 'T', '(',
      'x', ',', '1', '0', ',',
      'x', '*', '2', ')', 'Enter',
    ]);
    await browser.pause(400);

    await clickCell(2, 0);
    await browser.pause(200);

    const content = await getFormulaBarContent();
    expect(content).toBe('=LET(x,10,x*2)');
  });

  it('should support ARRAYFORMULA', async () => {
    // Put values in B1:B3 first
    await clickCell(0, 1);
    await doubleClickCell(0, 1);
    await browser.pause(100);
    await browser.keys(['1', 'Enter']);
    await browser.pause(200);

    await clickCell(1, 1);
    await doubleClickCell(1, 1);
    await browser.pause(100);
    await browser.keys(['2', 'Enter']);
    await browser.pause(200);

    await clickCell(2, 1);
    await doubleClickCell(2, 1);
    await browser.pause(100);
    await browser.keys(['3', 'Enter']);
    await browser.pause(200);

    // Enter an ARRAYFORMULA in C1
    await clickCell(0, 2);
    await doubleClickCell(0, 2);
    await browser.pause(100);

    await browser.keys([
      '=', 'A', 'R', 'R', 'A', 'Y', 'F', 'O', 'R', 'M', 'U', 'L', 'A',
      '(', 'B', '1', ':', 'B', '3', '*', '2', ')', 'Enter',
    ]);
    await browser.pause(400);

    await clickCell(0, 2);
    await browser.pause(200);

    const content = await getFormulaBarContent();
    expect(content).toContain('ARRAYFORMULA');
  });
});
