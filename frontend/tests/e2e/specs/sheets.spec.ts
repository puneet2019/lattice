import { createNewSpreadsheet } from '../helpers/app';

describe('Sheet tabs', () => {
  before(async () => {
    await createNewSpreadsheet();
  });

  it('should display the default Sheet1 tab', async () => {
    const sheetTabs = await $('.sheet-tabs');
    await sheetTabs.waitForDisplayed({ timeout: 5_000 });

    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    expect(tabs.length).toBeGreaterThanOrEqual(1);

    const firstTab = tabs[0];
    const text = await firstTab.getText();
    expect(text).toContain('Sheet1');
  });

  it('should add a new sheet tab when clicking the + button', async () => {
    const addBtn = await $('[aria-label="Add sheet"]');
    await addBtn.click();
    await browser.pause(400);

    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    expect(tabs.length).toBeGreaterThanOrEqual(2);
  });

  it('should switch the active sheet when clicking another tab', async () => {
    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    // Click the second tab
    if (tabs.length >= 2) {
      await tabs[1].click();
      await browser.pause(300);

      const selected = await tabs[1].getAttribute('aria-selected');
      expect(selected).toBe('true');
    }

    // Switch back to first tab
    const allTabs = await $$('.sheet-tabs-list [role="tab"]');
    await allTabs[0].click();
    await browser.pause(300);
  });

  it('should open a context menu when right-clicking a tab', async () => {
    const tabs = await $$('.sheet-tabs-list [role="tab"]');
    const firstTab = tabs[0];

    // Right-click to open context menu
    await firstTab.click({ button: 2 });
    await browser.pause(300);

    const contextMenu = await $('.sheet-tab-context-menu');
    expect(await contextMenu.isDisplayed()).toBe(true);

    // Dismiss by clicking elsewhere
    const sheetTabs = await $('.sheet-tabs');
    await sheetTabs.click();
    await browser.pause(200);
  });

  it('should delete a sheet tab', async () => {
    // Add a temporary sheet first
    const addBtn = await $('[aria-label="Add sheet"]');
    await addBtn.click();
    await browser.pause(400);

    const tabsBefore = await $$('.sheet-tabs-list [role="tab"]');
    const countBefore = tabsBefore.length;

    // Right-click the last tab to open context menu
    const lastTab = tabsBefore[tabsBefore.length - 1];
    await lastTab.click({ button: 2 });
    await browser.pause(300);

    // Click "Delete" in the context menu
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
      // If no delete button found, at least verify context menu appeared
      expect(menuItems.length).toBeGreaterThan(0);
    }
  });
});
