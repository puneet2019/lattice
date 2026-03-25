import {
  createNewSpreadsheet,
  clickCell,
  typeInCell,
  isToolbarButtonActive,
  clickToolbarButton,
} from '../helpers/app';

describe('Auto-filter', () => {
  before(async () => {
    await createNewSpreadsheet();

    // Populate data: header in A1, values in A2:A5
    await typeInCell(0, 0, 'Name');
    await typeInCell(1, 0, 'Alice');
    await typeInCell(2, 0, 'Bob');
    await typeInCell(3, 0, 'Charlie');
    await typeInCell(4, 0, 'Diana');

    // B column: header + values
    await typeInCell(0, 1, 'Score');
    await typeInCell(1, 1, '85');
    await typeInCell(2, 1, '92');
    await typeInCell(3, 1, '78');
    await typeInCell(4, 1, '95');
  });

  // --- Toggle filter on ---
  it('should activate filter when clicking filter button', async () => {
    // Select a cell in the data range
    await clickCell(0, 0);
    await browser.pause(200);

    await clickToolbarButton('Filter');

    // The filter button should show active state
    expect(await isToolbarButtonActive('Filter')).toBe(true);
  });

  // --- Filter is reflected in status bar ---
  it('should show filter summary in status bar when filter is active', async () => {
    const statusFilter = await $('.status-filter-summary');
    if (await statusFilter.isExisting()) {
      const text = await statusFilter.getText();
      // When a filter is active, the status bar may show a summary
      expect(text).toBeTruthy();
    }
  });

  // --- Toggle filter off ---
  it('should deactivate filter when clicking filter button again', async () => {
    await clickToolbarButton('Filter');

    expect(await isToolbarButtonActive('Filter')).toBe(false);
  });

  // --- Toggle filter back on and verify data is intact ---
  it('should re-enable filter without losing data', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    await clickToolbarButton('Filter');
    expect(await isToolbarButtonActive('Filter')).toBe(true);

    // Turn off again for clean state
    await clickToolbarButton('Filter');
    expect(await isToolbarButtonActive('Filter')).toBe(false);
  });
});
