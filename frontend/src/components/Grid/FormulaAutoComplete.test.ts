import { describe, it, expect } from 'vitest';
import {
  extractCurrentToken,
  filterFormulaFunctions,
  FORMULA_FUNCTIONS,
} from './formulaFunctions';

describe('extractCurrentToken', () => {
  it('extracts the function name from a simple formula', () => {
    expect(extractCurrentToken('=SU')).toBe('SU');
  });

  it('extracts the last token when there are operators', () => {
    expect(extractCurrentToken('=A1+SU')).toBe('SU');
  });

  it('extracts token after open paren', () => {
    expect(extractCurrentToken('=IF(SUM')).toBe('SUM');
  });

  it('returns empty string when cursor is at operator', () => {
    expect(extractCurrentToken('=A1+')).toBe('');
  });

  it('handles empty formula', () => {
    expect(extractCurrentToken('=')).toBe('');
  });

  it('handles formula without = prefix', () => {
    expect(extractCurrentToken('SUM')).toBe('SUM');
  });

  it('extracts full token with numbers', () => {
    expect(extractCurrentToken('=SUMIF')).toBe('SUMIF');
  });
});

describe('filterFormulaFunctions', () => {
  it('returns SUM-related functions for "SU"', () => {
    const results = filterFormulaFunctions('SU');
    const names = results.map((f) => f.name);
    expect(names).toContain('SUM');
    expect(names).toContain('SUMIF');
    expect(names).toContain('SUMIFS');
    expect(names).toContain('SUBTOTAL');
    expect(names).toContain('SUBSTITUTE');
    expect(names).toContain('SUMPRODUCT');
  });

  it('includes exact match and longer matches', () => {
    const results = filterFormulaFunctions('SUM');
    const names = results.map((f) => f.name);
    // SUM itself should be included (exact match shown like Google Sheets)
    expect(names).toContain('SUM');
    expect(names).toContain('SUMIF');
    expect(names).toContain('SUMIFS');
    expect(names).toContain('SUMPRODUCT');
  });

  it('returns empty for empty token', () => {
    expect(filterFormulaFunctions('')).toEqual([]);
  });

  it('returns empty for non-matching prefix', () => {
    expect(filterFormulaFunctions('ZZZZZ')).toEqual([]);
  });

  it('is case insensitive', () => {
    const results = filterFormulaFunctions('su');
    const names = results.map((f) => f.name);
    expect(names).toContain('SUM');
    expect(names).toContain('SUMIF');
  });

  it('limits results to 8', () => {
    // 'A' should match many functions but result is capped at 8
    const results = filterFormulaFunctions('A');
    expect(results.length).toBeLessThanOrEqual(8);
  });
});

describe('FORMULA_FUNCTIONS', () => {
  it('has more than 100 functions defined', () => {
    expect(FORMULA_FUNCTIONS.length).toBeGreaterThan(100);
  });

  it('each function has name, signature, and description', () => {
    for (const fn of FORMULA_FUNCTIONS) {
      expect(fn.name).toBeTruthy();
      expect(fn.signature).toBeTruthy();
      expect(fn.description).toBeTruthy();
    }
  });

  it('function names are uppercase', () => {
    for (const fn of FORMULA_FUNCTIONS) {
      expect(fn.name).toBe(fn.name.toUpperCase());
    }
  });

  it('signatures contain function name', () => {
    for (const fn of FORMULA_FUNCTIONS) {
      expect(fn.signature).toContain(fn.name);
    }
  });
});
