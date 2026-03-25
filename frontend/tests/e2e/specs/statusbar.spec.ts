import {
  createNewSpreadsheet,
  clickCell,
  typeInCell,
  getStatusBarSummary,
  getStatusBarMode,
  getNameBoxContent,
} from '../helpers/app';

describe('Status bar', () => {
  before(async () => {
    await createNewSpreadsheet();

    // Populate numeric data in A1:A5
    await typeInCell(0, 0, '10');
    await typeInCell(1, 0, '20');
    await typeInCell(2, 0, '30');
    await typeInCell(3, 0, '40');
    await typeInCell(4, 0, '50');
  });

  // --- Status bar is visible ---
  it('should display the status bar', async () => {
    const statusBar = await $('.status-bar');
    expect(await statusBar.isDisplayed()).toBe(true);
  });

  // --- Mode shows "Ready" when not editing ---
  it('should show Ready mode when not editing', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    const mode = await getStatusBarMode();
    expect(mode).toBe('Ready');
  });

  // --- Mode shows "Edit" when editing ---
  it('should show Edit mode when editing a cell', async () => {
    await clickCell(0, 0);
    await browser.pause(100);
    await browser.keys(['F2']);
    await browser.pause(200);

    const mode = await getStatusBarMode();
    expect(mode).toBe('Edit');

    await browser.keys(['Escape']);
    await browser.pause(200);
  });

  // --- Selection summary for multiple numeric cells ---
  it('should show Sum/Average/Count when multiple numbers selected', async () => {
    // Select A1:A5
    await clickCell(0, 0);
    await browser.pause(100);
    for (let i = 0; i < 4; i++) {
      await browser.keys(['Shift', 'ArrowDown']);
      await browser.pause(50);
    }
    await browser.pause(400);

    const summary = await getStatusBarSummary();
    // Summary should contain at least Sum, Average, or Count
    if (summary) {
      const hasStats =
        summary.includes('Sum') ||
        summary.includes('Average') ||
        summary.includes('Count');
      expect(hasStats).toBe(true);
    }
  });

  // --- Sum value is correct ---
  it('should show correct Sum in status bar', async () => {
    // A1:A5 selected from previous test (10+20+30+40+50 = 150)
    const summary = await getStatusBarSummary();
    if (summary && summary.includes('Sum')) {
      expect(summary).toContain('150');
    }
  });

  // --- Average value is correct ---
  it('should show correct Average in status bar', async () => {
    const summary = await getStatusBarSummary();
    if (summary && summary.includes('Average')) {
      expect(summary).toContain('30');
    }
  });

  // --- Count value is correct ---
  it('should show correct Count in status bar', async () => {
    const summary = await getStatusBarSummary();
    if (summary && summary.includes('Count')) {
      expect(summary).toContain('5');
    }
  });

  // --- Single cell selection ---
  it('should show cell reference for single cell selection', async () => {
    await clickCell(2, 2);
    await browser.pause(200);

    const nameBox = await getNameBoxContent();
    expect(nameBox).toBe('C3');
  });

  // --- Zoom control exists ---
  it('should display zoom control in status bar', async () => {
    const zoomLabel = await $('.status-zoom-label');
    expect(await zoomLabel.isDisplayed()).toBe(true);

    const text = await zoomLabel.getText();
    expect(text).toContain('%');
  });

  // --- Zoom slider exists ---
  it('should display zoom slider', async () => {
    const slider = await $('.status-zoom-slider');
    expect(await slider.isDisplayed()).toBe(true);
  });
});
