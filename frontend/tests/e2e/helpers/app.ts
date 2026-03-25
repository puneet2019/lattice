/**
 * E2E test helpers for Lattice spreadsheet.
 *
 * Grid geometry constants and utility functions for interacting with the
 * canvas-based spreadsheet UI via WebdriverIO.
 */

// ---------------------------------------------------------------------------
// Grid geometry constants (must match frontend/src/components/Grid/constants.ts)
// ---------------------------------------------------------------------------

/** Default column width in pixels. */
export const DEFAULT_COL_WIDTH = 100;

/** Default row height in pixels. */
export const DEFAULT_ROW_HEIGHT = 21;

/** Height of the column header row in pixels. */
export const HEADER_HEIGHT = 24;

/** Width of the row number gutter in pixels. */
export const ROW_NUMBER_WIDTH = 50;

/** Total number of columns (A..ZZ). */
export const TOTAL_COLS = 702;

/** Total number of rows. */
export const TOTAL_ROWS = 10_000;

// ---------------------------------------------------------------------------
// Pixel coordinate helpers
// ---------------------------------------------------------------------------

/**
 * Compute the pixel center of a cell at the given (row, col).
 * Row and col are 0-based. The returned coordinates are relative to the
 * top-left corner of the canvas element.
 */
export function cellCenter(
  row: number,
  col: number,
): { x: number; y: number } {
  const x = ROW_NUMBER_WIDTH + col * DEFAULT_COL_WIDTH + DEFAULT_COL_WIDTH / 2;
  const y = HEADER_HEIGHT + row * DEFAULT_ROW_HEIGHT + DEFAULT_ROW_HEIGHT / 2;
  return { x, y };
}

// ---------------------------------------------------------------------------
// WebdriverIO interaction helpers
// ---------------------------------------------------------------------------

/**
 * Wait for the virtual grid canvas to be present in the DOM and visible.
 */
export async function waitForGrid(): Promise<void> {
  const canvas = await $('canvas');
  await canvas.waitForDisplayed({ timeout: 10_000 });
}

/**
 * Click on a cell at the given (row, col) on the canvas.
 * Row and col are 0-based.
 */
export async function clickCell(row: number, col: number): Promise<void> {
  const canvas = await $('canvas');
  const { x, y } = cellCenter(row, col);
  await canvas.click({ x, y });
}

/**
 * Double-click on a cell to enter edit mode.
 */
export async function doubleClickCell(
  row: number,
  col: number,
): Promise<void> {
  const canvas = await $('canvas');
  const { x, y } = cellCenter(row, col);
  await canvas.doubleClick({ x, y });
}

/**
 * Click the "New blank spreadsheet" button on the welcome screen.
 * This transitions the app from the welcome screen to the grid view.
 */
export async function createNewSpreadsheet(): Promise<void> {
  const btn = await $('button.welcome-action-primary');
  await btn.waitForDisplayed({ timeout: 5_000 });
  await btn.click();
  // Wait for the grid canvas to appear
  await waitForGrid();
}

/**
 * Read the current text content of the formula bar.
 * The formula bar is a contentEditable div with class "formula-bar-input".
 */
export async function getFormulaBarContent(): Promise<string> {
  const el = await $('.formula-bar-input');
  return el.getText();
}

/**
 * Read the current cell reference displayed in the name box.
 * When not editing, it renders as a span inside .formula-bar-cell-ref.
 * When editing, it renders as an input.formula-bar-name-input.
 */
export async function getNameBoxContent(): Promise<string> {
  // Check if the name box input is currently visible (editing mode)
  const input = await $('.formula-bar-name-input');
  if (await input.isExisting()) {
    return input.getValue();
  }
  // Otherwise read the span text inside the cell-ref container
  const span = await $('.formula-bar-cell-ref span');
  return span.getText();
}

/**
 * Wait for a specific element to contain expected text.
 */
export async function waitForText(
  selector: string,
  text: string,
  timeout = 5_000,
): Promise<void> {
  const el = await $(selector);
  await el.waitUntil(
    async function () {
      const content = await this.getText();
      return content.includes(text);
    },
    { timeout, timeoutMsg: `Expected "${selector}" to contain "${text}"` },
  );
}

// ---------------------------------------------------------------------------
// Cell typing helpers
// ---------------------------------------------------------------------------

/**
 * Type a value into a cell and commit with Enter.
 * Handles click, double-click to enter edit mode, typing, and Enter.
 */
export async function typeInCell(
  row: number,
  col: number,
  value: string,
): Promise<void> {
  await clickCell(row, col);
  await doubleClickCell(row, col);
  await browser.pause(100);
  const chars = value.split('');
  await browser.keys([...chars, 'Enter']);
  await browser.pause(300);
}

/**
 * Click a cell and read the formula bar content.
 * Returns the formula bar text for the given cell.
 */
export async function readCellContent(
  row: number,
  col: number,
): Promise<string> {
  await clickCell(row, col);
  await browser.pause(200);
  return getFormulaBarContent();
}

// ---------------------------------------------------------------------------
// Toolbar helpers
// ---------------------------------------------------------------------------

/**
 * Check if a toolbar button identified by aria-label has the "active" class.
 */
export async function isToolbarButtonActive(
  ariaLabel: string,
): Promise<boolean> {
  const btn = await $(`[aria-label="${ariaLabel}"]`);
  const cls = await btn.getAttribute('class');
  return (cls ?? '').includes('active');
}

/**
 * Click a toolbar button by aria-label.
 */
export async function clickToolbarButton(
  ariaLabel: string,
): Promise<void> {
  const btn = await $(`[aria-label="${ariaLabel}"]`);
  await btn.waitForDisplayed({ timeout: 3_000 });
  await btn.click();
  await browser.pause(200);
}

// ---------------------------------------------------------------------------
// Status bar helpers
// ---------------------------------------------------------------------------

/**
 * Read the text content of the status bar selection summary.
 */
export async function getStatusBarSummary(): Promise<string> {
  const el = await $('.status-selection-summary');
  if (!(await el.isExisting())) return '';
  return el.getText();
}

/**
 * Read the mode indicator text from the status bar (e.g. "Ready" or "Edit").
 */
export async function getStatusBarMode(): Promise<string> {
  const el = await $('.status-mode');
  return el.getText();
}

// ---------------------------------------------------------------------------
// Column / Row header click helpers
// ---------------------------------------------------------------------------

/**
 * Click a column header (0-based column index).
 */
export async function clickColumnHeader(col: number): Promise<void> {
  const canvas = await $('canvas');
  const x = ROW_NUMBER_WIDTH + col * DEFAULT_COL_WIDTH + DEFAULT_COL_WIDTH / 2;
  const y = HEADER_HEIGHT / 2;
  await canvas.click({ x, y });
  await browser.pause(300);
}

/**
 * Click a row number header (0-based row index).
 */
export async function clickRowHeader(row: number): Promise<void> {
  const canvas = await $('canvas');
  const x = ROW_NUMBER_WIDTH / 2;
  const y = HEADER_HEIGHT + row * DEFAULT_ROW_HEIGHT + DEFAULT_ROW_HEIGHT / 2;
  await canvas.click({ x, y });
  await browser.pause(300);
}
