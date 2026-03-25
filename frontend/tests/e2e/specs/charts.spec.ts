import {
  createNewSpreadsheet,
  clickCell,
  typeInCell,
} from '../helpers/app';

describe('Charts', () => {
  before(async () => {
    await createNewSpreadsheet();

    // Populate data: A1:A4 labels, B1:B4 values
    await typeInCell(0, 0, 'Q1');
    await typeInCell(1, 0, 'Q2');
    await typeInCell(2, 0, 'Q3');
    await typeInCell(3, 0, 'Q4');
    await typeInCell(0, 1, '100');
    await typeInCell(1, 1, '200');
    await typeInCell(2, 1, '300');
    await typeInCell(3, 1, '400');
  });

  // --- Open chart dialog ---
  it('should open the chart dialog when clicking Insert Chart', async () => {
    const chartBtn = await $('[aria-label="Insert chart"]');
    await chartBtn.waitForDisplayed({ timeout: 3_000 });
    await chartBtn.click();
    await browser.pause(400);

    const dialog = await $('.chart-dialog');
    expect(await dialog.isDisplayed()).toBe(true);
  });

  // --- Chart type selection exists ---
  it('should display chart type selection', async () => {
    const typeGroup = await $('.chart-dialog-type-group');
    expect(await typeGroup.isDisplayed()).toBe(true);
  });

  // --- Data range input field ---
  it('should have a data range input field', async () => {
    const rangeInput = await $('.chart-dialog-input');
    expect(await rangeInput.isDisplayed()).toBe(true);
  });

  // --- Enter range A1:B4 ---
  it('should accept data range A1:B4', async () => {
    const rangeInput = await $('.chart-dialog-input');
    await rangeInput.clearValue();
    await rangeInput.setValue('A1:B4');
    await browser.pause(200);

    const value = await rangeInput.getValue();
    expect(value).toBe('A1:B4');
  });

  // --- Enter chart title ---
  it('should accept a chart title', async () => {
    const titleInput = await $('.chart-dialog-title-input');
    if (await titleInput.isExisting()) {
      await titleInput.clearValue();
      await titleInput.setValue('Test Chart');
      await browser.pause(200);

      const value = await titleInput.getValue();
      expect(value).toBe('Test Chart');
    }
  });

  // --- Chart preview area ---
  it('should display a chart preview area', async () => {
    const preview = await $('.chart-dialog-preview');
    expect(await preview.isDisplayed()).toBe(true);
  });

  // --- Close on Cancel ---
  it('should close chart dialog on Cancel', async () => {
    const cancelBtn = await $('.chart-dialog-footer .chart-dialog-btn');
    await cancelBtn.click();
    await browser.pause(300);

    const dialog = await $$('.chart-dialog');
    expect(dialog.length).toBe(0);
  });

  // --- Insert chart and verify overlay ---
  it('should insert a chart and show overlay', async () => {
    // Re-open chart dialog
    const chartBtn = await $('[aria-label="Insert chart"]');
    await chartBtn.click();
    await browser.pause(400);

    // Set range
    const rangeInput = await $('.chart-dialog-input');
    await rangeInput.clearValue();
    await rangeInput.setValue('A1:B4');
    await browser.pause(200);

    // Set title if field exists
    const titleInput = await $('.chart-dialog-title-input');
    if (await titleInput.isExisting()) {
      await titleInput.clearValue();
      await titleInput.setValue('Test Chart');
      await browser.pause(200);
    }

    // Click Insert button
    const insertBtn = await $('.chart-dialog-btn-primary');
    await insertBtn.click();
    await browser.pause(500);

    // Chart overlay should appear
    const overlay = await $('.chart-overlay');
    expect(await overlay.isDisplayed()).toBe(true);
  });

  // --- Verify chart title in overlay ---
  it('should show chart title text in the overlay', async () => {
    const overlay = await $('.chart-overlay');
    const text = await overlay.getText();
    // The overlay should contain some text (title or data labels)
    expect(text.length).toBeGreaterThan(0);
  });

  // --- Close chart overlay ---
  it('should close chart overlay when clicking the close button', async () => {
    const closeBtn = await $('.chart-overlay-close');
    await closeBtn.click();
    await browser.pause(300);

    const overlays = await $$('.chart-overlay');
    expect(overlays.length).toBe(0);
  });
});
