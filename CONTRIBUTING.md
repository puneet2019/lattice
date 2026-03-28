# Contributing to Lattice

Thanks for your interest in contributing to Lattice! This document explains how to get started.

## Development Environment

### Prerequisites

- macOS 13 (Ventura) or later
- Rust stable (via [rustup](https://rustup.rs))
- Node.js 20+ (for the SolidJS frontend)
- Xcode Command Line Tools (`xcode-select --install`)

### Setup

```sh
git clone https://github.com/puneet2019/lattice.git
cd lattice

# Install frontend dependencies and start dev server
make dev
```

This starts the Tauri development server with hot reload. The Rust backend recompiles on save, and the SolidJS frontend hot-reloads.

## Running Tests

```sh
# Run all Rust tests
make test

# Run MCP integration tests
make test-mcp

# Run E2E tests (requires a built frontend)
make test-e2e

# Run benchmarks
make bench
```

## Code Style

### Rust

All Rust code must pass `cargo fmt` and `cargo clippy` with zero warnings:

```sh
# Check formatting
cargo fmt --all -- --check

# Run clippy
cargo clippy --workspace -- -D warnings

# Auto-format
make fmt
```

Or use the combined lint target:

```sh
make lint
```

### Frontend (TypeScript / SolidJS)

The frontend uses the project's ESLint and Prettier configuration:

```sh
cd frontend
npm run lint
```

## Commit Convention

We use [conventional commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>
```

**Types:** `feat`, `fix`, `refactor`, `test`, `docs`, `build`, `ci`, `perf`, `chore`

**Scopes:** `core`, `mcp`, `io`, `charts`, `analysis`, `frontend`, `tauri`, `devops`

Examples:
```
feat(core): add XLOOKUP formula function
fix(mcp): handle empty range in read_range tool
test(io): add roundtrip tests for .ods export
docs: update MCP setup instructions
```

## Pull Request Process

1. **Branch off `main`** -- create a feature branch with a descriptive name.
2. **Keep PRs small** -- under 400 lines changed. Break large features into incremental pieces.
3. **Write tests** -- all new functionality needs tests. Aim for >90% coverage on core logic.
4. **Run lint and tests** before submitting:
   ```sh
   make lint && make test
   ```
5. **Describe your changes** -- explain what the PR does and why, not just what files changed.

## Architecture Overview

```
crates/
  lattice-core/     Spreadsheet engine (formulas, dependency graph, cell storage)
  lattice-io/       File I/O (xlsx, csv, ods, pdf)
  lattice-mcp/      MCP server (66 tools, 5 resources, 5 prompts)
  lattice-charts/   SVG chart generation (13 chart types)
  lattice-analysis/ Statistical and financial analysis
frontend/           SolidJS + Canvas grid UI
src-tauri/          Tauri app shell (IPC commands, menus, file watcher)
```

The Rust backend is the single source of truth. The frontend is a thin rendering layer that communicates via Tauri `invoke()` calls.

## Good First Issues

Look for issues labeled [`good first issue`](https://github.com/puneet2019/lattice/labels/good%20first%20issue) -- these are scoped, well-defined tasks suitable for new contributors.

## Questions?

Open a [discussion](https://github.com/puneet2019/lattice/discussions) or comment on an issue. We are happy to help.
