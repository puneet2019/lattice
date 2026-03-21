# Lattice

AI-native macOS spreadsheet with built-in MCP server.

[![CI](https://github.com/puneet2019/lattice/actions/workflows/ci.yml/badge.svg)](https://github.com/puneet2019/lattice/actions/workflows/ci.yml)

---

## Features

- Full spreadsheet engine — 70+ Excel-compatible formulas, multi-sheet, undo/redo
- Built-in MCP server — AI agents (Claude Desktop, Claude Code) can read, write, and analyze spreadsheets via 22+ tools
- Native macOS app — ~15MB, distributed as a `.dmg`, no Electron
- Cloud sync compatible — single-file `.xlsx` format works with Google Drive, Dropbox, iCloud
- Fast Rust core — formula evaluation, dependency graph with cycle detection, topological recalculation

---

## Current Status

**Phase 1 MVP — In Progress**

| Component | Status | Details |
|-----------|--------|---------|
| Core Engine | Done | 70+ formulas, dependency graph, sort, filter, clipboard, merge cells, undo/redo |
| MCP Server | Done | JSON-RPC 2.0, 22 tools (cell, sheet, data, analysis, chart, file ops), resources, prompts |
| File I/O | Done | xlsx read/write (calamine + rust_xlsxwriter), CSV, JSON export, format detection |
| Frontend | WIP | Toolbar, FormulaBar, SheetTabs, StatusBar done; Canvas grid in progress |
| Tauri App | Done | macOS menu bar, IPC commands (cell, sheet, file, edit, format, data) |
| MCP stdio | WIP | Transport exists, CLI wiring in progress |
| Tests | Passing | 293 tests across 5 crates, 0 warnings |

---

## Quick Start

### Requirements

- macOS 13 (Ventura) or later
- Rust stable (via [rustup](https://rustup.rs))
- Node 20 (for frontend)

### Build from source

```sh
# Install dependencies and run in dev mode
make dev

# Run tests
make test

# Build release .dmg
make bundle
```

---

## MCP Setup

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

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

### Claude Code

```sh
claude mcp add lattice /Applications/Lattice.app/Contents/MacOS/lattice --args --mcp-stdio
```

Once connected, Claude can call tools like `read_range`, `write_cell`, `describe_data`, `sort_range`, `create_chart`, and others directly against your open spreadsheet.

---

## Architecture

```
macOS App (.dmg)
  Tauri shell (WKWebView + Rust backend)
    Frontend — SolidJS + Canvas grid
    Rust backend
      lattice-core      Spreadsheet engine (formula eval, cell storage, dependency graph)
      lattice-io        File I/O — calamine (.xlsx read) + rust_xlsxwriter (.xlsx write) + CSV
      lattice-mcp       MCP server — 22+ tools, resources, prompts
      lattice-charts    SVG chart generation
      lattice-analysis  Statistical and financial analysis
```

The Rust backend is the single source of truth. Frontend communicates via Tauri `invoke()`. MCP and GUI share state through `Arc<RwLock<Workbook>>` with a tokio broadcast channel event bus for live sync.

---

## Makefile Targets

| Target | Description |
|--------|-------------|
| `make dev` | Start Tauri dev server with hot reload |
| `make build` | Build release app |
| `make test` | Run all Rust tests |
| `make test-mcp` | Run MCP integration tests |
| `make lint` | cargo fmt check + clippy |
| `make fmt` | Auto-format all Rust code |
| `make bench` | Run benchmarks |
| `make clean` | Remove build artifacts |
| `make bundle` | Build release .dmg |

---

## License

[MIT](LICENSE) — Copyright 2026 Puneet Mahajan
