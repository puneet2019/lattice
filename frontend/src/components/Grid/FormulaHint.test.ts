import { describe, it, expect } from 'vitest';
import { findEnclosingFunction } from './FormulaHint';

describe('findEnclosingFunction', () => {
  it('returns null when cursor is outside any function call', () => {
    expect(findEnclosingFunction('=A1+B2', 3)).toBeNull();
  });

  it('detects SUM when cursor is after open paren', () => {
    // =SUM(|
    const result = findEnclosingFunction('=SUM(', 5);
    expect(result).toEqual({ funcName: 'SUM', argIndex: 0 });
  });

  it('detects SUM with first argument', () => {
    // =SUM(A1|
    const result = findEnclosingFunction('=SUM(A1', 7);
    expect(result).toEqual({ funcName: 'SUM', argIndex: 0 });
  });

  it('detects second argument after comma', () => {
    // =SUM(A1, B2|
    const result = findEnclosingFunction('=SUM(A1, B2', 11);
    expect(result).toEqual({ funcName: 'SUM', argIndex: 1 });
  });

  it('detects third argument', () => {
    // =SUM(A1, B2, C3|
    const result = findEnclosingFunction('=SUM(A1, B2, C3', 15);
    expect(result).toEqual({ funcName: 'SUM', argIndex: 2 });
  });

  it('detects nested function (innermost)', () => {
    // =SUM(A1, IF(B1>0, | — cursor at end (pos = text.length)
    const text = '=SUM(A1, IF(B1>0, ';
    const result = findEnclosingFunction(text, text.length);
    expect(result).toEqual({ funcName: 'IF', argIndex: 1 });
  });

  it('returns null when cursor is after closing paren', () => {
    // =SUM(A1)|
    const result = findEnclosingFunction('=SUM(A1)', 8);
    expect(result).toBeNull();
  });

  it('returns outer function when nested is closed', () => {
    // =SUM(IF(A1,B1), | — cursor at end
    const text = '=SUM(IF(A1,B1), ';
    const result = findEnclosingFunction(text, text.length);
    expect(result).toEqual({ funcName: 'SUM', argIndex: 1 });
  });

  it('works without leading =', () => {
    const result = findEnclosingFunction('SUM(A1', 6);
    expect(result).toEqual({ funcName: 'SUM', argIndex: 0 });
  });

  it('returns null for empty input', () => {
    expect(findEnclosingFunction('', 0)).toBeNull();
  });

  it('returns null for = only', () => {
    expect(findEnclosingFunction('=', 1)).toBeNull();
  });

  it('handles VLOOKUP with multiple commas', () => {
    // =VLOOKUP(A1, B:C, 2, | — cursor at end
    const text = '=VLOOKUP(A1, B:C, 2, ';
    const result = findEnclosingFunction(text, text.length);
    expect(result).toEqual({ funcName: 'VLOOKUP', argIndex: 3 });
  });
});
