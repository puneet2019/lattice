---
name: formula-check
description: Verify formula compatibility with Google Sheets — test formula outputs against known fixtures
---

# /formula-check

Verify Lattice formula outputs match Google Sheets.

## Steps

1. Run formula compatibility test suite:
   ```bash
   cd /Users/puneetmahajan/GolandProjects/lattice
   cargo test -p lattice-core formula_compat 2>&1
   ```

2. Check coverage of Google Sheets functions:
   ```bash
   # Count implemented vs total Google Sheets functions
   cargo test -p lattice-core -- --list 2>&1 | grep "formula" | wc -l
   ```

3. Run edge case tests:
   ```bash
   cargo test -p lattice-core formula_edge_cases 2>&1
   ```

4. Report:
   - Total Google Sheets functions: ~500
   - Implemented: X
   - Tested: Y
   - Failing: Z (list each failure with expected vs actual)

## Formula Categories to Check

| Category | Count | Examples |
|----------|-------|---------|
| Math | ~50 | SUM, AVERAGE, ROUND, ABS, CEILING |
| Statistical | ~30 | STDEV, MEDIAN, PERCENTILE, CORREL |
| Logical | ~10 | IF, AND, OR, IFS, SWITCH, IFERROR |
| Lookup | ~15 | VLOOKUP, HLOOKUP, INDEX, MATCH, XLOOKUP |
| Text | ~30 | CONCATENATE, LEFT, RIGHT, SUBSTITUTE |
| Date | ~25 | TODAY, DATE, DATEDIF, EDATE, NETWORKDAYS |
| Financial | ~15 | PMT, FV, PV, NPV, IRR, XIRR |
| Array | ~10 | ARRAYFORMULA, FLATTEN, TRANSPOSE |
| Info | ~10 | ISBLANK, ISNUMBER, TYPE |

## How to Add Formula Tests

1. Create test case in Google Sheets with input + expected output
2. Export as .xlsx to `tests/fixtures/`
3. Add test in `crates/lattice-core/src/formula/tests/`
4. Verify output matches Google Sheets exactly (within f64 precision)
