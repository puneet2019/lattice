# Lattice

AI-native macOS spreadsheet with built-in MCP server.

[![CI](https://github.com/puneet2019/lattice/actions/workflows/ci.yml/badge.svg)](https://github.com/puneet2019/lattice/actions/workflows/ci.yml)

---

## Features

- **128 formula functions** -- SUM, VLOOKUP, QUERY, IMPORTRANGE, XIRR, LAMBDA, and more
- **66 MCP tools** -- AI agents (Claude Desktop, Claude Code) can read, write, sort, chart, and analyze spreadsheets programmatically
- **13 chart types** -- bar, line, pie, scatter, area, combo, histogram, candlestick, treemap, waterfall, radar, bubble, gauge
- **Native macOS app** -- ~15 MB `.dmg`, no Electron, no GC languages
- **Cloud sync** -- single-file `.xlsx` format works with Google Drive, Dropbox, iCloud
- **Full Rust core** -- 1,272 tests, formula evaluation, dependency graph with cycle detection, topological recalculation

---

## Screenshot

<!-- TODO: add screenshot after first public release -->

---

## Current Status

**v0.1.0 Release Candidate**

| Component | Status | Details |
|-----------|--------|---------|
| Core Engine | Done | 128 formulas, dependency graph, sort, filter, clipboard, merge cells, undo/redo, pivot tables, QUERY, IMPORTRANGE |
| MCP Server | Done | JSON-RPC 2.0, 66 tools, 5 resources, 5 prompts (cell, sheet, data, analysis, chart, file, format, validation ops) |
| File I/O | Done | .xlsx read/write (calamine + rust_xlsxwriter), .csv, .tsv, .ods, .xls, JSON export, PDF export |
| Frontend | Done | Canvas grid, toolbar, formula bar, sheet tabs, status bar, conditional format rendering, chart overlays |
| Tauri App | Done | macOS menu bar, IPC commands, file watcher, auto-save |
| MCP stdio | Done | stdio transport for Claude Desktop / Claude Code |
| Tests | Passing | 1,272 tests across 5 crates, 0 warnings |

---

## Quick Start

### Requirements

- macOS 13 (Ventura) or later
- Rust stable (via [rustup](https://rustup.rs))
- Node 20 (for frontend)

### Install via Homebrew

```sh
brew tap puneet2019/lattice
brew install --cask lattice
```

### Download DMG

Download the latest `.dmg` from [Releases](https://github.com/puneet2019/lattice/releases).

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

Once connected, Claude can call tools like `read_range`, `write_cell`, `describe_data`, `sort_range`, `create_chart`, `import_range`, and others directly against your open spreadsheet.

---

## Architecture

```
macOS App (.dmg)
  Tauri shell (WKWebView + Rust backend)
    Frontend -- SolidJS + Canvas grid
    Rust backend
      lattice-core      Spreadsheet engine (128 formulas, dependency graph, cell storage)
      lattice-io        File I/O -- xlsx, xls, ods, csv, tsv, json, pdf
      lattice-mcp       MCP server -- 66 tools, 5 resources, 5 prompts
      lattice-charts    SVG chart generation (13 chart types)
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

## Documentation

- [Master Plan](docs/PLAN.md) -- full feature list, architecture, phase breakdown
- [Changelog](docs/CHANGELOG.md) -- plan revision history
- [References](docs/REFERENCES.md) -- IronCalc, LibreOffice, ONLYOFFICE, HyperFormula
- [MCP References](docs/MCP_REFERENCES.md) -- MCP integration patterns

---

## License

[MIT](LICENSE) -- Copyright 2026 Puneet Mahajan
