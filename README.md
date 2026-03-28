<p align="center">
  <h1 align="center">Lattice</h1>
  <p align="center">The AI-native spreadsheet for macOS. Built in Rust, powered by MCP.</p>
</p>

<p align="center">
  <a href="https://github.com/puneet2019/lattice/actions/workflows/ci.yml"><img src="https://github.com/puneet2019/lattice/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/puneet2019/lattice/releases/latest"><img src="https://img.shields.io/github/v/release/puneet2019/lattice?label=release" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://github.com/puneet2019/lattice/stargazers"><img src="https://img.shields.io/github/stars/puneet2019/lattice?style=flat" alt="Stars"></a>
</p>

---

<!-- TODO: Add screenshot/GIF -->

## Why Lattice

- **AI-native from day one.** A built-in [MCP](https://modelcontextprotocol.io/) server with 66 tools lets Claude Desktop and Claude Code read, write, sort, chart, and analyze your spreadsheets directly. No plugins, no export/import loops.
- **Pure Rust, native speed.** 128 formula functions, dependency-graph recalculation, and a Canvas-rendered grid -- all compiled to a single native binary. No Electron, no GC pauses.
- **18 MB macOS app.** A lightweight `.dmg` that launches instantly and works offline. Single-file `.xlsx` format syncs cleanly with Google Drive, Dropbox, and iCloud.

## Quick Demo

Connect Lattice to Claude Desktop, then ask:

> "Create a new sheet called Q1 Revenue. Add columns for Month, Product, Units, Price, and Revenue. Fill in sample data for Jan--Mar with 3 products. Add a SUM row at the bottom and create a bar chart of revenue by product."

Claude calls Lattice MCP tools (`write_cell`, `create_chart`, `sort_range`, ...) and builds the spreadsheet live -- no copy-pasting, no manual formatting.

## Features

| Category | Highlights |
|----------|-----------|
| **Core** | Cell editing, undo/redo, clipboard, merge cells, freeze panes, auto-fill, find & replace, conditional formatting, data validation, pivot tables |
| **Formulas** | 128 functions -- SUM, VLOOKUP, XLOOKUP, INDEX/MATCH, QUERY, IMPORTRANGE, XIRR, LAMBDA, LET, ARRAYFORMULA, and more |
| **Charts** | 13 types -- bar, line, pie, scatter, area, combo, histogram, candlestick, treemap, waterfall, radar, bubble, gauge |
| **AI / MCP** | 66 tools, 5 resources, 5 prompts -- cell ops, sheet ops, data analysis, charting, formatting, file I/O, sorting, filtering |
| **File I/O** | Read/write: `.xlsx`, `.xls`, `.ods`, `.csv`, `.tsv`. Export: JSON, PDF |
| **App** | macOS menu bar, auto-save, file watcher, dark mode, SF Pro typography |

## Install

### Homebrew

```sh
brew tap puneet2019/lattice
brew install --cask lattice
```

### Download DMG

Grab the latest `.dmg` from [Releases](https://github.com/puneet2019/lattice/releases/latest).

### Build from Source

Requires macOS 13+, Rust stable ([rustup](https://rustup.rs)), and Node.js 20+.

```sh
git clone https://github.com/puneet2019/lattice.git
cd lattice
make dev        # Start dev server with hot reload
make test       # Run all 1,272 tests
make bundle     # Build release .dmg
```

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

Once connected, Claude can call tools like `read_range`, `write_cell`, `describe_data`, `sort_range`, `create_chart`, and `import_range` directly against your open spreadsheet.

## Architecture

```
macOS App (.dmg, ~18 MB)
  Tauri v2 (WKWebView + Rust backend)
    Frontend -- SolidJS + Canvas grid
    Rust backend
      lattice-core       Spreadsheet engine (128 formulas, dependency graph, cell storage)
      lattice-io         File I/O (xlsx, xls, ods, csv, tsv, json, pdf)
      lattice-mcp        MCP server (66 tools, 5 resources, 5 prompts)
      lattice-charts     SVG chart generation (13 chart types)
      lattice-analysis   Statistical and financial analysis
```

The Rust backend is the single source of truth. The frontend communicates via Tauri `invoke()`. MCP and GUI share state through `Arc<RwLock<Workbook>>` with a tokio broadcast channel for live sync.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, code style, and PR process.

Looking for a place to start? Check out [good first issues](https://github.com/puneet2019/lattice/labels/good%20first%20issue).

## License

[MIT](LICENSE) -- Copyright 2026 Puneet Mahajan
