import { describe, it, expect } from 'vitest';
import {
  getColWidth,
  getRowHeight,
  getColX,
  getRowY,
  colAtX,
  rowAtY,
} from './gridGeometry';
import { DEFAULT_COL_WIDTH, DEFAULT_ROW_HEIGHT } from './constants';

describe('getColWidth', () => {
  it('returns default width when no custom width set', () => {
    const widths = new Map<number, number>();
    expect(getColWidth(widths, 0)).toBe(DEFAULT_COL_WIDTH);
    expect(getColWidth(widths, 100)).toBe(DEFAULT_COL_WIDTH);
  });

  it('returns custom width when set', () => {
    const widths = new Map<number, number>([[3, 120]]);
    expect(getColWidth(widths, 3)).toBe(120);
    expect(getColWidth(widths, 4)).toBe(DEFAULT_COL_WIDTH);
  });
});

describe('getRowHeight', () => {
  it('returns default height when no custom height set', () => {
    const heights = new Map<number, number>();
    expect(getRowHeight(heights, 0)).toBe(DEFAULT_ROW_HEIGHT);
  });

  it('returns custom height when set', () => {
    const heights = new Map<number, number>([[5, 40]]);
    expect(getRowHeight(heights, 5)).toBe(40);
    expect(getRowHeight(heights, 6)).toBe(DEFAULT_ROW_HEIGHT);
  });
});

describe('getColX', () => {
  it('returns correct x with all default widths', () => {
    const widths = new Map<number, number>();
    expect(getColX(widths, 0)).toBe(0);
    expect(getColX(widths, 1)).toBe(DEFAULT_COL_WIDTH);
    expect(getColX(widths, 5)).toBe(5 * DEFAULT_COL_WIDTH);
  });

  it('accounts for custom widths before the target column', () => {
    const widths = new Map<number, number>([[1, 120]]);
    // Col 0 is at x=0, col 1 is at x=DEFAULT_COL_WIDTH, col 2 is at DEFAULT_COL_WIDTH + 120
    expect(getColX(widths, 0)).toBe(0);
    expect(getColX(widths, 1)).toBe(DEFAULT_COL_WIDTH);
    // Col 2 = default calculation (2*DEFAULT_COL_WIDTH) + adjustment (120-DEFAULT_COL_WIDTH)
    expect(getColX(widths, 2)).toBe(DEFAULT_COL_WIDTH + 120);
    expect(getColX(widths, 3)).toBe(DEFAULT_COL_WIDTH + 120 + DEFAULT_COL_WIDTH);
  });

  it('handles narrower custom widths', () => {
    const widths = new Map<number, number>([[0, 30]]);
    expect(getColX(widths, 1)).toBe(30);
    expect(getColX(widths, 2)).toBe(30 + DEFAULT_COL_WIDTH);
  });

  it('handles multiple custom widths', () => {
    const widths = new Map<number, number>([
      [0, 150],
      [2, 50],
    ]);
    // Col 0: x=0
    // Col 1: x=150
    // Col 2: x=150+DEFAULT_COL_WIDTH
    // Col 3: x=150+DEFAULT_COL_WIDTH+50
    expect(getColX(widths, 0)).toBe(0);
    expect(getColX(widths, 1)).toBe(150);
    expect(getColX(widths, 2)).toBe(150 + DEFAULT_COL_WIDTH);
    expect(getColX(widths, 3)).toBe(150 + DEFAULT_COL_WIDTH + 50);
  });
});

describe('getRowY', () => {
  it('returns correct y with all default heights', () => {
    const heights = new Map<number, number>();
    expect(getRowY(heights, 0)).toBe(0);
    expect(getRowY(heights, 1)).toBe(DEFAULT_ROW_HEIGHT);
    expect(getRowY(heights, 10)).toBe(10 * DEFAULT_ROW_HEIGHT);
  });

  it('accounts for custom heights', () => {
    const heights = new Map<number, number>([[2, 40]]);
    expect(getRowY(heights, 3)).toBe(2 * DEFAULT_ROW_HEIGHT + 40);
    expect(getRowY(heights, 4)).toBe(2 * DEFAULT_ROW_HEIGHT + 40 + DEFAULT_ROW_HEIGHT);
  });
});

describe('colAtX', () => {
  it('finds correct column with default widths', () => {
    const widths = new Map<number, number>();
    expect(colAtX(widths, 0)).toBe(0);
    expect(colAtX(widths, DEFAULT_COL_WIDTH - 1)).toBe(0);
    expect(colAtX(widths, DEFAULT_COL_WIDTH)).toBe(1);
    expect(colAtX(widths, DEFAULT_COL_WIDTH * 5 + 10)).toBe(5);
  });

  it('finds correct column with custom widths', () => {
    const widths = new Map<number, number>([[0, 120]]);
    // Col 0: 0..119 (width 120)
    // Col 1: 120..(120+DEFAULT_COL_WIDTH-1)
    expect(colAtX(widths, 0)).toBe(0);
    expect(colAtX(widths, 119)).toBe(0);
    expect(colAtX(widths, 120)).toBe(1);
    expect(colAtX(widths, 120 + DEFAULT_COL_WIDTH - 1)).toBe(1);
    expect(colAtX(widths, 120 + DEFAULT_COL_WIDTH)).toBe(2);
  });

  it('finds correct column with narrow custom width', () => {
    const widths = new Map<number, number>([[1, 30]]);
    // Col 0: 0..(DEFAULT_COL_WIDTH-1)
    // Col 1: DEFAULT_COL_WIDTH..(DEFAULT_COL_WIDTH+29) (width 30)
    // Col 2: (DEFAULT_COL_WIDTH+30)..
    expect(colAtX(widths, DEFAULT_COL_WIDTH - 1)).toBe(0);
    expect(colAtX(widths, DEFAULT_COL_WIDTH)).toBe(1);
    expect(colAtX(widths, DEFAULT_COL_WIDTH + 29)).toBe(1);
    expect(colAtX(widths, DEFAULT_COL_WIDTH + 30)).toBe(2);
  });
});

describe('rowAtY', () => {
  it('finds correct row with default heights', () => {
    const heights = new Map<number, number>();
    expect(rowAtY(heights, 0)).toBe(0);
    expect(rowAtY(heights, DEFAULT_ROW_HEIGHT - 1)).toBe(0);
    expect(rowAtY(heights, DEFAULT_ROW_HEIGHT)).toBe(1);
  });

  it('finds correct row with custom heights', () => {
    const heights = new Map<number, number>([[0, 40]]);
    // Row 0: 0..39 (height 40)
    // Row 1: 40..60 (height 21)
    expect(rowAtY(heights, 0)).toBe(0);
    expect(rowAtY(heights, 39)).toBe(0);
    expect(rowAtY(heights, 40)).toBe(1);
    expect(rowAtY(heights, 60)).toBe(1);
    expect(rowAtY(heights, 61)).toBe(2);
  });
});
