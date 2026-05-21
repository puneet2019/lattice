import { describe, it, expect, vi } from 'vitest';

// `files.ts` imports the backend selector which reads `import.meta.env`;
// default (unset) env keeps it on the Tauri branch, which is fine here —
// we only exercise the pure extension-classification logic.
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import { kindOf } from './files';

describe('kindOf — file extension classification', () => {
  it('classifies .csv as csv', () => {
    expect(kindOf('data.csv')).toBe('csv');
    expect(kindOf('/Users/me/Reports/Q1.CSV')).toBe('csv');
  });

  it('classifies .tsv and .tab as tsv', () => {
    expect(kindOf('export.tsv')).toBe('tsv');
    expect(kindOf('legacy.tab')).toBe('tsv');
    expect(kindOf('EXPORT.TSV')).toBe('tsv');
  });

  it('classifies .xlsx and .lattice as xlsx', () => {
    expect(kindOf('book.xlsx')).toBe('xlsx');
    expect(kindOf('book.lattice')).toBe('xlsx');
  });

  it('defaults unknown / extensionless names to xlsx', () => {
    expect(kindOf('noextension')).toBe('xlsx');
    expect(kindOf('archive.zip')).toBe('xlsx');
  });

  it('handles names with multiple dots', () => {
    expect(kindOf('2026.05.report.csv')).toBe('csv');
    expect(kindOf('backup.final.xlsx')).toBe('xlsx');
  });
});
