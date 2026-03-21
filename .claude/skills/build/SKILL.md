---
name: build
description: Build Lattice — compile Rust backend + frontend, produce debug or release .app
---

# /build

Build the Lattice application.

## Steps

1. Install dependencies if needed:
   ```bash
   cd /Users/puneetmahajan/GolandProjects/lattice
   cargo check 2>&1 | head -5  # Verify Rust compiles
   cd frontend && npm install && cd ..  # Install frontend deps
   ```

2. Build frontend:
   ```bash
   cd frontend && npm run build && cd ..
   ```

3. Build Tauri app:
   ```bash
   cargo tauri build --debug  # Use --release for production
   ```

4. Report build status, errors, and output location.

## Flags
- `--release` — Production build with optimizations
- `--debug` — Debug build (default, faster compilation)

## Output
- Debug: `src-tauri/target/debug/bundle/macos/Lattice.app`
- Release: `src-tauri/target/release/bundle/macos/Lattice.app` + `.dmg`
