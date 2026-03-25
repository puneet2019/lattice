import { createNewSpreadsheet, waitForGrid } from '../helpers/app';

describe('Lattice launch', () => {
  it('should open the app window without a white screen', async () => {
    // The app should have rendered _something_ within the root element.
    const root = await $('#root');
    await root.waitForExist({ timeout: 10_000 });

    // The root should contain visible child elements (not an empty white page).
    const children = await root.$$('*');
    expect(children.length).toBeGreaterThan(0);
  });

  it('should render the welcome screen', async () => {
    // The welcome screen shows a "New blank spreadsheet" primary action button.
    const btn = await $('button.welcome-action-primary');
    await btn.waitForDisplayed({ timeout: 5_000 });
    const text = await btn.getText();
    expect(text).toContain('New blank spreadsheet');
  });

  it('should show the grid after clicking "New blank spreadsheet"', async () => {
    await createNewSpreadsheet();

    // The canvas element should now be visible.
    const canvas = await $('canvas');
    const displayed = await canvas.isDisplayed();
    expect(displayed).toBe(true);
  });

  it('should display the toolbar', async () => {
    const toolbar = await $('.toolbar');
    const displayed = await toolbar.isDisplayed();
    expect(displayed).toBe(true);
  });

  it('should display the formula bar', async () => {
    const formulaBar = await $('.formula-bar');
    const displayed = await formulaBar.isDisplayed();
    expect(displayed).toBe(true);
  });

  it('should display the status bar', async () => {
    const statusBar = await $('.status-bar');
    const displayed = await statusBar.isDisplayed();
    expect(displayed).toBe(true);
  });

  it('should display sheet tabs', async () => {
    const sheetTabs = await $('.sheet-tabs');
    const displayed = await sheetTabs.isDisplayed();
    expect(displayed).toBe(true);
  });
});
