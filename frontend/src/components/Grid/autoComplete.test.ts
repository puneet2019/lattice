import { describe, it, expect } from 'vitest';
import { getColumnSuggestions, filterSuggestions } from './autoCompleteUtils';

describe('getColumnSuggestions', () => {
  it('returns empty array for empty cache', () => {
    const cache = new Map<string, { value: string }>();
    expect(getColumnSuggestions(cache, 0)).toEqual([]);
  });

  it('collects unique values from the target column only', () => {
    const cache = new Map<string, { value: string }>([
      ['0:0', { value: 'Apple' }],
      ['1:0', { value: 'Banana' }],
      ['2:0', { value: 'Apple' }], // duplicate
      ['3:1', { value: 'Cherry' }], // different column
    ]);
    const result = getColumnSuggestions(cache, 0);
    expect(result).toEqual(['Apple', 'Banana']);
  });

  it('excludes empty values', () => {
    const cache = new Map<string, { value: string }>([
      ['0:0', { value: 'Hello' }],
      ['1:0', { value: '' }],
      ['2:0', { value: '  ' }],
    ]);
    const result = getColumnSuggestions(cache, 0);
    expect(result).toEqual(['Hello']);
  });

  it('excludes numeric values', () => {
    const cache = new Map<string, { value: string }>([
      ['0:0', { value: '42' }],
      ['1:0', { value: '3.14' }],
      ['2:0', { value: 'Text' }],
    ]);
    const result = getColumnSuggestions(cache, 0);
    expect(result).toEqual(['Text']);
  });

  it('excludes data URL values', () => {
    const cache = new Map<string, { value: string }>([
      ['0:0', { value: 'data:image/png;base64,abc123' }],
      ['1:0', { value: 'Normal text' }],
    ]);
    const result = getColumnSuggestions(cache, 0);
    expect(result).toEqual(['Normal text']);
  });

  it('returns sorted results', () => {
    const cache = new Map<string, { value: string }>([
      ['0:0', { value: 'Zebra' }],
      ['1:0', { value: 'Apple' }],
      ['2:0', { value: 'Mango' }],
    ]);
    const result = getColumnSuggestions(cache, 0);
    expect(result).toEqual(['Apple', 'Mango', 'Zebra']);
  });

  it('handles multiple columns correctly', () => {
    const cache = new Map<string, { value: string }>([
      ['0:0', { value: 'A1' }],
      ['0:1', { value: 'B1' }],
      ['1:0', { value: 'A2' }],
      ['1:1', { value: 'B2' }],
    ]);
    expect(getColumnSuggestions(cache, 0)).toEqual(['A1', 'A2']);
    expect(getColumnSuggestions(cache, 1)).toEqual(['B1', 'B2']);
  });
});

describe('filterSuggestions', () => {
  const suggestions = ['Apple', 'Apricot', 'Banana', 'Cherry'];

  it('returns empty for empty input', () => {
    expect(filterSuggestions(suggestions, '')).toEqual([]);
    expect(filterSuggestions(suggestions, '   ')).toEqual([]);
  });

  it('filters by prefix case-insensitively', () => {
    expect(filterSuggestions(suggestions, 'ap')).toEqual(['Apple', 'Apricot']);
    expect(filterSuggestions(suggestions, 'AP')).toEqual(['Apple', 'Apricot']);
    expect(filterSuggestions(suggestions, 'b')).toEqual(['Banana']);
  });

  it('excludes exact matches', () => {
    expect(filterSuggestions(suggestions, 'Apple')).toEqual([]);
    expect(filterSuggestions(suggestions, 'apple')).toEqual([]);
  });

  it('returns empty when no match', () => {
    expect(filterSuggestions(suggestions, 'z')).toEqual([]);
  });
});
