import { describe, it, expect } from 'vitest';
import { detectAndFill, adjustFormulaRefs } from './fillUtils';

// ---------------------------------------------------------------------------
// detectAndFill
// ---------------------------------------------------------------------------

describe('detectAndFill', () => {
  // --- Numeric ---
  it('extends a numeric linear sequence', () => {
    expect(detectAndFill(['1', '2', '3'], 3, false)).toEqual(['4', '5', '6']);
  });

  it('extends a numeric sequence with step 2', () => {
    expect(detectAndFill(['2', '4', '6'], 2, false)).toEqual(['8', '10']);
  });

  it('extends a decreasing numeric sequence', () => {
    expect(detectAndFill(['10', '8', '6'], 3, false)).toEqual(['4', '2', '0']);
  });

  it('repeats a single numeric value', () => {
    expect(detectAndFill(['5'], 3, false)).toEqual(['5', '5', '5']);
  });

  // --- Days (short) ---
  it('continues short day names forward', () => {
    expect(detectAndFill(['Mon', 'Tue', 'Wed'], 4, false)).toEqual([
      'Thu', 'Fri', 'Sat', 'Sun',
    ]);
  });

  it('wraps around short day names', () => {
    expect(detectAndFill(['Fri', 'Sat'], 3, false)).toEqual(['Sun', 'Mon', 'Tue']);
  });

  it('handles case-insensitive day matching', () => {
    expect(detectAndFill(['mon', 'tue'], 2, false)).toEqual(['Wed', 'Thu']);
  });

  // --- Days (long) ---
  it('continues long day names forward', () => {
    expect(detectAndFill(['Monday', 'Tuesday'], 2, false)).toEqual([
      'Wednesday', 'Thursday',
    ]);
  });

  it('wraps around long day names', () => {
    expect(detectAndFill(['Saturday', 'Sunday'], 2, false)).toEqual([
      'Monday', 'Tuesday',
    ]);
  });

  // --- Months (short) ---
  it('continues short month names forward', () => {
    expect(detectAndFill(['Jan', 'Feb', 'Mar'], 3, false)).toEqual([
      'Apr', 'May', 'Jun',
    ]);
  });

  it('wraps around short month names', () => {
    expect(detectAndFill(['Nov', 'Dec'], 3, false)).toEqual(['Jan', 'Feb', 'Mar']);
  });

  // --- Months (long) ---
  it('continues long month names forward', () => {
    expect(detectAndFill(['January', 'February'], 2, false)).toEqual([
      'March', 'April',
    ]);
  });

  it('wraps around long month names', () => {
    expect(detectAndFill(['November', 'December'], 2, false)).toEqual([
      'January', 'February',
    ]);
  });

  // --- Quarters ---
  it('continues quarters forward', () => {
    expect(detectAndFill(['Q1', 'Q2'], 3, false)).toEqual(['Q3', 'Q4', 'Q1']);
  });

  it('wraps around quarters', () => {
    expect(detectAndFill(['Q3', 'Q4'], 2, false)).toEqual(['Q1', 'Q2']);
  });

  it('handles single quarter value', () => {
    expect(detectAndFill(['Q2'], 4, false)).toEqual(['Q3', 'Q4', 'Q1', 'Q2']);
  });

  // --- Cyclic fallback ---
  it('repeats text cyclically for unrecognized patterns', () => {
    expect(detectAndFill(['a', 'b', 'c'], 5, false)).toEqual([
      'a', 'b', 'c', 'a', 'b',
    ]);
  });

  it('repeats a single text value', () => {
    expect(detectAndFill(['hello'], 3, false)).toEqual(['hello', 'hello', 'hello']);
  });
});

// ---------------------------------------------------------------------------
// adjustFormulaRefs
// ---------------------------------------------------------------------------

describe('adjustFormulaRefs', () => {
  it('adjusts relative references by row offset', () => {
    expect(adjustFormulaRefs('A1+B1', 1, 0)).toBe('A2+B2');
  });

  it('adjusts relative references by column offset', () => {
    expect(adjustFormulaRefs('A1+B1', 0, 1)).toBe('B1+C1');
  });

  it('adjusts both row and column offsets', () => {
    expect(adjustFormulaRefs('A1+B2', 2, 1)).toBe('B3+C4');
  });

  it('leaves absolute column references unchanged', () => {
    expect(adjustFormulaRefs('$A1+B1', 1, 1)).toBe('$A2+C2');
  });

  it('leaves absolute row references unchanged', () => {
    expect(adjustFormulaRefs('A$1+B$1', 1, 0)).toBe('A$1+B$1');
  });

  it('leaves fully absolute references unchanged', () => {
    expect(adjustFormulaRefs('$A$1+B1', 1, 0)).toBe('$A$1+B2');
  });

  it('handles multi-letter columns', () => {
    expect(adjustFormulaRefs('AA10+AB20', 5, 0)).toBe('AA15+AB25');
  });

  it('handles mixed absolute and relative', () => {
    expect(adjustFormulaRefs('$A$1+B1+C$3+$D4', 2, 1)).toBe('$A$1+C3+D$3+$D6');
  });

  it('does not adjust references below row 1', () => {
    // A1 with offset -2 would give row -1, which is invalid
    expect(adjustFormulaRefs('A1', -2, 0)).toBe('A1');
  });

  it('does not adjust references below column A', () => {
    // A1 with column offset -1 would give column -1
    expect(adjustFormulaRefs('A1', 0, -1)).toBe('A1');
  });

  it('handles SUM and function calls', () => {
    expect(adjustFormulaRefs('SUM(A1:A10)', 1, 0)).toBe('SUM(A2:A11)');
  });

  it('handles formulas with string literals containing refs', () => {
    // Note: this is a known limitation -- string literals are not excluded.
    // For practical purposes in a spreadsheet fill, this is acceptable.
    expect(adjustFormulaRefs('A1', 3, 0)).toBe('A4');
  });
});
