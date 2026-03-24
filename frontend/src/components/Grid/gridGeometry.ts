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
 * Hidden columns are skipped (zero width) so they don't occupy space.
 */
export function getColX(
  colWidths: Map<number, number>,
  col: number,
  hiddenCols?: Set<number>,
): number {
  let x = col * DEFAULT_COL_WIDTH;
  colWidths.forEach((w, c) => {
    if (c < col) {
      x += w - DEFAULT_COL_WIDTH;
    }
  });
  // Subtract the width of hidden columns before this column.
  if (hiddenCols) {
    hiddenCols.forEach((c) => {
      if (c < col) {
        x -= colWidths.get(c) ?? DEFAULT_COL_WIDTH;
      }
    });
  }
  return x;
}

/**
 * Return the y-offset (in content coordinates) of the top edge of `row`.
 * Hidden rows are skipped (zero height) so they don't occupy space.
 */
export function getRowY(
  rowHeights: Map<number, number>,
  row: number,
  hiddenRows?: Set<number>,
): number {
  let y = row * DEFAULT_ROW_HEIGHT;
  rowHeights.forEach((h, r) => {
    if (r < row) {
      y += h - DEFAULT_ROW_HEIGHT;
    }
  });
  // Subtract the height of hidden rows before this row.
  if (hiddenRows) {
    hiddenRows.forEach((r) => {
      if (r < row) {
        y -= rowHeights.get(r) ?? DEFAULT_ROW_HEIGHT;
      }
    });
  }
  return y;
}

/** Find which column a content-x coordinate falls in (skipping hidden). */
export function colAtX(
  colWidths: Map<number, number>,
  contentX: number,
  hiddenCols?: Set<number>,
): number {
  let col = Math.floor(contentX / DEFAULT_COL_WIDTH);
  col = Math.max(0, Math.min(col, TOTAL_COLS - 1));
  while (col > 0 && getColX(colWidths, col, hiddenCols) > contentX) col--;
  while (col < TOTAL_COLS - 1 && getColX(colWidths, col + 1, hiddenCols) <= contentX)
    col++;
  // Skip hidden columns forward to nearest visible column
  if (hiddenCols) {
    while (col < TOTAL_COLS - 1 && hiddenCols.has(col)) col++;
  }
  return col;
}

/** Find which row a content-y coordinate falls in (skipping hidden). */
export function rowAtY(
  rowHeights: Map<number, number>,
  contentY: number,
  hiddenRows?: Set<number>,
): number {
  let row = Math.floor(contentY / DEFAULT_ROW_HEIGHT);
  row = Math.max(0, Math.min(row, TOTAL_ROWS - 1));
  while (row > 0 && getRowY(rowHeights, row, hiddenRows) > contentY) row--;
  while (row < TOTAL_ROWS - 1 && getRowY(rowHeights, row + 1, hiddenRows) <= contentY)
    row++;
  // Skip hidden rows forward to nearest visible row
  if (hiddenRows) {
    while (row < TOTAL_ROWS - 1 && hiddenRows.has(row)) row++;
  }
  return row;
}
