---
name: release
description: Release process — version bump, build, sign, create GitHub release
---

# /release

Full release process for Lattice. REQUIRES EXPLICIT USER APPROVAL before proceeding.

## Steps

1. **Pre-flight checks**:
   ```bash
   cd /Users/puneetmahajan/GolandProjects/lattice
   git status  # Must be clean
   git branch --show-current  # Must be main
   cargo test --workspace  # All tests must pass
   cargo clippy --workspace -- -D warnings  # No warnings
   cargo audit  # No known vulnerabilities
   ```

2. **Version bump** (ask user for version):
   - Update `Cargo.toml` (workspace version)
   - Update `src-tauri/tauri.conf.json` (version field)
   - Update `frontend/package.json` (version field)

3. **Update CHANGELOG.md** with release notes

4. **Build release**:
   ```bash
   cd frontend && npm run build && cd ..
   cargo tauri build --release
   ```

5. **STOP — Show diff and ask for explicit user approval before proceeding**

6. **Commit and tag** (only after approval):
   ```bash
   git add -A
   git commit -m "chore(release): v<VERSION>"
   git tag -s v<VERSION> -m "Release v<VERSION>"
   ```

7. **Push** (only after approval):
   ```bash
   git push origin main
   git push origin v<VERSION>
   ```

8. **Create GitHub release** (only after approval):
   ```bash
   gh release create v<VERSION> \
     --title "Lattice v<VERSION>" \
     --notes-file CHANGELOG.md \
     src-tauri/target/release/bundle/dmg/Lattice_*.dmg
   ```

## IMPORTANT
- NEVER create tags or releases without explicit user approval
- NEVER push without explicit user approval
- Always show the diff before committing
- Repository must be PRIVATE
