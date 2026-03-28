---
name: lattice
description: Use Lattice spreadsheet via MCP — create, read, format, and analyze spreadsheets
---

# Lattice Spreadsheet Skill

You have access to a Lattice spreadsheet via MCP. Use the lattice MCP tools to create and manipulate spreadsheets.

## Quick Reference

### Write data
- `write_cell(sheet, cell_ref, value)` — write a value ("42" auto-parsed as number)
- `write_range(sheet, range, values)` — write 2D array
- `insert_formula(sheet, cell_ref, formula)` — write formula (no `=` prefix needed)

### Read data
- `read_cell(sheet, cell_ref)` → `{value, formula}`
- `read_range(sheet, range)` → 2D array
- `describe_data(sheet, range)` → `{mean, median, min, max, sum, std_dev, count}`

### Format
- `set_cell_format(sheet, cell_ref, bold, italic, font_color, bg_color, font_size, h_align, borders)`
- Range refs work: `cell_ref="A1:C10"` formats the entire range
- Colors as hex: `font_color="#ff0000"`, `bg_color="#ffff00"`

### Sheets
- `list_sheets()`, `create_sheet(name)`, `rename_sheet(old, new)`, `delete_sheet(name)`

### Data Operations
- `sort_range(sheet, range, sort_by)` — multi-column sort
- `find_in_workbook(query, case_sensitive, use_regex)` — search
- `remove_duplicates(sheet, start_row, end_row, columns)` — dedup
- `text_to_columns(sheet, col, delimiter, start_row, end_row)` — split
- `generate_pivot(sheet, source_range, row_fields, value_fields)` — pivot table
- `auto_fill(sheet, source_range, target_range, direction)` — fill patterns

### Charts (13 types)
- `create_chart(sheet, chart_type, data_range, title)`
- Types: bar, line, pie, scatter, area, combo, histogram, candlestick, treemap, waterfall, radar, bubble, gauge
- `list_charts()`, `delete_chart(chart_id)`

### Analysis
- `describe_data(sheet, range)` — statistics
- `correlate(sheet, range_x, range_y)` — Pearson correlation
- `trend_analysis(sheet, range)` — linear trend + forecast

### Named Ranges & Functions
- `add_named_range(name, range, sheet)` / `list_named_ranges()` / `resolve_named_range(name)`
- `add_named_function(name, params, body, description)` / `list_named_functions()`

### Advanced
- `add_conditional_format(sheet, start_row, start_col, end_row, end_col, rule_type, style)`
- `set_validation(sheet, row, col, ...)` — data validation
- `add_sparkline(sheet, row, col, spark_type, data_range)` — in-cell mini charts
- `hide_rows/unhide_rows/hide_cols/unhide_cols` — visibility
- `protect_sheet/unprotect_sheet` — protection
- `save_filter_view/apply_filter_view` — saved filters

## Tips
- String values are auto-parsed: "42"→number, "true"→boolean
- Set formatting AFTER writing data
- Charts need headers in row 1: first column = labels, rest = data series
- Use `describe_data` first to understand data shape before analysis
- The GUI shows changes live when the Lattice app is running (via Unix socket bridge)
