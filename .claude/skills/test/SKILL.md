---
name: test
description: Run all Lattice tests — unit, integration, and frontend
---

# /test

Run the full Lattice test suite.

## Steps

1. Run Rust unit tests (all crates):
   ```bash
   cd /Users/puneetmahajan/GolandProjects/lattice
   cargo test --workspace 2>&1
   ```

2. Run integration tests:
   ```bash
   cargo test --test '*' 2>&1
   ```

3. Run frontend tests:
   ```bash
   cd frontend && npm test 2>&1
   ```

4. Report: total tests, passed, failed, skipped. Show failure details.

## Flags
- Pass crate name to test specific crate: `/test lattice-core`
- Pass test name to run specific test: `/test test_sum_formula`

## Quality Gate
- All tests must pass before any commit
- Coverage should meet targets in QA Lead's spec (>90% core, >85% mcp, >80% io)
