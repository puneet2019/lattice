# Lattice

AI-native macOS spreadsheet with built-in MCP server.

[![CI](https://github.com/puneetmahajan/lattice/actions/workflows/ci.yml/badge.svg)](https://github.com/puneetmahajan/lattice/actions/workflows/ci.yml)

---

## Features

- Full spreadsheet engine — 400+ Excel-compatible formulas, multi-sheet, undo/redo
- Built-in MCP server — AI agents (Claude Desktop, Claude Code) can read, write, and analyze spreadsheets via 40+ tools
- Dual MCP transports — stdio (local) and streamable HTTP (networked/multi-agent)
- Native macOS app — ~15MB, distributed as a `.dmg`, no Electron
- Cloud sync compatible — single-file `.xlsx` format works with Google Drive, Dropbox, iCloud
- Live bidirectional sync — MCP writes appear instantly in the GUI
- Fast Rust core — recalculate 100k formulas in milliseconds

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

Once connected, Claude can call tools like `read_range`, `write_cell`, `create_chart`, `describe_data`, and 36 others directly against your open spreadsheet.

---

## Architecture

```
macOS App (.dmg)
  Tauri shell (WKWebView + Rust backend)
    Frontend — SolidJS + Canvas grid
    Rust backend
      lattice-core      Spreadsheet engine (formula eval, cell storage, dependency graph)
      lattice-io        File I/O — calamine (.xlsx read) + rust_xlsxwriter (.xlsx write) + CSV
      lattice-mcp       MCP server — stdio + streamable HTTP transports, 40+ tools
      lattice-charts    SVG chart generation
      lattice-analysis  Statistical and financial analysis
```

The Rust backend is the single source of truth. Frontend communicates via Tauri `invoke()`. MCP and GUI share state through `Arc<RwLock<Workbook>>` with a tokio broadcast channel event bus for live sync.

When launched with `--mcp-stdio`, Lattice checks for a running GUI instance via Unix socket. If found, MCP commands proxy to the live app so changes appear in real time. If not, it runs headless for scripted file manipulation.

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
