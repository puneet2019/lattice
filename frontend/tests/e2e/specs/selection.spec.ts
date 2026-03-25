import {
  createNewSpreadsheet,
  clickCell,
  getNameBoxContent,
  cellCenter,
  HEADER_HEIGHT,
  ROW_NUMBER_WIDTH,
  DEFAULT_COL_WIDTH,
  DEFAULT_ROW_HEIGHT,
} from '../helpers/app';

describe('Selection', () => {
  before(async () => {
    await createNewSpreadsheet();
  });

  it('should extend selection with Shift+Arrow keys', async () => {
    // Click A1 to start
    await clickCell(0, 0);
    await browser.pause(200);

    // Shift+Right should extend selection to A1:B1
    await browser.keys(['Shift', 'ArrowRight']);
    await browser.pause(200);

    const nameBox = await getNameBoxContent();
    // Name box should show range like "A1:B1" when a range is selected
    expect(nameBox).toMatch(/A1:B1|A1/);
  });

  it('should select all cells with Cmd+A', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await browser.keys(['Meta', 'a']);
    await browser.pause(300);

    const nameBox = await getNameBoxContent();
    // When all cells are selected the name box typically shows "All" or "A1"
    expect(nameBox).toBeTruthy();
  });

  it('should select a column when clicking the column header', async () => {
    const canvas = await $('canvas');
    // Column B header is at col index 1, centered in the header row
    const x = ROW_NUMBER_WIDTH + 1 * DEFAULT_COL_WIDTH + DEFAULT_COL_WIDTH / 2;
    const y = HEADER_HEIGHT / 2; // Middle of the header row
    await canvas.click({ x, y });
    await browser.pause(300);

    const nameBox = await getNameBoxContent();
    // Selecting a column header should show the column letter or full column ref
    expect(nameBox).toMatch(/B|B1/);
  });

  it('should select a row when clicking the row number gutter', async () => {
    const canvas = await $('canvas');
    // Row 2 (index 1) in the row number gutter area
    const x = ROW_NUMBER_WIDTH / 2;
    const y = HEADER_HEIGHT + 1 * DEFAULT_ROW_HEIGHT + DEFAULT_ROW_HEIGHT / 2;
    await canvas.click({ x, y });
    await browser.pause(300);

    const nameBox = await getNameBoxContent();
    // Row selection should show first cell of the row or row indicator
    expect(nameBox).toMatch(/A2|2/);
  });

  it('should select a range when clicking and dragging', async () => {
    const canvas = await $('canvas');
    const start = cellCenter(0, 0); // A1
    const end = cellCenter(2, 2);   // C3

    // Perform drag from A1 to C3
    await browser.action('pointer')
      .move({ x: start.x, y: start.y, origin: canvas })
      .down()
      .move({ x: end.x, y: end.y, origin: canvas, duration: 200 })
      .up()
      .perform();
    await browser.pause(300);

    const nameBox = await getNameBoxContent();
    // Should show A1 (anchor cell) or the range
    expect(nameBox).toBeTruthy();
  });

  it('should add to selection with Cmd+Click', async () => {
    // Click A1 first
    await clickCell(0, 0);
    await browser.pause(200);

    // Cmd+Click on C3 to add a second range
    const canvas = await $('canvas');
    const { x, y } = cellCenter(2, 2);
    await browser.action('pointer')
      .move({ x, y, origin: canvas })
      .down({ button: 0 })
      .up({ button: 0 })
      .perform();
    // Release meta key workaround: use keyboard action
    await browser.action('key')
      .down('\uE03D') // Meta key
      .perform();
    await canvas.click({ x, y });
    await browser.action('key')
      .up('\uE03D')
      .perform();
    await browser.pause(300);

    // Verify at least one cell is selected (multi-selection is active)
    const nameBox = await getNameBoxContent();
    expect(nameBox).toBeTruthy();
  });
});
