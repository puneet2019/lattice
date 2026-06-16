import { describe, it, expect } from 'vitest';
import { isOriginAllowed, parseOriginList } from './origin';

describe('isOriginAllowed', () => {
  it('rejects empty / null origins regardless of allowlist', () => {
    expect(isOriginAllowed('', ['https://a.test'])).toBe(false);
    expect(isOriginAllowed(undefined, ['https://a.test'])).toBe(false);
    expect(isOriginAllowed(null, ['https://a.test'])).toBe(false);
    // Even `"*"` does not unlock empty origin.
    expect(isOriginAllowed('', '*')).toBe(false);
    expect(isOriginAllowed(undefined, '*')).toBe(false);
  });

  it('rejects the literal "null" opaque-origin string', () => {
    // Browsers serialize opaque origins as the string "null" — never trust it.
    expect(isOriginAllowed('null', ['https://a.test'])).toBe(false);
    expect(isOriginAllowed('null', '*')).toBe(false);
  });

  it('exact-matches against an array allowlist', () => {
    const list = ['https://a.test', 'https://b.test'];
    expect(isOriginAllowed('https://a.test', list)).toBe(true);
    expect(isOriginAllowed('https://b.test', list)).toBe(true);
    expect(isOriginAllowed('https://c.test', list)).toBe(false);
  });

  it('is case-sensitive (browser origins are canonicalized lowercase)', () => {
    expect(isOriginAllowed('https://A.test', ['https://a.test'])).toBe(false);
  });

  it('treats an empty allowlist as "deny all"', () => {
    expect(isOriginAllowed('https://a.test', [])).toBe(false);
  });

  it('"*" allows any real origin', () => {
    expect(isOriginAllowed('https://anyone.test', '*')).toBe(true);
    expect(isOriginAllowed('http://localhost:5173', '*')).toBe(true);
  });
});

describe('parseOriginList', () => {
  it('returns an empty list for null / empty input', () => {
    expect(parseOriginList(null)).toEqual([]);
    expect(parseOriginList(undefined)).toEqual([]);
    expect(parseOriginList('')).toEqual([]);
  });

  it('splits on commas and trims whitespace', () => {
    expect(parseOriginList('https://a.test, https://b.test ,https://c.test')).toEqual([
      'https://a.test',
      'https://b.test',
      'https://c.test',
    ]);
  });

  it('drops empty entries from trailing or doubled commas', () => {
    expect(parseOriginList(',https://a.test,,https://b.test,')).toEqual([
      'https://a.test',
      'https://b.test',
    ]);
  });
});
