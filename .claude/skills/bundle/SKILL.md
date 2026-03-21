---
name: bundle
description: Create macOS .dmg bundle for distribution
---

# /bundle

Build and package Lattice as a macOS .dmg.

## Steps

1. Verify clean build:
   ```bash
   cd /Users/puneetmahajan/GolandProjects/lattice
   cargo test --workspace 2>&1
   ```

2. Build release:
   ```bash
   cd frontend && npm run build && cd ..
   cargo tauri build --release 2>&1
   ```

3. Verify .app bundle:
   ```bash
   ls -la src-tauri/target/release/bundle/macos/
   ```

4. Verify DMG:
   ```bash
   ls -la src-tauri/target/release/bundle/dmg/
   ```

5. Test installation (mount DMG, check .app runs):
   ```bash
   hdiutil attach src-tauri/target/release/bundle/dmg/Lattice_*.dmg
   /Volumes/Lattice/Lattice.app/Contents/MacOS/Lattice --version
   hdiutil detach /Volumes/Lattice
   ```

6. Report: DMG path, file size, architecture (arm64/x86_64/universal).

## Code Signing (when Apple Developer account is configured)
```bash
cargo tauri build --release -- --sign
```

## Output
- `.app` bundle: `src-tauri/target/release/bundle/macos/Lattice.app`
- `.dmg` installer: `src-tauri/target/release/bundle/dmg/Lattice_<version>_<arch>.dmg`
