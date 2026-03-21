# Lattice - AI-Native Spreadsheet for macOS

## Project Overview
Lattice is a macOS-first spreadsheet application with a built-in MCP (Model Context Protocol) server, enabling AI agents like Claude to read, write, and manipulate spreadsheets programmatically. Full Google Sheets feature parity. Distributed as a `.dmg`.

## Tech Stack
- **Backend**: Rust (100% — engine, MCP server, file I/O)
- **Frontend**: Tauri v2 + SolidJS + Canvas-based grid
- **Formula Engine**: IronCalc (Rust, 400+ Excel-compatible formulas) behind trait abstraction
- **File I/O**: calamine (read .xlsx) + rust_xlsxwriter (write .xlsx)
- **MCP**: Custom implementation (tokio + serde_json), stdio + streamable HTTP transports
- **Cloud Sync**: Google Drive compatible (single-file format, no lock files)

## Key References
- `docs/PLAN.md` — Master plan (long-standing document, edit directly for obvious changes, append changelog for additions)
- `docs/REFERENCES.md` — Other spreadsheet apps (LibreOffice, ONLYOFFICE, IronCalc, etc.)
- `docs/MCP_REFERENCES.md` — Apps with good MCP integrations for implementation reference
- `docs/CHANGELOG.md` — Plan changelog

## Agent Team
Agents are in `.claude/agents/` organized by team:
- `product/` — PM, Product Designer, UX Researcher
- `engineering/` — EM, Tech Lead, SDE-Core, SDE-Frontend, SDE-MCP, SDE-IO, DevOps
- `quality/` — QA Lead, QA Engineer, Security Engineer

## Skills
Skills are in `.claude/skills/` — build, test, test-mcp, lint, bundle, benchmark, pr-check, sync-check, formula-check, release.

## Engineering Standards
- Branch off `main`, small PRs (<400 lines), conventional commits
- All core logic in Rust, frontend is thin rendering layer
- MCP server is a first-class citizen — every user-facing feature MUST have an MCP tool equivalent
- Single-file `.lattice` format (zip-based, like .xlsx) for cloud sync compatibility
- No GC languages in the core path
- Tests required: unit (>90% core coverage), integration (MCP tools), E2E (Tauri window)
- Benchmark critical paths: formula recalculation, file I/O, grid rendering

## Commit Convention
```
<type>(<scope>): <description>

Types: feat, fix, refactor, test, docs, build, ci, perf, chore
Scopes: core, mcp, io, charts, analysis, frontend, tauri, devops
```

## File Format & Cloud Sync
- `.lattice` files are `.xlsx` compatible (read/write standard .xlsx)
- Native format is also zip-based for Google Drive/Dropbox/iCloud sync
- No folder-based storage, no lock files, no temp files in the sync directory
- Conflict resolution: last-write-wins for cloud sync (future: CRDT for real-time collab)

## Do NOT
- Introduce any Electron or heavy JS framework dependencies
- Add GC languages (Java, Go, Python) to the runtime critical path
- Break MCP backward compatibility without a migration path
- Store secrets, API keys, or credentials in any project file
- Create tags or releases without explicit approval
