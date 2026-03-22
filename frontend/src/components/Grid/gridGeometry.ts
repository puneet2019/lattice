/**
 * Pure helper functions for computing grid geometry with variable
 * column widths and row heights.
 *
 * These are extracted from VirtualGrid so they can be unit-tested
 * independently of the canvas rendering.
 */

import {
  DEFAULT_COL_WIDTH,
  DEFAULT_ROW_HEIGHT,
  TOTAL_COLS,
  TOTAL_ROWS,
} from './constants';

// -----------------------------------------------------------------------
// Column / Row dimension helpers
// -----------------------------------------------------------------------

/** Return the width of a specific column. */
export function getColWidth(
  colWidths: Map<number, number>,
  col: number,
): number {
  return colWidths.get(col) ?? DEFAULT_COL_WIDTH;
}

/** Return the height of a specific row. */
export function getRowHeight(
  rowHeights: Map<number, number>,
  row: number,
): number {
  return rowHeights.get(row) ?? DEFAULT_ROW_HEIGHT;
}

/**
 * Return the x-offset (in content coordinates) of the left edge of `col`.
 * Uses default widths for all columns except those with custom widths.
 */
export function getColX(
  colWidths: Map<number, number>,
  col: number,
): number {
  let x = col * DEFAULT_COL_WIDTH;
  colWidths.forEach((w, c) => {
    if (c < col) {
      x += w - DEFAULT_COL_WIDTH;
    }
  });
  return x;
}

/**
 * Return the y-offset (in content coordinates) of the top edge of `row`.
 * Uses default heights for all rows except those with custom heights.
 */
export function getRowY(
  rowHeights: Map<number, number>,
  row: number,
): number {
  let y = row * DEFAULT_ROW_HEIGHT;
  rowHeights.forEach((h, r) => {
    if (r < row) {
      y += h - DEFAULT_ROW_HEIGHT;
    }
  });
  return y;
}

/** Find which column a content-x coordinate falls in. */
export function colAtX(
  colWidths: Map<number, number>,
  contentX: number,
): number {
  let col = Math.floor(contentX / DEFAULT_COL_WIDTH);
  col = Math.max(0, Math.min(col, TOTAL_COLS - 1));
  while (col > 0 && getColX(colWidths, col) > contentX) col--;
  while (col < TOTAL_COLS - 1 && getColX(colWidths, col + 1) <= contentX)
    col++;
  return col;
}

/** Find which row a content-y coordinate falls in. */
export function rowAtY(
  rowHeights: Map<number, number>,
  contentY: number,
): number {
  let row = Math.floor(contentY / DEFAULT_ROW_HEIGHT);
  row = Math.max(0, Math.min(row, TOTAL_ROWS - 1));
  while (row > 0 && getRowY(rowHeights, row) > contentY) row--;
  while (row < TOTAL_ROWS - 1 && getRowY(rowHeights, row + 1) <= contentY)
    row++;
  return row;
}
