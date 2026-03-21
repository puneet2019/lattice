---
name: pr-check
description: Pre-PR validation — run all checks before requesting review
---

# /pr-check

Validate a branch before creating a PR.

## Steps

1. Check branch is not main:
   ```bash
   cd /Users/puneetmahajan/GolandProjects/lattice
   git branch --show-current
   ```

2. Check diff size (<400 lines changed):
   ```bash
   git diff main --stat | tail -1
   ```

3. Run formatting check:
   ```bash
   cargo fmt --all -- --check 2>&1
   ```

4. Run clippy:
   ```bash
   cargo clippy --workspace -- -D warnings 2>&1
   ```

5. Run all tests:
   ```bash
   cargo test --workspace 2>&1
   ```

6. Run frontend checks:
   ```bash
   cd frontend && npm run lint && npm test && cd ..
   ```

7. Check for uncommitted files:
   ```bash
   git status
   ```

8. Verify conventional commit messages:
   ```bash
   git log main..HEAD --oneline
   ```

9. Report: pass/fail for each check, overall verdict.

## Pass Criteria
- [ ] Branch is not `main`
- [ ] Diff < 400 lines (warn if >400, block if >800)
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] All tests pass
- [ ] Frontend lint passes
- [ ] No uncommitted changes
- [ ] Commit messages follow conventional format
- [ ] CHANGELOG.md updated (if feature or fix)
