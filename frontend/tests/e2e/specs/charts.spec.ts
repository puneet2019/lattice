import {
  createNewSpreadsheet,
  clickCell,
  doubleClickCell,
} from '../helpers/app';

describe('Charts', () => {
  before(async () => {
    await createNewSpreadsheet();

    // Populate some data for chart creation: A1:A3 labels, B1:B3 values
    const data: [number, number, string][] = [
      [0, 0, 'Q1'],
      [1, 0, 'Q2'],
      [2, 0, 'Q3'],
      [0, 1, '100'],
      [1, 1, '200'],
      [2, 1, '300'],
    ];

    for (const [row, col, value] of data) {
      await clickCell(row, col);
      await doubleClickCell(row, col);
      await browser.pause(100);
      for (const ch of value) {
        await browser.keys([ch]);
      }
      await browser.keys(['Enter']);
      await browser.pause(200);
    }
  });

  it('should open the chart dialog when clicking Insert Chart', async () => {
    const chartBtn = await $('[aria-label="Insert chart"]');
    await chartBtn.waitForDisplayed({ timeout: 3_000 });
    await chartBtn.click();
    await browser.pause(400);

    const dialog = await $('.chart-dialog');
    expect(await dialog.isDisplayed()).toBe(true);
  });

  it('should display chart type selection', async () => {
    const typeGroup = await $('.chart-dialog-type-group');
    expect(await typeGroup.isDisplayed()).toBe(true);
  });

  it('should have a data range input field', async () => {
    const rangeInput = await $('.chart-dialog-input');
    expect(await rangeInput.isDisplayed()).toBe(true);
  });

  it('should display a chart preview area', async () => {
    const preview = await $('.chart-dialog-preview');
    expect(await preview.isDisplayed()).toBe(true);
  });

  it('should close chart dialog on Cancel', async () => {
    // The first button in the footer is Cancel
    const cancelBtn = await $('.chart-dialog-footer .chart-dialog-btn');
    await cancelBtn.click();
    await browser.pause(300);

    const dialog = await $$('.chart-dialog');
    expect(dialog.length).toBe(0);
  });

  it('should insert a chart and show overlay after clicking Insert', async () => {
    // Re-open chart dialog
    const chartBtn = await $('[aria-label="Insert chart"]');
    await chartBtn.click();
    await browser.pause(400);

    // Click the Insert (primary) button
    const insertBtn = await $('.chart-dialog-btn-primary');
    await insertBtn.click();
    await browser.pause(500);

    // A chart overlay should appear on the grid
    const overlay = await $('.chart-overlay');
    expect(await overlay.isDisplayed()).toBe(true);
  });

  it('should close chart overlay when clicking the close button', async () => {
    const closeBtn = await $('.chart-overlay-close');
    await closeBtn.click();
    await browser.pause(300);

    const overlays = await $$('.chart-overlay');
    expect(overlays.length).toBe(0);
  });
});
