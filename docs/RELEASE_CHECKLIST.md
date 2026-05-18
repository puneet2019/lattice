# Release Checklist

Use this checklist for every tagged release. v0.1.0 shipped 2026-03-25; entries below apply to v0.2.0 onward.

## Pre-release

- [ ] All tests pass: `make test`
- [ ] Clippy clean: `make lint`
- [ ] Frontend builds: `cd frontend && npm run build`
- [ ] DMG builds: `make bundle`
- [ ] MCP responds: test with `--mcp-stdio`
- [ ] Version bumped in `src-tauri/tauri.conf.json` and workspace `Cargo.toml`
- [ ] `docs/CHANGELOG.md` entry written
- [ ] README counts (tests, formulas, MCP tools) reflect reality

## Release

- [ ] Tag: `git tag vX.Y.Z`
- [ ] Push tag: `git push origin vX.Y.Z`
- [ ] GitHub Actions builds and creates release
- [ ] DMG attached to release
- [ ] Homebrew formula updated

## Post-release

- [ ] Test `brew install --cask lattice` from tap
- [ ] Update Claude Desktop config to point to installed binary
- [ ] Verify MCP tools work from Claude Desktop
