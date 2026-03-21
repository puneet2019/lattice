/**
 * Utility functions for converting between A1-style cell references
 * and zero-based (row, col) coordinates.
 */

/**
 * Parse an A1-style cell reference (e.g. "A1", "B3", "AA1") into
 * zero-based row and column indices.
 *
 * Returns `null` if the reference string is not valid.
 */
export function parse_cell_ref(
  ref: string,
): { row: number; col: number } | null {
  const match = ref.match(/^([A-Za-z]+)(\d+)$/);
  if (!match) return null;

  const letters = match[1].toUpperCase();
  const rowNum = parseInt(match[2], 10);
  if (isNaN(rowNum) || rowNum < 1) return null;

  let col = 0;
  for (let i = 0; i < letters.length; i++) {
    col = col * 26 + (letters.charCodeAt(i) - 64);
  }
  col -= 1; // convert to 0-based

  return { row: rowNum - 1, col };
}

/**
 * Convert a zero-based column index to a letter string.
 * 0 -> "A", 25 -> "Z", 26 -> "AA", 27 -> "AB", ...
 */
export function col_to_letter(col: number): string {
  let result = '';
  let c = col;
  do {
    result = String.fromCharCode(65 + (c % 26)) + result;
    c = Math.floor(c / 26) - 1;
  } while (c >= 0);
  return result;
}

/**
 * Convert zero-based (row, col) to an A1-style cell reference string.
 * (0, 0) -> "A1", (2, 1) -> "B3", (0, 26) -> "AA1"
 */
export function cell_ref_str(row: number, col: number): string {
  return `${col_to_letter(col)}${row + 1}`;
}
