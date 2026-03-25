import {
  createNewSpreadsheet,
  clickCell,
  doubleClickCell,
} from '../helpers/app';

describe('Formatting', () => {
  before(async () => {
    await createNewSpreadsheet();

    // Type some text into A1 so formatting actions have content to act on.
    await clickCell(0, 0);
    await doubleClickCell(0, 0);
    await browser.pause(100);
    await browser.keys(['T', 'e', 's', 't', 'Enter']);
    await browser.pause(300);
    await clickCell(0, 0);
    await browser.pause(200);
  });

  it('should toggle Bold button', async () => {
    const boldBtn = await $('[aria-label="Bold"]');
    await boldBtn.waitForDisplayed({ timeout: 3_000 });

    await boldBtn.click();
    await browser.pause(200);

    // Clicking Bold should add an "active" class or similar indicator
    const classAfterOn = await boldBtn.getAttribute('class');
    expect(classAfterOn).toContain('active');

    // Toggle off
    await boldBtn.click();
    await browser.pause(200);

    const classAfterOff = await boldBtn.getAttribute('class');
    expect(classAfterOff).not.toContain('active');
  });

  it('should toggle Italic button', async () => {
    const btn = await $('[aria-label="Italic"]');
    await btn.waitForDisplayed({ timeout: 3_000 });

    await btn.click();
    await browser.pause(200);

    const classOn = await btn.getAttribute('class');
    expect(classOn).toContain('active');

    await btn.click();
    await browser.pause(200);

    const classOff = await btn.getAttribute('class');
    expect(classOff).not.toContain('active');
  });

  it('should toggle Underline button', async () => {
    const btn = await $('[aria-label="Underline"]');
    await btn.waitForDisplayed({ timeout: 3_000 });

    await btn.click();
    await browser.pause(200);

    const classOn = await btn.getAttribute('class');
    expect(classOn).toContain('active');

    await btn.click();
    await browser.pause(200);

    const classOff = await btn.getAttribute('class');
    expect(classOff).not.toContain('active');
  });

  it('should change font size via dropdown', async () => {
    const fontSizeSelect = await $('[aria-label="Font size"]');
    await fontSizeSelect.waitForDisplayed({ timeout: 3_000 });

    // Read the initial value
    const initialVal = await fontSizeSelect.getValue();

    // Change font size to 14
    await fontSizeSelect.selectByAttribute('value', '14');
    await browser.pause(300);

    const newVal = await fontSizeSelect.getValue();
    expect(newVal).toBe('14');
  });

  it('should display the text color picker button', async () => {
    const colorBtn = await $('[aria-label="Text color"]');
    await colorBtn.waitForDisplayed({ timeout: 3_000 });

    const displayed = await colorBtn.isDisplayed();
    expect(displayed).toBe(true);
  });

  it('should display the fill color picker button', async () => {
    const fillBtn = await $('[aria-label="Fill color"]');
    await fillBtn.waitForDisplayed({ timeout: 3_000 });

    const displayed = await fillBtn.isDisplayed();
    expect(displayed).toBe(true);
  });

  it('should display the borders dropdown button', async () => {
    const bordersBtn = await $('[aria-label="Borders"]');
    await bordersBtn.waitForDisplayed({ timeout: 3_000 });

    const displayed = await bordersBtn.isDisplayed();
    expect(displayed).toBe(true);
  });

  it('should clear formatting with Cmd+Backslash', async () => {
    // First, apply bold
    const boldBtn = await $('[aria-label="Bold"]');
    await boldBtn.click();
    await browser.pause(200);

    const classOn = await boldBtn.getAttribute('class');
    expect(classOn).toContain('active');

    // Clear formatting with Cmd+\
    await browser.keys(['Meta', '\\']);
    await browser.pause(300);

    // Re-select the cell to refresh toolbar state
    await clickCell(0, 0);
    await browser.pause(200);

    const classOff = await boldBtn.getAttribute('class');
    expect(classOff).not.toContain('active');
  });
});
