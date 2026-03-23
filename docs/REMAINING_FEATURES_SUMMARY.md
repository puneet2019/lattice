# Remaining Features Summary

Full audit produced 2026-03-23. See session transcript for the complete 800-line document.

## Quick Wins (do first — highest ROI)

1. Wire `insert_note` menu to comment editor (30 min)
2. Wire `insert_checkbox` to set_cell_value (30 min)
3. Wire `view_show_formulas` to grid signal (1 hr)
4. Add 8 missing chart types to ChartDialog.tsx (1 hr)
5. Wire dark mode system preference (2 hr)
6. Add stacked bar/area chart variants (3 hr)
7. Add RATE/PMT/PV/FV formulas (4 hr)
8. Add WORKDAY/NETWORKDAYS (4 hr)
9. Add remove_duplicates Tauri command (4 hr)
10. Add text_to_columns Tauri command (4 hr)

## Wiring Gaps (backend done, frontend stub)

- Remove duplicates, Text to columns, Pivot table
- Insert note/checkbox, Show formulas toggle
- Vertical alignment, Color scale/Data bar/Icon set CF
- 8 hidden chart types in dialog

## Missing Features

- Column grouping, Filter views, IMPORTRANGE
- Print preview dialog, Formula auditing
- Colored formula bar references
- XIRR/XNPV, RATE, ARRAYFORMULA, LET, LAMBDA

## Sprint Order

1. Wire stubs (1 week)
2. Formatting completeness (1 week)
3. Financial formulas (3-4 days)
4. Pivot + Print (1 week)
5. Chart depth (1 week)
6. Modern formula language (2 weeks)
7. Power features (2 weeks)
