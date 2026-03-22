# Lattice Plan Changelog

All notable additions and modifications to the project plan.

For obvious/minor edits, update `PLAN.md` directly. For significant additions, scope changes, or architectural decisions, log them here.

## [2026-03-22] Phase 2 Complete, Phase 3 In Progress — Major Checklist Update

Updated `PLAN.md` status from "Phase 2 — In Progress" to "Phase 3 — In Progress". Phase 2 is now fully complete. Significant Phase 3 progress recorded.

### Phase 2 — Now Complete (all 20 items done)
- Print / PDF export: HTML-based print and PDF export implemented (was the last remaining item)
- Keyboard shortcuts expanded: Cmd+B/I/U, Cmd+F, Cmd+H, Cmd+A, Cmd+Enter, Ctrl+Space, Shift+Space, Cmd+Shift+V, Cmd+;

### Phase 3 Charts — 8 of 22 items done
- Bar/Column charts: SVG renderer done
- Line charts: SVG renderer done
- Pie/Donut charts: SVG renderer done with data labels
- Scatter plots: SVG renderer done with linear trendlines
- Area charts: SVG renderer done
- Chart customization: titles, legends, axis labels, gridlines all done
- Auto-save: config module with periodic saves
- Recent files: RecentFileStore with persistence
- Zoom (25%-200%): frontend zoom control with Cmd+=/- and StatusBar slider

### Formula engine progress
- Lookup: XLOOKUP, FILTER, SORT, UNIQUE now implemented (all lookup functions complete)
- Financial: NPV, IRR now implemented (XIRR/XNPV/RATE still TODO)
- Array: TRANSPOSE, SEQUENCE, FLATTEN implemented (ARRAYFORMULA pending)
- Database: DSUM, DAVERAGE, DCOUNT, DMAX, DMIN all implemented
- REGEXMATCH: confirmed implemented

### Additional completed features (pulled forward from later phases)
- Sheet tab colors
- Cross-sheet references (Sheet2!A1 syntax)
- Protected sheets/ranges (with password, protected ranges) — originally Phase 4
- Text to columns — originally Phase 4
- Conflict detection (FileWatcher with SHA-256 hash checking)
- Single-file format (xlsx, no folder dependencies)
- No lock files that break sync

### Section 6 checklist summary (Google Sheets parity)
- Cell & Data: 5/12 done
- Formatting: 4/8 done
- Layout: 6/7 done (Zoom now done; only Split panes remaining)
- Formulas: 10/11 categories done or partial (only Google-specific equivalents remaining)
- Data Operations: 6/9 done (Text to columns, Transpose now done)
- Charts: 8/14 done
- Sheets: 5/5 done
- File Operations: 6/7 done (only File info/properties remaining)
- Keyboard Shortcuts: 14/16 done
- Cloud Sync: 3/6 done (format/lockfile/conflict detection done; API integration pending)

---

## [2026-03-22] Phase 1 Complete, Phase 2 In Progress — Checklist Update (earlier)

Updated `PLAN.md` status from "Planning" to "Phase 2 — In Progress" and marked completed items across Sections 6 and 7.

### Phase 1 MVP — All 20 items complete
- Core engine: sparse cell storage, cell types, in-cell editing, virtual scrolling grid, cell selection, basic formatting, 70+ formulas (exceeding original 50-formula target), formula bar, dependency graph with cycle detection, undo/redo
- File I/O: xlsx read/write (calamine + rust_xlsxwriter), CSV import/export, JSON export, format detection — 36 tests
- MCP server: 40 tools (exceeded original 10-tool target), resources, prompts, stdio transport — 129 tests
- Frontend: canvas VirtualGrid, toolbar, formula bar, sheet tabs, status bar, copy/paste/cut, file open/save
- DevOps: 4.7MB DMG bundle, multi-resolution icons, Makefile targets

### Phase 2 Rich Editing — 17 of 20 items complete
- Done: cell references ($A$1), auto-fill pattern detection, number formatting, conditional formatting (comparison/text/color scales/data bars/icon sets), column/row resize, freeze panes (4-quadrant), find & replace with regex, named ranges, data validation (list/number/date/text/custom), multi-column sort, auto-filter, MCP data/analysis tools (40 tools total), MCP prompts (5 templates), cell merging, cell comments, insert/delete rows/columns, hide/unhide rows/columns
- Partial: formulas at 70+ (target 400+), keyboard shortcuts (core set done, full set pending)
- Remaining: print/PDF export

### Test coverage
- lattice-core: 322 tests passing
- lattice-mcp: 129 tests passing
- lattice-io: 36 tests passing
- Frontend: TypeScript clean (no type errors)

### Section 6 checklist summary (Google Sheets parity)
- Cell & Data: 5/12 done
- Formatting: 4/8 done
- Layout: 5/7 done
- Formulas: 6/11 categories done (2 partial)
- Data Operations: 4/9 done
- Charts: 0/14 done
- Sheets: 2/5 done
- File Operations: 2/7 done (with partial coverage notes)
- Keyboard Shortcuts: 6/16 done
- Cloud Sync: 0/6 done

## [2026-03-21] Initial Plan

- Created master plan (`docs/PLAN.md`) with full architecture, phase breakdown, and technical decisions
- Established project team: 13 agents across Product, Engineering, and Quality teams
- Defined 10 project skills for build, test, lint, bundle, etc.
- Documented reference apps (`docs/REFERENCES.md`) and MCP integration examples (`docs/MCP_REFERENCES.md`)
- Key decisions:
  - Tauri v2 + SolidJS + Canvas grid for UI
  - IronCalc for formula engine (behind trait abstraction)
  - Custom MCP server implementation (tokio + serde_json)
  - Google Drive compatible single-file format
  - 4-phase delivery: MVP (8-10w) -> Full Formulas (6-8w) -> Charts (6-8w) -> Advanced (ongoing)
