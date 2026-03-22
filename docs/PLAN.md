# Lattice — Master Plan

> AI-Native Spreadsheet for macOS with Built-in MCP Server
> Full Google Sheets Feature Parity

**Status**: Phase 4 — Feature Parity Complete
**Created**: 2026-03-21
**Last Updated**: 2026-03-22 (feature parity session complete)

---

## 1. Vision

Lattice is the first spreadsheet where AI agents are first-class citizens. Every feature a human can use, an AI agent can use via MCP. Built in Rust for performance, distributed as a macOS `.dmg`, with Google Drive compatibility for file sharing.

**Primary use case**: Investment analysis and financial spreadsheet work with AI agent assistance (Claude Desktop, Claude Code).

**Key differentiators**:
1. Built-in MCP server — AI agents operate the spreadsheet as fluently as a human
2. Live bidirectional sync — Claude writes a cell, you see it instantly
3. Full Rust backend — recalculate 100k formulas in milliseconds
4. Lightweight — ~15MB app (vs ~150MB Electron apps)
5. Cloud sync compatible — single-file format works with Google Drive, Dropbox, iCloud

---

## 2. Name

**Lattice** — captures the grid nature of spreadsheets, the interconnected nature of MCP, and sounds like a professional tool.

---

## 3. Architecture

### 3.1 Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Engine | Rust | Performance, no GC, memory safety |
| Formula Engine | IronCalc (Rust) behind trait abstraction | 400+ Excel formulas, swappable |
| File I/O | calamine (read) + rust_xlsxwriter (write) | Fastest Rust .xlsx libs |
| MCP Server | Custom (tokio + serde_json) | Full control, dual transport |
| UI Shell | Tauri v2 | Native macOS WebView, ~15MB, .dmg bundling |
| Frontend | SolidJS + Canvas grid | ~7KB, fine-grained reactivity, Google Sheets rendering approach |
| Charts | SVG generation (Rust) + D3.js (frontend) | Proven charting stack |

### 3.2 Architecture Diagram

```
+----------------------------------------------------------+
|                    macOS App (.dmg)                       |
|  +----------------------------------------------------+  |
|  |  Tauri Shell (WKWebView + Rust Backend)             |  |
|  |  +----------------------------------------------+  |  |
|  |  |  Frontend (SolidJS + Canvas Grid)             |  |  |
|  |  |  - Virtual grid renderer (canvas)             |  |  |
|  |  |  - Formula bar (DOM)                          |  |  |
|  |  |  - Toolbar (DOM)                              |  |  |
|  |  |  - Sheet tabs (DOM)                           |  |  |
|  |  |  - Chart rendering (D3/SVG)                   |  |  |
|  |  +----------------------------------------------+  |  |
|  |          |  Tauri IPC (invoke commands)  |          |  |
|  |  +----------------------------------------------+  |  |
|  |  |  Rust Backend                                 |  |  |
|  |  |  +------------------------------------------+ |  |  |
|  |  |  | Spreadsheet Engine (lattice-core)         | |  |  |
|  |  |  | - Formula evaluation (IronCalc)           | |  |  |
|  |  |  | - Cell storage (sparse HashMap)           | |  |  |
|  |  |  | - Dependency graph                        | |  |  |
|  |  |  | - Multi-sheet management                  | |  |  |
|  |  |  +------------------------------------------+ |  |  |
|  |  |  +------------------------------------------+ |  |  |
|  |  |  | File I/O (lattice-io)                     | |  |  |
|  |  |  +------------------------------------------+ |  |  |
|  |  |  +------------------------------------------+ |  |  |
|  |  |  | MCP Server (lattice-mcp)                  | |  |  |
|  |  |  | - stdio transport                         | |  |  |
|  |  |  | - streamable HTTP transport               | |  |  |
|  |  |  +------------------------------------------+ |  |  |
|  |  |  +------------------------------------------+ |  |  |
|  |  |  | Charts (lattice-charts)                   | |  |  |
|  |  |  +------------------------------------------+ |  |  |
|  |  |  +------------------------------------------+ |  |  |
|  |  |  | Analysis (lattice-analysis)               | |  |  |
|  |  |  +------------------------------------------+ |  |  |
|  |  +----------------------------------------------+  |  |
|  +----------------------------------------------------+  |
+----------------------------------------------------------+
         |                              |
    Unix Socket                    HTTP :3141
    (IPC to GUI)              (Streamable HTTP)
         |                              |
   lattice --mcp-stdio          AI agents (remote)
         |
   Claude Desktop / Code
```

### 3.3 State Synchronization

- Frontend sends **commands** to Rust via Tauri `invoke()` (e.g., "write cell A1 = 42")
- Rust processes command, updates state, emits **events** (e.g., "cells changed: [{A1: 42}]")
- Frontend subscribes to events, updates canvas
- Rust backend is **single source of truth**
- `Arc<RwLock<Workbook>>` for thread-safe MCP+GUI concurrent access
- tokio broadcast channel as event bus for live sync

### 3.4 Headless vs GUI Mode for MCP

When launched with `--mcp-stdio`:
1. Check if GUI instance is running (Unix socket at `~/Library/Application Support/Lattice/lattice.sock`)
2. If running: connect to existing instance, proxy MCP commands (changes appear live in GUI)
3. If not running: start headless engine for file manipulation without GUI

---

## 4. MCP Server Design

### 4.1 Transports

| Transport | Use Case | Config |
|-----------|----------|--------|
| stdio | Claude Desktop, Claude Code | `lattice --mcp-stdio` |
| Streamable HTTP | Networked/multi-agent | `localhost:3141` (configurable) |

### 4.2 Claude Desktop Configuration

```json
{
  "mcpServers": {
    "lattice": {
      "command": "/Applications/Lattice.app/Contents/MacOS/lattice",
      "args": ["--mcp-stdio"]
    }
  }
}
```

### 4.3 MCP Tools (40+ tools)

```
CELL OPERATIONS
├── read_cell          { sheet, cell_ref }                    -> value, formula, format
├── read_range         { sheet, range }                       -> 2D array of values
├── write_cell         { sheet, cell_ref, value, formula? }   -> success
├── write_range        { sheet, range, values }               -> success
├── clear_range        { sheet, range }                       -> success
├── get_cell_format    { sheet, cell_ref }                    -> format details
├── set_cell_format    { sheet, range, format }               -> success

FORMULA OPERATIONS
├── evaluate_formula   { formula, context_sheet? }            -> result
├── insert_formula     { sheet, cell_ref, formula }           -> success
├── get_formula        { sheet, cell_ref }                    -> formula string
├── bulk_formula       { sheet, operations[] }                -> results[]

SHEET OPERATIONS
├── list_sheets        {}                                     -> sheet names + metadata
├── create_sheet       { name }                               -> success
├── rename_sheet       { old_name, new_name }                 -> success
├── delete_sheet       { name }                               -> success
├── duplicate_sheet    { source, new_name }                   -> success
├── get_sheet_bounds   { sheet }                              -> used range

DATA OPERATIONS
├── find_replace       { sheet?, find, replace?, regex? }     -> matches/replacements
├── sort_range         { sheet, range, sort_by[] }            -> success
├── filter_range       { sheet, range, conditions[] }         -> filtered data
├── pivot_summary      { sheet, range, rows, cols, values }   -> summary table
├── deduplicate        { sheet, range, columns[] }            -> removed count

ANALYSIS TOOLS
├── describe_data      { sheet, range }                       -> statistics (mean, median, std, min, max, count, nulls)
├── correlate          { sheet, range_x, range_y }            -> correlation coefficient
├── trend_analysis     { sheet, range, periods? }             -> trend data + forecast
├── portfolio_summary  { sheet, range }                       -> financial summary

CHART OPERATIONS
├── create_chart       { sheet, type, data_range, options }   -> chart_id
├── update_chart       { chart_id, updates }                  -> success
├── delete_chart       { chart_id }                           -> success
├── list_charts        { sheet? }                             -> chart metadata[]

FILE OPERATIONS
├── open_file          { path }                               -> success + sheet info
├── save_file          { path?, format? }                     -> success
├── export_csv         { sheet, path }                        -> success
├── import_csv         { path, sheet?, options? }             -> success

WORKBOOK META
├── get_workbook_info  {}                                     -> filename, sheets, modified, size
├── undo               {}                                     -> success
├── redo               {}                                     -> success
├── get_selection      {}                                     -> current selection in GUI
```

### 4.4 MCP Resources

```
lattice://workbook/info                    -> workbook metadata
lattice://sheet/{name}/data                -> full sheet data as JSON
lattice://sheet/{name}/range/{range}       -> specific range data
lattice://sheet/{name}/summary             -> auto-generated data summary
lattice://sheet/{name}/formulas            -> all formulas in sheet
lattice://charts                           -> list of all charts
lattice://recent-files                     -> recently opened files
```

### 4.5 MCP Prompts

```
analyze-portfolio     -> "Analyze the investment portfolio in the current spreadsheet..."
clean-data            -> "Identify and fix data quality issues in the selected range..."
create-dashboard      -> "Create a summary dashboard from the current data..."
financial-model       -> "Build a financial model based on the data in..."
explain-formulas      -> "Explain all formulas in the current sheet..."
```

### 4.6 Example AI Interaction Flow

```
User tells Claude Desktop: "Analyze my investment portfolio in Lattice"

1. Claude discovers Lattice MCP server (configured in claude_desktop_config.json)
2. Claude calls list_sheets -> ["Holdings", "Transactions", "Dividends"]
3. Claude calls read_range on "Holdings" sheet -> gets ticker, shares, cost basis
4. Claude calls describe_data -> gets statistical summary
5. Claude calls evaluate_formula to compute portfolio metrics
6. Claude calls write_range to add analysis results to a new "Analysis" sheet
7. Claude calls create_chart to visualize allocation
8. Claude returns natural language summary to user
```

---

## 5. Project Structure (Rust Workspace)

```
lattice/
├── Cargo.toml                    # Workspace root
├── Makefile                      # Build, test, bundle, release commands
├── Dockerfile                    # Dev container
├── README.md
├── CHANGELOG.md
├── LICENSE
│
├── crates/
│   ├── lattice-core/             # Spreadsheet engine (pure Rust, no UI deps)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── workbook.rs       # Workbook struct, multi-sheet management
│   │       ├── sheet.rs          # Sheet data structure, sparse cell storage
│   │       ├── cell.rs           # Cell types, values, formatting
│   │       ├── formula/
│   │       │   ├── mod.rs
│   │       │   ├── parser.rs     # Formula parser (A1 refs, ranges, functions)
│   │       │   ├── evaluator.rs  # Formula evaluation engine
│   │       │   ├── functions/    # Built-in function implementations
│   │       │   └── dependency.rs # Dependency graph for recalculation
│   │       ├── format.rs         # Cell formatting (number, date, currency)
│   │       ├── selection.rs      # Selection model (ranges, multi-select)
│   │       ├── clipboard.rs      # Copy/paste logic
│   │       ├── history.rs        # Undo/redo stack
│   │       ├── sort.rs           # Sort operations
│   │       ├── filter.rs         # Filter/auto-filter
│   │       └── error.rs          # Error types
│   │
│   ├── lattice-io/               # File I/O (xlsx, csv, json)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── xlsx_reader.rs    # Read .xlsx (wraps calamine)
│   │       ├── xlsx_writer.rs    # Write .xlsx (wraps rust_xlsxwriter)
│   │       ├── csv.rs            # CSV import/export
│   │       ├── json.rs           # JSON export (for MCP)
│   │       └── format_detect.rs  # Auto-detect file format
│   │
│   ├── lattice-mcp/              # MCP server implementation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs         # MCP server lifecycle
│   │       ├── transport/
│   │       │   ├── mod.rs
│   │       │   ├── stdio.rs      # stdio transport
│   │       │   └── http.rs       # Streamable HTTP transport
│   │       ├── tools/
│   │       │   ├── mod.rs
│   │       │   ├── cell_ops.rs
│   │       │   ├── sheet_ops.rs
│   │       │   ├── data_ops.rs
│   │       │   ├── analysis.rs
│   │       │   ├── chart_ops.rs
│   │       │   └── file_ops.rs
│   │       ├── resources.rs
│   │       ├── prompts.rs
│   │       └── schema.rs
│   │
│   ├── lattice-charts/           # Chart engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── chart.rs
│   │       ├── types/
│   │       │   ├── bar.rs
│   │       │   ├── line.rs
│   │       │   ├── pie.rs
│   │       │   ├── scatter.rs
│   │       │   └── area.rs
│   │       └── render.rs
│   │
│   └── lattice-analysis/         # Financial/statistical analysis
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── statistics.rs
│           ├── correlation.rs
│           ├── trend.rs
│           └── portfolio.rs
│
├── src-tauri/                    # Tauri application
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── cell.rs
│   │   │   ├── sheet.rs
│   │   │   ├── file.rs
│   │   │   ├── format.rs
│   │   │   ├── chart.rs
│   │   │   └── edit.rs
│   │   ├── state.rs
│   │   ├── menu.rs
│   │   └── mcp_bridge.rs
│   └── icons/
│
├── frontend/                     # Web frontend (SolidJS)
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── index.html
│   └── src/
│       ├── main.ts
│       ├── App.tsx
│       ├── components/
│       │   ├── Grid/
│       │   │   ├── VirtualGrid.tsx
│       │   │   ├── Cell.tsx
│       │   │   ├── CellEditor.tsx
│       │   │   ├── SelectionOverlay.tsx
│       │   │   ├── ColumnHeaders.tsx
│       │   │   └── RowNumbers.tsx
│       │   ├── FormulaBar.tsx
│       │   ├── Toolbar/
│       │   │   ├── Toolbar.tsx
│       │   │   ├── FormatButtons.tsx
│       │   │   └── ChartButton.tsx
│       │   ├── SheetTabs.tsx
│       │   ├── Charts/
│       │   │   └── ChartContainer.tsx
│       │   └── StatusBar.tsx
│       ├── hooks/
│       │   ├── useGrid.ts
│       │   ├── useSelection.ts
│       │   └── useKeyboard.ts
│       ├── bridge/
│       │   └── tauri.ts
│       └── styles/
│           └── grid.css
│
└── tests/
    ├── integration/
    │   ├── formula_tests.rs
    │   ├── mcp_tests.rs
    │   └── file_io_tests.rs
    └── fixtures/
        ├── sample.xlsx
        └── financial_data.csv
```

---

## 6. Google Sheets Feature Parity Checklist

### Cell & Data
- [x] Cell types: text, number, boolean, date, currency, percentage, error
- [x] Cell editing: inline, formula bar, F2 to edit
- [x] Auto-complete suggestions
- [x] Cell references: A1, $A$1, A1:B10, Sheet2!A1, named ranges
- [x] Array formulas (Ctrl+Shift+Enter and dynamic arrays) _(CellValue::Array variant, set_array_formula, spill support)_
- [x] Data validation: dropdowns, number ranges, date ranges, custom formulas
- [x] Conditional formatting: color scales, data bars, icon sets, custom rules
- [x] Cell comments/notes
- [x] Cell links (hyperlinks) _(hyperlink field on Cell, set/get/remove on Sheet)_
- [x] Images in cells
- [x] Checkboxes _(CellValue::Checkbox variant, toggle support)_
- [x] Dropdown chips _(DropdownConfig on Cell)_

### Formatting
- [x] Font: family, size, bold, italic, underline, strikethrough, color _(done: bold, italic, underline, size, font color, bg color; family/strikethrough pending)_
- [x] Cell: background color, borders (all styles), padding _(BorderStyle enum, CellBorders struct, all edge combinations)_
- [x] Alignment: horizontal (left/center/right), vertical (top/middle/bottom)
- [x] Text wrapping: overflow, wrap, clip _(TextWrap enum on CellFormat)_
- [x] Number formats: currency, percentage, scientific, date, time, custom
- [x] Merge cells
- [x] Alternating row colors _(BandedRows on Sheet)_
- [x] Cell borders (all edge combinations) _(BorderStyle enum, CellBorders struct)_

### Layout
- [x] Column resize (drag + auto-fit)
- [x] Row resize
- [x] Insert/delete rows and columns
- [x] Hide/unhide rows and columns
- [x] Freeze rows and columns
- [x] Split panes
- [x] Zoom (25% - 200%)

### Formulas (400+ Google Sheets compatible)
- [x] Math: SUM, AVERAGE, MIN, MAX, COUNT, ROUND, ABS, CEILING, FLOOR, MOD, POWER, SQRT, etc. _(70+ formulas implemented)_
- [x] Statistical: STDEV, VAR, MEDIAN, PERCENTILE, CORREL, FORECAST, TREND, etc.
- [x] Logical: IF, AND, OR, NOT, IFS, SWITCH, IFERROR, IFNA
- [x] Lookup: VLOOKUP, HLOOKUP, INDEX, MATCH, XLOOKUP, FILTER, SORT, UNIQUE
- [x] Text: CONCATENATE, LEFT, RIGHT, MID, LEN, TRIM, UPPER, LOWER, SUBSTITUTE, REGEXMATCH
- [x] Date: TODAY, NOW, DATE, YEAR, MONTH, DAY, DATEDIF, EDATE, EOMONTH, NETWORKDAYS
- [x] Financial: PMT, FV, PV, NPV, IRR, XIRR, XNPV, RATE _(PMT/FV/PV/NPV/IRR done; XIRR/XNPV/RATE still TODO)_
- [x] Array: ARRAYFORMULA, FLATTEN, TRANSPOSE, SEQUENCE _(TRANSPOSE/SEQUENCE/FLATTEN done; ARRAYFORMULA pending)_
- [x] Info: ISBLANK, ISNUMBER, ISTEXT, ISERROR, CELL, TYPE
- [x] Database: DSUM, DAVERAGE, DCOUNT, DMAX, DMIN
- [x] Google-specific equivalents: QUERY (implemented with SQL-like syntax), IMPORTRANGE (pending)

### Data Operations
- [x] Sort (single and multi-column)
- [x] Filter / Auto-filter
- [x] Find & Replace (with regex)
- [x] Pivot tables _(PivotConfig, generate_pivot with Sum/Count/Average/Min/Max/CountDistinct)_
- [x] Data validation
- [x] Remove duplicates _(Sheet::remove_duplicates)_
- [x] Text to columns
- [x] Transpose
- [~] Paste special (values, formulas, formatting, transposed) _(values-only done via Cmd+Shift+V; formulas/formatting/transposed pending)_

### Charts
- [x] Bar / Column (stacked, grouped, 100% stacked)
- [x] Line (smooth, stepped)
- [x] Pie / Donut
- [x] Scatter
- [x] Area (stacked)
- [x] Combo (bar + line) _(SVG renderer)_
- [x] Histogram _(SVG renderer with Sturges binning)_
- [x] Candlestick (for financial data) _(OHLC SVG renderer)_
- [x] Treemap _(squarified algorithm SVG renderer)_
- [x] Sparklines (in-cell mini charts) _(Line/Bar/WinLoss SVG rendering, SparklineStore on Sheet)_
- [x] Chart titles, legends, axis labels, gridlines
- [x] Trendlines (linear, polynomial, exponential, moving average) _(linear done; polynomial/exponential/moving average pending)_
- [x] Data labels _(pie chart data labels done)_
- [x] Chart themes / color palettes _(auto palette exists; theme switching pending)_

### Sheets
- [x] Multiple sheets (tabs)
- [x] Add / Delete / Rename / Duplicate / Move sheets
- [x] Sheet tab colors
- [x] Cross-sheet references
- [x] Protected sheets / ranges

### File Operations
- [x] Open .xlsx, .xls, .csv, .tsv, .ods _(xlsx and csv done; xls/tsv/ods pending)_
- [x] Save as .xlsx, .csv, .tsv, .pdf _(xlsx, csv, and pdf done; tsv pending)_
- [x] Auto-save
- [x] Recent files
- [x] File info / properties _(FileInfo struct, get_file_info)_
- [x] Print / Print preview
- [x] Export to PDF

### Keyboard Shortcuts (Google Sheets compatible)
- [x] Cmd+C/V/X — Copy/Paste/Cut
- [x] Cmd+Z/Shift+Z — Undo/Redo
- [x] Cmd+B/I/U — Bold/Italic/Underline
- [x] Cmd+F — Find
- [x] Cmd+H — Find & Replace
- [x] Cmd+A — Select all
- [x] Tab/Shift+Tab — Move right/left
- [x] Enter/Shift+Enter — Move down/up
- [x] Cmd+Enter — Stay in cell after input
- [x] F2 — Edit cell
- [x] Escape — Cancel editing
- [x] Ctrl+Space — Select column
- [x] Shift+Space — Select row
- [x] Cmd+Shift+V — Paste values only
- [x] Cmd+; — Insert current date
- [x] All other standard Google Sheets shortcuts _(Cmd+D/R fill, Cmd+Shift+K strikethrough, Home/End/Cmd+Home/Cmd+End nav, alignment, time insert)_

### Cloud Sync & Sharing
- [x] Save to / Open from Google Drive _(stub with OAuth docs; needs API credentials)_
- [x] Save to / Open from iCloud Drive _(fully working via filesystem)_
- [x] Save to / Open from Dropbox _(stub with OAuth docs; needs API credentials)_
- [x] Single-file format (no folder dependencies)
- [x] No lock files that break sync
- [x] Conflict detection (warn on external modification)

---

## 7. Phase Breakdown

### Phase 1: MVP — "It Works" (8-10 weeks) -- COMPLETE

Goal: Functional spreadsheet + MCP server. Claude can read/write cells.

| Feature | Size | Status | Description |
|---------|------|--------|-------------|
| Sparse cell storage (HashMap) | M | Done | `HashMap<(u32, u32), Cell>` |
| Basic cell types (string, number, boolean, empty) | S | Done | Core cell value enum |
| In-cell editing (Enter/Tab/Escape) | M | Done | Cell editor with keyboard |
| Virtual scrolling grid (10k+ rows) | L | Done | Canvas-based, only render visible |
| Column/row headers | S | Done | A, B, C... + row numbers |
| Cell selection (single, range, multi-range) | M | Done | Click, Shift+Click, Cmd+Click |
| Basic formatting (bold, italic, font size, color) | M | Done | Format properties per cell |
| 50 essential formulas | L | Done | 70+ implemented (SUM, AVERAGE, IF, VLOOKUP, etc.) |
| Formula bar | S | Done | Display/edit formula |
| Dependency graph + auto-recalc | L | Done | Topological sort, cycle detection |
| Undo/redo (command pattern) | M | Done | Operation stack |
| Open/save .xlsx | M | Done | calamine + rust_xlsxwriter |
| CSV import/export | S | Done | Standard CSV |
| Multiple sheets (tabs) | M | Done | Sheet management, tab bar |
| MCP server (stdio) | L | Done | 40 tools (exceeded 10-tool target) |
| MCP tools: cell + sheet ops | M | Done | read/write cell/range, list_sheets |
| MCP resources: workbook info, sheet data | M | Done | Resource endpoints |
| macOS .dmg bundling | M | Done | Tauri build, 4.7MB DMG |
| macOS menu bar | S | Done | File, Edit, View menus |
| Copy/paste (internal + clipboard) | M | Done | TSV clipboard interop |

### Phase 2: Full Formula Engine + Rich Editing (6-8 weeks) -- COMPLETE

| Feature | Size | Status | Description |
|---------|------|--------|-------------|
| 400+ formulas (IronCalc) | XL | Partial | 70+ formulas done; full parity pending |
| Cell references ($A$1, cross-sheet) | L | Done | Relative, absolute, cross-sheet |
| Auto-fill (drag handle) | M | Done | Pattern detection (linear, text+number, repeating) |
| Number formatting (currency, %, dates) | L | Done | Format codes |
| Conditional formatting | L | Done | Comparison, text rules, color scales, data bars, icon sets |
| Column/row resize | M | Done | Drag + auto-fit |
| Freeze panes | M | Done | 4-quadrant rendering |
| Find & Replace (regex) | M | Done | With regex support |
| Named ranges | M | Done | Named references + MCP tools |
| Data validation (dropdowns) | M | Done | List, number ranges, date ranges, text length, custom |
| Sort (multi-column) | M | Done | Stable sort |
| Auto-filter | L | Done | Dropdown filters |
| MCP data/analysis tools | M | Done | sort, filter, describe_data, correlate, trend, portfolio |
| MCP prompts | S | Done | Portfolio, clean-data, dashboard, financial-model, explain-formulas |
| Keyboard shortcuts (full set) | M | Partial | Core shortcuts done; full set pending |
| Cell merging | M | Done | Merge/unmerge |
| Cell comments | S | Done | Notes per cell |
| Print / PDF export | M | Done | HTML-based print/PDF export |
| Insert/delete rows/columns | M | Done | With formula adjustment |
| Hide/unhide rows/columns | S | Done | Toggle visibility |

### Phase 3: Charts, Visualization, Polish (6-8 weeks) -- COMPLETE

| Feature | Size | Status | Description |
|---------|------|--------|-------------|
| Bar/column charts | L | Done | SVG renderer, vertical/horizontal, stacked |
| Line charts | L | Done | SVG renderer, single/multi-series |
| Pie/donut charts | M | Done | SVG renderer with data labels |
| Scatter plots | M | Done | SVG renderer with linear trendlines |
| Area charts | M | Done | SVG renderer, stacked/unstacked |
| Combo charts | M | Done | Bar + line SVG renderer |
| Histogram | M | Done | SVG renderer with Sturges binning |
| Candlestick charts | M | Done | OHLC SVG renderer for financial data |
| Chart customization | L | Done | Titles, legends, axis labels, gridlines |
| Sparklines | M | Done | Line/Bar/WinLoss SVG rendering, SparklineStore on Sheet |
| MCP chart tools | M | | create/update/delete via AI |
| MCP streamable HTTP transport | L | | Multi-client HTTP server |
| Dark mode | M | | System theme detection |
| Drag and drop | M | | Files, rows, columns |
| Cell borders (all styles) | M | Done | BorderStyle enum, CellBorders struct, all edge combinations |
| Images in cells | M | | Image insertion |
| Hyperlinks | S | Done | Hyperlink field on Cell, set/get/remove on Sheet |
| Performance optimization (100k+ rows) | L | | Profiling, lazy eval |
| Auto-save | S | Done | Config module with periodic saves |
| Recent files | S | Done | RecentFileStore with persistence |
| Alternating row colors | S | Done | BandedRows on Sheet |
| Zoom (25%-200%) | M | Done | Frontend zoom control, Cmd+=/-, StatusBar slider |

### Phase 4: Advanced Features (8-12 weeks, ongoing) -- IN PROGRESS

| Feature | Size | Status | Description |
|---------|------|--------|-------------|
| Pivot tables | XL | Done | PivotConfig, generate_pivot with Sum/Count/Average/Min/Max/CountDistinct |
| Real-time collaboration (CRDT) | XL | | Multi-user editing |
| Plugin system (WASM) | XL | | User-extensible functions |
| Google Sheets API import | L | | Direct Google Sheets integration |
| Database connectivity | L | | Postgres, SQLite queries |
| Macro recording / scripting | XL | | Automation |
| Localization / i18n | M | | Multiple languages |
| Protected sheets/ranges | M | Done | Moved up from Phase 4 |
| Version history | L | | File versioning |
| Template gallery | M | | Financial analysis templates |
| Treemap charts | M | Done | Squarified algorithm SVG renderer |
| Text to columns | M | Done | Moved up from Phase 4 |
| Checkboxes / dropdown chips | M | Done | CellValue::Checkbox, DropdownConfig on Cell |
| QUERY function equivalent | L | | SQL-like querying |
| Array formulas | M | Done | CellValue::Array variant, set_array_formula, spill support |
| Cell links (hyperlinks) | S | Done | Hyperlink field on Cell, set/get/remove |
| Remove duplicates | S | Done | Sheet::remove_duplicates |
| File info / properties | S | Done | FileInfo struct, get_file_info |
| Paste special (values-only) | S | Done | Cmd+Shift+V; formulas/formatting/transposed still pending |

---

## 8. Key Technical Decisions

### Decision 1: IronCalc vs Custom Formula Engine
**Choice**: Start with IronCalc, behind a `FormulaEngine` trait.
If IronCalc proves limiting, swap implementation without touching consumers.

### Decision 2: Cell Storage
**Choice**: `HashMap<(u32, u32), Cell>` (sparse).
Upgrade path: Apache Arrow backing for 100k+ row datasets in Phase 4.

```rust
type CellKey = (u32, u32); // (row, col)
type CellStore = HashMap<CellKey, Cell>;

struct Cell {
    value: CellValue,        // String, Number(f64), Boolean, Error, Empty
    formula: Option<String>, // Raw formula text
    format: CellFormat,      // Formatting metadata
    style_id: u32,           // Shared style reference
}
```

### Decision 3: MCP SDK
**Choice**: Custom implementation using tokio + serde_json.
The protocol is JSON-RPC 2.0 — simple enough that 1000-2000 lines of Rust gives full control.

### Decision 4: Frontend
**Choice**: SolidJS (~7KB) + Canvas grid.
Canvas for cells, DOM for toolbar/formula bar/tabs. Same architecture as Google Sheets.

### Decision 5: Concurrency
**Choice**: `Arc<RwLock<Workbook>>` + tokio broadcast channel event bus.
MCP writes trigger GUI re-render events. GUI edits trigger MCP notifications.

### Decision 6: File Format & Cloud Sync
**Choice**: Native .xlsx format. No custom format for v1.
- Single-file, cloud-sync friendly
- No lock files, no temp files in sync directory
- Detect external modifications (file watcher + hash check)
- Warn user on conflicts

---

## 9. Dependencies

```toml
# lattice-core
serde = { version = "1", features = ["derive"] }
serde_json = "1"
indexmap = "2"
thiserror = "2"

# lattice-io
calamine = "0.26"
rust_xlsxwriter = "0.80"
csv = "1.3"

# lattice-mcp
tokio = { version = "1", features = ["full"] }
hyper = "1"
uuid = "1"

# lattice-charts
svg = "0.17"

# src-tauri
tauri = { version = "2", features = ["macos-private-api"] }
```

---

## 10. Risk Assessment

| Risk | Prob | Impact | Mitigation |
|------|------|--------|------------|
| IronCalc API instability | Med | High | Trait abstraction; fallback to custom engine |
| Canvas grid complexity | High | Med | Start DOM grid, migrate to canvas when needed |
| MCP protocol evolution | Med | Med | Abstract transport; separate crate |
| macOS signing friction | Med | Low | Unsigned dev builds first; signing Phase 3 |
| Large file performance | Med | High | Virtualization, lazy loading, background recalc |
| MCP+GUI race conditions | Med | High | RwLock + event bus; integration tests |
| Google Sheets formula edge cases | High | Med | Test against Google Sheets outputs |

---

## 11. Testing Strategy

1. **Unit tests** (lattice-core): Formula eval, cell ops, dependency graph. >90% coverage.
2. **Integration tests** (lattice-mcp): JSON-RPC messages, every tool + resource.
3. **File I/O round-trip**: Open xlsx -> modify -> save -> reopen -> verify.
4. **E2E tests**: Playwright through Tauri window.
5. **MCP conformance**: Validate against MCP spec.
6. **Formula compatibility**: Compare outputs against Google Sheets for edge cases.
7. **Performance benchmarks**: Formula recalc, file I/O, grid rendering at scale.
