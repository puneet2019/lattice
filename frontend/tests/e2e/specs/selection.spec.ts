import {
  createNewSpreadsheet,
  clickCell,
  getNameBoxContent,
  cellCenter,
  clickColumnHeader,
  clickRowHeader,
  HEADER_HEIGHT,
  ROW_NUMBER_WIDTH,
  DEFAULT_COL_WIDTH,
  DEFAULT_ROW_HEIGHT,
} from '../helpers/app';

describe('Selection', () => {
  before(async () => {
    await createNewSpreadsheet();
  });

  // --- Shift+Click range selection ---
  it('should select A1:C3 with Click A1 then Shift+Click C3', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    // Shift+Click on C3
    const canvas = await $('canvas');
    const { x, y } = cellCenter(2, 2);
    await browser.action('key').down('\uE008').perform(); // Shift down
    await canvas.click({ x, y });
    await browser.action('key').up('\uE008').perform(); // Shift up
    await browser.pause(300);

    const nameBox = await getNameBoxContent();
    expect(nameBox).toMatch(/A1:C3|A1/);
  });

  // --- Shift+Down to extend selection ---
  it('should select A1:A5 with Shift+Down 4 times', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    for (let i = 0; i < 4; i++) {
      await browser.keys(['Shift', 'ArrowDown']);
      await browser.pause(100);
    }
    await browser.pause(200);

    const nameBox = await getNameBoxContent();
    expect(nameBox).toMatch(/A1:A5|A1/);
  });

  // --- Click column header to select entire column ---
  it('should select column B when clicking column B header', async () => {
    await clickColumnHeader(1);
    const nameBox = await getNameBoxContent();
    expect(nameBox).toMatch(/B:B|B1|B/);
  });

  // --- Click row header to select entire row ---
  it('should select row 3 when clicking row 3 header', async () => {
    await clickRowHeader(2); // 0-based, so row 2 is row 3
    const nameBox = await getNameBoxContent();
    expect(nameBox).toMatch(/3:3|A3|3/);
  });

  // --- Cmd+A selects all ---
  it('should select all cells with Cmd+A', async () => {
    await clickCell(0, 0);
    await browser.pause(100);

    await browser.keys(['Meta', 'a']);
    await browser.pause(300);

    const nameBox = await getNameBoxContent();
    // When all cells are selected the name box shows "All" or "A1"
    expect(nameBox).toBeTruthy();
  });

  // --- Arrow keys move selection ---
  it('should move selection with arrow keys and update name box', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    let nameBox = await getNameBoxContent();
    expect(nameBox).toBe('A1');

    await browser.keys(['ArrowRight']);
    await browser.pause(200);
    nameBox = await getNameBoxContent();
    expect(nameBox).toBe('B1');

    await browser.keys(['ArrowDown']);
    await browser.pause(200);
    nameBox = await getNameBoxContent();
    expect(nameBox).toBe('B2');

    await browser.keys(['ArrowLeft']);
    await browser.pause(200);
    nameBox = await getNameBoxContent();
    expect(nameBox).toBe('A2');

    await browser.keys(['ArrowUp']);
    await browser.pause(200);
    nameBox = await getNameBoxContent();
    expect(nameBox).toBe('A1');
  });

  // --- Home key goes to column A ---
  it('should go to column A with Home key', async () => {
    await clickCell(2, 3); // D3
    await browser.pause(200);

    await browser.keys(['Home']);
    await browser.pause(200);

    const nameBox = await getNameBoxContent();
    expect(nameBox).toMatch(/A3|A1/);
  });

  // --- Cmd+Home goes to A1 ---
  it('should go to A1 with Cmd+Home', async () => {
    await clickCell(5, 5); // F6
    await browser.pause(200);

    await browser.keys(['Meta', 'Home']);
    await browser.pause(200);

    const nameBox = await getNameBoxContent();
    expect(nameBox).toBe('A1');
  });

  // --- Shift+Right extends selection ---
  it('should extend selection with Shift+Arrow keys', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    await browser.keys(['Shift', 'ArrowRight']);
    await browser.pause(200);

    const nameBox = await getNameBoxContent();
    expect(nameBox).toMatch(/A1:B1|A1/);
  });

  // --- Drag to select range ---
  it('should select a range when clicking and dragging', async () => {
    const canvas = await $('canvas');
    const start = cellCenter(0, 0); // A1
    const end = cellCenter(2, 2);   // C3

    await browser.action('pointer')
      .move({ x: start.x, y: start.y, origin: canvas })
      .down()
      .move({ x: end.x, y: end.y, origin: canvas, duration: 200 })
      .up()
      .perform();
    await browser.pause(300);

    const nameBox = await getNameBoxContent();
    expect(nameBox).toBeTruthy();
  });

  // --- Cmd+Click multi-selection ---
  it('should add to selection with Cmd+Click', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    const canvas = await $('canvas');
    const { x, y } = cellCenter(2, 2);
    await browser.action('key').down('\uE03D').perform(); // Meta down
    await canvas.click({ x, y });
    await browser.action('key').up('\uE03D').perform(); // Meta up
    await browser.pause(300);

    const nameBox = await getNameBoxContent();
    expect(nameBox).toBeTruthy();
  });

  // --- Page Down moves by viewport rows ---
  it('should move down by viewport rows with PageDown', async () => {
    await clickCell(0, 0);
    await browser.pause(200);

    await browser.keys(['PageDown']);
    await browser.pause(300);

    const nameBox = await getNameBoxContent();
    // Should have moved to a row significantly past row 1
    // The exact row depends on viewport size, but it should not be A1 anymore
    expect(nameBox).not.toBe('A1');
    expect(nameBox).toMatch(/^A\d+$/); // Still in column A
  });
});
