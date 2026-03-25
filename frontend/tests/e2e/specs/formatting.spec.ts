import {
  createNewSpreadsheet,
  clickCell,
  doubleClickCell,
  typeInCell,
  readCellContent,
  isToolbarButtonActive,
  clickToolbarButton,
} from '../helpers/app';

describe('Formatting', () => {
  before(async () => {
    await createNewSpreadsheet();

    // Type text into A1 and B1 for formatting tests
    await typeInCell(0, 0, 'Test');
    await typeInCell(0, 1, 'Test');
    await clickCell(0, 0);
    await browser.pause(200);
  });

  // --- Bold applies and persists ---
  it('should apply Bold and verify it is active on re-select', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await clickToolbarButton('Bold');
    expect(await isToolbarButtonActive('Bold')).toBe(true);

    // Click away and back to verify persistence
    await clickCell(1, 1);
    await browser.pause(200);
    await clickCell(0, 0);
    await browser.pause(200);

    expect(await isToolbarButtonActive('Bold')).toBe(true);
  });

  // --- Italic applies and persists ---
  it('should apply Italic to B1 and verify it is active on re-select', async () => {
    await clickCell(0, 1);
    await browser.pause(100);

    await clickToolbarButton('Italic');
    expect(await isToolbarButtonActive('Italic')).toBe(true);

    // Click away and back
    await clickCell(1, 1);
    await browser.pause(200);
    await clickCell(0, 1);
    await browser.pause(200);

    expect(await isToolbarButtonActive('Italic')).toBe(true);
  });

  // --- Toggle Bold off ---
  it('should toggle Bold off', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    // Bold is currently on from earlier test, toggle off
    await clickToolbarButton('Bold');
    expect(await isToolbarButtonActive('Bold')).toBe(false);
  });

  // --- Toggle Italic off ---
  it('should toggle Italic off', async () => {
    await clickCell(0, 1);
    await browser.pause(100);

    await clickToolbarButton('Italic');
    expect(await isToolbarButtonActive('Italic')).toBe(false);
  });

  // --- Underline toggle ---
  it('should toggle Underline button', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await clickToolbarButton('Underline');
    expect(await isToolbarButtonActive('Underline')).toBe(true);

    await clickToolbarButton('Underline');
    expect(await isToolbarButtonActive('Underline')).toBe(false);
  });

  // --- Font size dropdown ---
  it('should change font size via dropdown', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    const fontSizeSelect = await $('[aria-label="Font size"]');
    await fontSizeSelect.waitForDisplayed({ timeout: 3_000 });

    await fontSizeSelect.selectByAttribute('value', '18');
    await browser.pause(300);

    const newVal = await fontSizeSelect.getValue();
    expect(newVal).toBe('18');
  });

  // --- Text color picker button exists ---
  it('should display the text color picker button', async () => {
    const colorBtn = await $('[aria-label="Text color"]');
    await colorBtn.waitForDisplayed({ timeout: 3_000 });
    expect(await colorBtn.isDisplayed()).toBe(true);
  });

  // --- Fill color picker button exists ---
  it('should display the fill color picker button', async () => {
    const fillBtn = await $('[aria-label="Fill color"]');
    await fillBtn.waitForDisplayed({ timeout: 3_000 });
    expect(await fillBtn.isDisplayed()).toBe(true);
  });

  // --- Borders dropdown button exists ---
  it('should display the borders dropdown button', async () => {
    const bordersBtn = await $('[aria-label="Borders"]');
    await bordersBtn.waitForDisplayed({ timeout: 3_000 });
    expect(await bordersBtn.isDisplayed()).toBe(true);
  });

  // --- Bold range: select A1:B1, apply Bold to both ---
  it('should apply Bold to a selected range', async () => {
    // Select A1:B1
    await clickCell(0, 0);
    await browser.pause(100);
    await browser.keys(['Shift', 'ArrowRight']);
    await browser.pause(200);

    await clickToolbarButton('Bold');

    // Verify both cells show Bold active
    await clickCell(0, 0);
    await browser.pause(200);
    expect(await isToolbarButtonActive('Bold')).toBe(true);

    await clickCell(0, 1);
    await browser.pause(200);
    expect(await isToolbarButtonActive('Bold')).toBe(true);
  });

  // --- Clear formatting with Cmd+\ ---
  it('should clear formatting with Cmd+Backslash', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    // Bold should be active from previous test
    expect(await isToolbarButtonActive('Bold')).toBe(true);

    await browser.keys(['Meta', '\\']);
    await browser.pause(300);

    // Re-select to refresh toolbar state
    await clickCell(0, 0);
    await browser.pause(200);

    expect(await isToolbarButtonActive('Bold')).toBe(false);
  });

  // --- Keyboard shortcut Cmd+B ---
  it('should toggle Bold via Cmd+B shortcut', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await browser.keys(['Meta', 'b']);
    await browser.pause(200);
    expect(await isToolbarButtonActive('Bold')).toBe(true);

    await browser.keys(['Meta', 'b']);
    await browser.pause(200);
    expect(await isToolbarButtonActive('Bold')).toBe(false);
  });

  // --- Keyboard shortcut Cmd+I ---
  it('should toggle Italic via Cmd+I shortcut', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await browser.keys(['Meta', 'i']);
    await browser.pause(200);
    expect(await isToolbarButtonActive('Italic')).toBe(true);

    await browser.keys(['Meta', 'i']);
    await browser.pause(200);
    expect(await isToolbarButtonActive('Italic')).toBe(false);
  });

  // --- Keyboard shortcut Cmd+U ---
  it('should toggle Underline via Cmd+U shortcut', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await browser.keys(['Meta', 'u']);
    await browser.pause(200);
    expect(await isToolbarButtonActive('Underline')).toBe(true);

    await browser.keys(['Meta', 'u']);
    await browser.pause(200);
    expect(await isToolbarButtonActive('Underline')).toBe(false);
  });
});
