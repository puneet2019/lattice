# Lattice — Reference Apps & Libraries

> For agents to consult when stuck on implementation decisions.
> Each entry includes what to look at and why.

---

## 1. Spreadsheet Applications (Study Their UX & Features)

### LibreOffice Calc
- **What**: Full-featured open-source spreadsheet (closest to Excel)
- **Why study**: Formula compatibility, file format handling, edge cases in spreadsheet behavior
- **Repo**: https://github.com/LibreOffice/core
- **Key dirs**: `sc/` (spreadsheet calc module), `sc/source/core/` (engine), `sc/source/filter/` (file I/O)
- **License**: MPL 2.0 / LGPL 3+
- **Look at for**: Formula evaluation edge cases, .xlsx import/export quirks, conditional formatting rules, pivot table logic

### ONLYOFFICE Document Server
- **What**: Open-source office suite with high .xlsx compatibility
- **Repo**: https://github.com/nicedoc/ONLYOFFICE-DocumentServer (mirror), https://github.com/nicedoc/ONLYOFFICE-sdkjs (JS SDK with spreadsheet logic)
- **License**: AGPL 3.0
- **Look at for**: Real-time collaboration model, OOXML format handling, formula functions in JS (can port logic to Rust)

### Google Sheets (Web — our feature parity target)
- **What**: The gold standard we're matching
- **Not open source** but thoroughly documented
- **Key references**:
  - Function list: https://support.google.com/docs/table/25273
  - Keyboard shortcuts: https://support.google.com/docs/answer/181110
  - API (for understanding data model): https://developers.google.com/sheets/api/reference/rest
- **Look at for**: Every feature, every formula, every keyboard shortcut. This is our spec.

### Gnumeric
- **What**: Lightweight open-source spreadsheet, famously accurate math
- **Repo**: https://gitlab.gnome.org/GNOME/gnumeric
- **License**: GPL 2+
- **Look at for**: Statistical function accuracy (often more accurate than Excel), formula parser design

---

## 2. Spreadsheet Engines & Libraries (Build With These)

### IronCalc (Rust) — PRIMARY ENGINE CANDIDATE
- **What**: Rust spreadsheet engine, 400+ Excel-compatible formulas, .xlsx I/O
- **Repo**: https://github.com/ironcalc/IronCalc
- **License**: Open source (NLnet funded)
- **Status**: Pre-1.0, active development
- **Key files**:
  - `base/src/functions/` — all formula implementations
  - `base/src/model.rs` — workbook data model
  - `base/src/parser/` — formula parser
  - `base/src/evaluation/` — formula evaluator
  - `xlsx/` — .xlsx reading/writing
- **Look at for**: Formula implementation patterns, dependency graph, cell reference parsing
- **Caveat**: API may change. Wrap behind trait abstraction.

### Formualizer (Rust)
- **What**: Rust formula engine, 320+ functions, Apache Arrow storage
- **Repo**: https://github.com/psu3d0/formualizer
- **License**: MIT / Apache 2.0
- **Look at for**: Apache Arrow integration for columnar storage (Phase 4 performance), incremental dependency graph (CSR format), SheetPort concept

### HyperFormula (TypeScript)
- **What**: Headless spreadsheet engine, 400+ formulas, by Handsontable team
- **Repo**: https://github.com/handsontable/hyperformula
- **License**: GPLv3 (free) / proprietary (commercial)
- **Look at for**: Formula function specs, i18n (17 languages), named expressions, array formula handling. Good reference for "what a complete formula engine looks like" even though it's TypeScript.

### Univer (TypeScript)
- **What**: Full spreadsheet suite with Canvas rendering, 450+ formulas, plugin architecture
- **Repo**: https://github.com/dream-num/univer
- **License**: Apache 2.0 (core)
- **Look at for**: Canvas-based grid rendering architecture, plugin system design, formula engine in web workers, real-time collaboration model

---

## 3. File I/O Libraries (Rust — Use Directly)

### calamine (Rust) — XLSX READER
- **What**: Pure Rust reader for .xls, .xlsx, .xlsm, .xlsb, .ods
- **Repo**: https://github.com/tafia/calamine
- **License**: MIT / Apache 2.0
- **Performance**: 1.75x faster than Go's excelize, 7x faster than C#'s ClosedXML, 9.4x faster than Python's openpyxl
- **Look at for**: Direct dependency for reading spreadsheet files. Study its API for our `lattice-io` crate.

### rust_xlsxwriter (Rust) — XLSX WRITER
- **What**: Pure Rust .xlsx writer with formulas, charts, conditional formatting
- **Repo**: https://github.com/jmcnamara/rust_xlsxwriter
- **License**: MIT / Apache 2.0
- **Look at for**: Direct dependency for writing .xlsx files. Supports charts in .xlsx output.

### umya-spreadsheet (Rust)
- **What**: Read and write .xlsx with lazy loading
- **Repo**: https://github.com/MathNya/umya-spreadsheet
- **License**: MIT
- **Look at for**: Alternative to calamine+rust_xlsxwriter if we need combined read/write in one lib. Lazy loading for large files.

---

## 4. Grid UI Components (Frontend Reference)

### Handsontable (JavaScript)
- **What**: Spreadsheet-like data grid, 400 formulas via HyperFormula
- **Repo**: https://github.com/handsontable/handsontable
- **Look at for**: Grid rendering patterns, cell editor lifecycle, selection model, clipboard handling, virtualization strategy

### AG Grid (JavaScript)
- **What**: High-performance data grid, framework-agnostic
- **Site**: https://www.ag-grid.com/
- **Look at for**: Virtual scrolling at scale (millions of rows), column pinning, row grouping. Their blog has excellent performance articles.

### Jspreadsheet (JavaScript)
- **What**: Lightweight spreadsheet component
- **Repo**: https://github.com/jspreadsheet/ce
- **Look at for**: Simple spreadsheet component architecture. Good reference for "minimum viable grid."

### Luckysheet / Univer (JavaScript)
- **What**: Full web spreadsheet (Luckysheet evolved into Univer)
- **Repo**: https://github.com/dream-num/univer (successor to Luckysheet)
- **Look at for**: Canvas rendering for spreadsheet grids, how they handle selection, cell overflow, merged cells on canvas

---

## 5. Desktop App Frameworks (Rust)

### Tauri v2 — OUR FRAMEWORK
- **What**: Rust backend + system WebView, native desktop apps
- **Repo**: https://github.com/nicedoc/nicedoc-tauri (mirror), official: https://github.com/nicedoc/nicedoc-tauri
- **Docs**: https://v2.tauri.app/
- **Look at for**: IPC patterns, macOS bundling, menu bar, system tray, file dialogs, auto-updater, code signing

### Zed Editor (gpui) — REFERENCE ONLY
- **What**: GPU-accelerated Rust UI framework built for Zed
- **Repo**: https://github.com/zed-industries/zed (gpui is in `crates/gpui/`)
- **Look at for**: How to build a high-performance text/grid editor in pure Rust. Reference only — we're using Tauri, but gpui patterns may inform our canvas rendering.

---

## 6. Data Analysis (Rust Crates)

### polars (Rust)
- **What**: Fast DataFrame library in Rust (alternative to pandas)
- **Repo**: https://github.com/pola-rs/polars
- **Look at for**: Phase 4 — if we add database connectivity or need to handle very large datasets. Could power a QUERY() function equivalent.

### ndarray (Rust)
- **What**: N-dimensional array library
- **Repo**: https://github.com/rust-ndarray/ndarray
- **Look at for**: Statistical computations, matrix operations for lattice-analysis crate

---

## 7. Usage Patterns by Problem

| Problem | Look At |
|---------|---------|
| "How should formula X work?" | Google Sheets function list → IronCalc implementation → LibreOffice sc/ source |
| "How to render a grid efficiently?" | Univer canvas renderer → AG Grid virtual scrolling blog → Handsontable |
| "How to handle .xlsx edge cases?" | calamine source → LibreOffice sc/source/filter/ → ONLYOFFICE sdkjs |
| "How to implement conditional formatting?" | rust_xlsxwriter conditional format → LibreOffice → Google Sheets docs |
| "How to build charts?" | rust_xlsxwriter chart support → D3.js examples → Google Sheets chart types |
| "How to handle clipboard/paste special?" | Handsontable clipboard module → LibreOffice clipboard |
| "How to implement undo/redo?" | Any command pattern reference → IronCalc if available |
| "How to do real-time collaboration?" | ONLYOFFICE collab model → Univer collab → CRDT literature |
| "How to build pivot tables?" | LibreOffice DataPilot → Google Sheets pivot API |
