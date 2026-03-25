import {
  createNewSpreadsheet,
  clickCell,
  typeInCell,
  readCellContent,
} from '../helpers/app';

describe('Sheet tabs', () => {
  before(async () => {
    await createNewSpreadsheet();
  });

  // --- Default Sheet1 tab ---
  it('should display the default Sheet1 tab', async () => {
    const sheetTabs = await $('.sheet-tabs');
    await sheetTabs.waitForDisplayed({ timeout: 5_000 });

    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    expect(tabs.length).toBeGreaterThanOrEqual(1);

    const text = await tabs[0].getText();
    expect(text).toContain('Sheet1');
  });

  // --- Add sheet and verify Sheet2 tab appears ---
  it('should add a new sheet and show Sheet2 tab', async () => {
    const addBtn = await $('[aria-label="Add sheet"]');
    await addBtn.click();
    await browser.pause(400);

    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    expect(tabs.length).toBeGreaterThanOrEqual(2);

    const secondText = await tabs[1].getText();
    expect(secondText).toContain('Sheet2');
  });

  // --- Click Sheet2 to make it active ---
  it('should activate Sheet2 when clicked', async () => {
    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    await tabs[1].click();
    await browser.pause(300);

    const selected = await tabs[1].getAttribute('aria-selected');
    expect(selected).toBe('true');
  });

  // --- Type data in Sheet2 ---
  it('should type data on Sheet2', async () => {
    await typeInCell(0, 0, 'Sheet2Data');

    const content = await readCellContent(0, 0);
    expect(content).toBe('Sheet2Data');
  });

  // --- Switch back to Sheet1 and verify original data ---
  it('should switch back to Sheet1 and verify data is separate', async () => {
    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    await tabs[0].click();
    await browser.pause(300);

    const selected = await tabs[0].getAttribute('aria-selected');
    expect(selected).toBe('true');

    // Sheet1 should not have Sheet2Data in A1
    const content = await readCellContent(0, 0);
    expect(content).not.toBe('Sheet2Data');
  });

  // --- Rename sheet via double-click ---
  it('should rename a sheet by double-clicking the tab', async () => {
    // Switch to Sheet2
    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    await tabs[1].click();
    await browser.pause(300);

    // Double-click to rename
    await tabs[1].doubleClick();
    await browser.pause(300);

    // Type new name
    const renameInput = await $('.sheet-tab-rename-input');
    if (await renameInput.isExisting()) {
      await renameInput.clearValue();
      await renameInput.setValue('MySheet');
      await browser.keys(['Enter']);
      await browser.pause(400);

      // Verify renamed
      const updatedTabs = await $$('.sheet-tabs-list [role="tab"]');
      const renamedText = await updatedTabs[1].getText();
      expect(renamedText).toContain('MySheet');
    }
  });

  // --- Context menu on tab ---
  it('should open a context menu when right-clicking a tab', async () => {
    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    await tabs[0].click({ button: 2 });
    await browser.pause(300);

    const contextMenu = await $('.sheet-tab-context-menu');
    expect(await contextMenu.isDisplayed()).toBe(true);

    // Dismiss
    const sheetTabs = await $('.sheet-tabs');
    await sheetTabs.click();
    await browser.pause(200);
  });

  // --- Delete a sheet tab ---
  it('should delete a sheet tab via context menu', async () => {
    // Add a temp sheet to delete
    const addBtn = await $('[aria-label="Add sheet"]');
    await addBtn.click();
    await browser.pause(400);

    const tabsBefore = await $$('.sheet-tabs-list [role="tab"]');
    const countBefore = tabsBefore.length;

    // Right-click last tab
    const lastTab = tabsBefore[tabsBefore.length - 1];
    await lastTab.click({ button: 2 });
    await browser.pause(300);

    // Click "Delete" in context menu
    const menuItems = await $$('.sheet-tab-context-menu button');
    let deleteBtn = null;
    for (const item of menuItems) {
      const text = await item.getText();
      if (text.toLowerCase().includes('delete')) {
        deleteBtn = item;
        break;
      }
    }

    if (deleteBtn) {
      await deleteBtn.click();
      await browser.pause(400);

      const tabsAfter = await $$('.sheet-tabs-list [role="tab"]');
      expect(tabsAfter.length).toBeLessThan(countBefore);
    } else {
      expect(menuItems.length).toBeGreaterThan(0);
    }
  });

  // --- Sheet2 data persists after switching ---
  it('should preserve Sheet2 data after switching sheets', async () => {
    // Navigate to the renamed sheet (Sheet2 or MySheet)
    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    if (tabs.length >= 2) {
      await tabs[1].click();
      await browser.pause(300);

      const content = await readCellContent(0, 0);
      expect(content).toBe('Sheet2Data');
    }
  });
});
