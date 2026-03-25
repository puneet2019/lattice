import {
  createNewSpreadsheet,
  clickCell,
  doubleClickCell,
  getFormulaBarContent,
} from '../helpers/app';

describe('Keyboard shortcuts', () => {
  before(async () => {
    await createNewSpreadsheet();
  });

  it('should toggle bold with Cmd+B', async () => {
    await clickCell(0, 0);
    await doubleClickCell(0, 0);
    await browser.pause(100);
    await browser.keys(['H', 'i', 'Enter']);
    await browser.pause(200);
    await clickCell(0, 0);
    await browser.pause(200);

    // Toggle bold on
    await browser.keys(['Meta', 'b']);
    await browser.pause(200);

    const boldBtn = await $('[aria-label="Bold"]');
    const classOn = await boldBtn.getAttribute('class');
    expect(classOn).toContain('active');

    // Toggle bold off
    await browser.keys(['Meta', 'b']);
    await browser.pause(200);

    const classOff = await boldBtn.getAttribute('class');
    expect(classOff).not.toContain('active');
  });

  it('should undo with Cmd+Z', async () => {
    // Type something in B1
    await clickCell(0, 1);
    await doubleClickCell(0, 1);
    await browser.pause(100);
    await browser.keys(['U', 'n', 'd', 'o', 'Enter']);
    await browser.pause(300);

    // Delete the content
    await clickCell(0, 1);
    await browser.pause(100);
    await browser.keys(['Delete']);
    await browser.pause(300);

    // Undo the delete
    await browser.keys(['Meta', 'z']);
    await browser.pause(300);

    await clickCell(0, 1);
    await browser.pause(200);
    const content = await getFormulaBarContent();
    expect(content).toBe('Undo');
  });

  it('should redo with Cmd+Shift+Z', async () => {
    // Undo again (removes "Undo" text)
    await browser.keys(['Meta', 'z']);
    await browser.pause(300);

    // Redo to restore
    await browser.keys(['Meta', 'Shift', 'z']);
    await browser.pause(300);

    await clickCell(0, 1);
    await browser.pause(200);
    const content = await getFormulaBarContent();
    expect(content).toBe('Undo');
  });

  it('should open find bar with Cmd+F', async () => {
    await browser.keys(['Meta', 'f']);
    await browser.pause(300);

    const findBar = await $('.find-bar');
    const displayed = await findBar.isDisplayed();
    expect(displayed).toBe(true);

    // Close it with Escape
    await browser.keys(['Escape']);
    await browser.pause(200);
  });

  it('should open keyboard shortcuts dialog with Cmd+/', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await browser.keys(['Meta', '/']);
    await browser.pause(300);

    const dialog = await $('.kbd-shortcuts-dialog');
    const displayed = await dialog.isDisplayed();
    expect(displayed).toBe(true);

    // Close with Escape
    await browser.keys(['Escape']);
    await browser.pause(200);
  });

  it('should open format cells dialog with Cmd+1', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await browser.keys(['Meta', '1']);
    await browser.pause(300);

    const dialog = await $('.format-dialog');
    const displayed = await dialog.isDisplayed();
    expect(displayed).toBe(true);

    // Close with Escape
    await browser.keys(['Escape']);
    await browser.pause(200);
  });

  it('should open print preview with Cmd+P', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await browser.keys(['Meta', 'p']);
    await browser.pause(300);

    const dialog = await $('.print-preview-dialog');
    const displayed = await dialog.isDisplayed();
    expect(displayed).toBe(true);

    // Close it
    const closeBtn = await $('.print-preview-close');
    await closeBtn.click();
    await browser.pause(200);
  });

  it('should enter edit mode with F2', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    await browser.keys(['F2']);
    await browser.pause(200);

    // In edit mode, the cell editor textarea should be visible
    const editor = await $('textarea');
    const exists = await editor.isExisting();
    expect(exists).toBe(true);

    // Cancel with Escape
    await browser.keys(['Escape']);
    await browser.pause(200);
  });

  it('should cancel editing with Escape', async () => {
    // Enter edit mode and type something
    await clickCell(0, 0);
    await browser.pause(100);
    await browser.keys(['F2']);
    await browser.pause(100);
    await browser.keys(['X', 'X', 'X']);
    await browser.pause(100);

    // Cancel with Escape -- should discard the typed text
    await browser.keys(['Escape']);
    await browser.pause(200);

    await clickCell(0, 0);
    await browser.pause(200);
    const content = await getFormulaBarContent();
    // Should still show original content, not "XXX" appended
    expect(content).not.toContain('XXX');
  });
});
