/**
 * Auto-fill utilities: series detection, formula reference adjustment,
 * and pattern-based fill value generation.
 */

// ---------------------------------------------------------------------------
// Named series constants
// ---------------------------------------------------------------------------

export const DAYS_SHORT = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
export const DAYS_LONG = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
export const MONTHS_SHORT = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
export const MONTHS_LONG = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
export const QUARTERS = ['Q1', 'Q2', 'Q3', 'Q4'];

// ---------------------------------------------------------------------------
// Series detection helpers
// ---------------------------------------------------------------------------

/**
 * Case-insensitive lookup of a value in a named series.
 * Returns the index if found, -1 otherwise.
 */
function findInSeries(value: string, series: string[]): number {
  const lower = value.trim().toLowerCase();
  return series.findIndex((s) => s.toLowerCase() === lower);
}

/**
 * Detect whether all source values belong to the same named series
 * (days, months, or quarters). Returns the matching series array and
 * the index of the last value, or null if no match.
 */
function detectNamedSeries(
  sourceVals: string[],
): { series: string[]; lastIndex: number; step: number } | null {
  const allSeries: string[][] = [
    DAYS_SHORT,
    DAYS_LONG,
    MONTHS_SHORT,
    MONTHS_LONG,
    QUARTERS,
  ];

  for (const series of allSeries) {
    const indices = sourceVals.map((v) => findInSeries(v, series));
    if (indices.every((idx) => idx >= 0)) {
      const lastIndex = indices[indices.length - 1];
      // Determine step: default to 1 if single value, else infer from last two
      let step = 1;
      if (indices.length >= 2) {
        const diff = indices[indices.length - 1] - indices[indices.length - 2];
        // Handle wrap-around: e.g. Sun(6) -> Mon(0) is step +1, not -6
        step = ((diff % series.length) + series.length) % series.length;
        if (step === 0) step = series.length; // same value repeated => step full cycle
      }
      return { series, lastIndex, step };
    }
  }

  return null;
}

// ---------------------------------------------------------------------------
// Pattern detection and fill value generation
// ---------------------------------------------------------------------------

/**
 * Detect the pattern in source values and generate `count` fill values.
 *
 * Supported patterns (checked in order):
 * 1. Numeric linear sequence (two or more numbers with a constant step)
 * 2. Single numeric value (constant repeat)
 * 3. Named series (days, months, quarters) -- cyclical continuation
 * 4. Default: cyclic repeat of source values
 */
export function detectAndFill(
  sourceVals: string[],
  count: number,
  _reverse: boolean,
): string[] {
  const result: string[] = [];
  const len = sourceVals.length;

  // --- Numeric linear pattern ---
  const nums = sourceVals.map(Number);
  const allNumeric = sourceVals.every((v) => v.trim() !== '' && !isNaN(Number(v)));

  if (allNumeric && len >= 2) {
    const step = nums[len - 1] - nums[len - 2];
    const isInteger = nums.every((n) => Number.isInteger(n)) && Number.isInteger(step);
    for (let i = 0; i < count; i++) {
      const val = nums[len - 1] + step * (i + 1);
      result.push(isInteger ? String(Math.round(val)) : String(val));
    }
    return result;
  }

  // --- Single numeric value: constant repeat ---
  if (allNumeric && len === 1) {
    for (let i = 0; i < count; i++) {
      result.push(sourceVals[0]);
    }
    return result;
  }

  // --- Named series (days, months, quarters) ---
  const namedMatch = detectNamedSeries(sourceVals);
  if (namedMatch) {
    const { series, lastIndex, step } = namedMatch;
    for (let i = 0; i < count; i++) {
      const idx = (lastIndex + step * (i + 1)) % series.length;
      result.push(series[idx]);
    }
    return result;
  }

  // --- Default: cyclic repeat ---
  for (let i = 0; i < count; i++) {
    result.push(sourceVals[i % len]);
  }
  return result;
}

// ---------------------------------------------------------------------------
// Formula reference adjustment
// ---------------------------------------------------------------------------

/**
 * Regex matching a cell reference in a formula.
 * Captures optional $ before column letters, the column letters,
 * optional $ before row digits, and the row digits.
 * Examples: A1, $A1, A$1, $A$1, AA10, $BC$200
 */
const CELL_REF_RE = /(\$?)([A-Z]+)(\$?)(\d+)/g;

/**
 * Adjust relative cell references in a formula by the given row/column offset.
 * References prefixed with $ are absolute and not adjusted.
 *
 * @param formula - The formula text (without leading =)
 * @param rowOffset - Number of rows to shift (positive = down)
 * @param colOffset - Number of columns to shift (positive = right)
 * @returns The adjusted formula text (without leading =)
 */
export function adjustFormulaRefs(
  formula: string,
  rowOffset: number,
  colOffset: number,
): string {
  return formula.replace(CELL_REF_RE, (_match, colDollar: string, colLetters: string, rowDollar: string, rowDigits: string) => {
    // Adjust column if not absolute
    let newColLetters = colLetters;
    if (!colDollar) {
      const colIndex = letterToCol(colLetters) + colOffset;
      if (colIndex < 0) return _match; // don't adjust to negative
      newColLetters = colToLetter(colIndex);
    }

    // Adjust row if not absolute
    let newRowDigits = rowDigits;
    if (!rowDollar) {
      const rowNum = parseInt(rowDigits, 10) + rowOffset;
      if (rowNum < 1) return _match; // don't adjust to invalid row
      newRowDigits = String(rowNum);
    }

    return `${colDollar}${newColLetters}${rowDollar}${newRowDigits}`;
  });
}

/**
 * Convert column letters (A, B, ..., Z, AA, AB, ...) to 0-based index.
 */
function letterToCol(letters: string): number {
  let col = 0;
  for (let i = 0; i < letters.length; i++) {
    col = col * 26 + (letters.charCodeAt(i) - 64);
  }
  return col - 1;
}

/**
 * Convert 0-based column index to letter string.
 */
function colToLetter(col: number): string {
  let result = '';
  let c = col;
  do {
    result = String.fromCharCode(65 + (c % 26)) + result;
    c = Math.floor(c / 26) - 1;
  } while (c >= 0);
  return result;
}
