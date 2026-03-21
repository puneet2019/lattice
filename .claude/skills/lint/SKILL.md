---
name: lint
description: Run all linters — cargo clippy, cargo fmt, eslint, prettier
---

# /lint

Run all code quality checks.

## Steps

1. Rust formatting:
   ```bash
   cd /Users/puneetmahajan/GolandProjects/lattice
   cargo fmt --all -- --check 2>&1
   ```

2. Rust linting:
   ```bash
   cargo clippy --workspace -- -D warnings 2>&1
   ```

3. Frontend linting:
   ```bash
   cd frontend && npx eslint src/ 2>&1
   ```

4. Frontend formatting:
   ```bash
   cd frontend && npx prettier --check src/ 2>&1
   ```

5. Report all warnings and errors. Suggest fixes.

## Auto-fix
- `cargo fmt --all` — auto-fix Rust formatting
- `npx eslint src/ --fix` — auto-fix JS lint issues
- `npx prettier --write src/` — auto-fix JS formatting
