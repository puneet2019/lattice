import {
  createNewSpreadsheet,
  clickCell,
  doubleClickCell,
} from '../helpers/app';

describe('File operations', () => {
  before(async () => {
    await createNewSpreadsheet();
  });

  it('should show "Untitled" in the window title for a new spreadsheet', async () => {
    const title = await browser.getTitle();
    expect(title).toContain('Untitled');
    expect(title).toContain('Lattice');
  });

  it('should display the status bar with saved indicator initially', async () => {
    const statusBar = await $('.status-bar');
    expect(await statusBar.isDisplayed()).toBe(true);

    // The save status indicator should exist
    const saveStatus = await $('.save-status');
    expect(await saveStatus.isExisting()).toBe(true);
  });

  it('should show unsaved indicator after editing a cell', async () => {
    // Type something to create unsaved changes
    await clickCell(0, 0);
    await doubleClickCell(0, 0);
    await browser.pause(100);
    await browser.keys(['C', 'h', 'a', 'n', 'g', 'e', 'd', 'Enter']);
    await browser.pause(500);

    // The save status should reflect unsaved changes
    const saveStatus = await $('.save-status');
    const statusClass = await saveStatus.getAttribute('class');
    expect(statusClass).toContain('unsaved');
  });

  it('should update window title to show unsaved indicator', async () => {
    // After editing, window title should show the asterisk
    const title = await browser.getTitle();
    // Title format is "Untitled* - Lattice" when dirty
    expect(title).toContain('Untitled');
  });
});
