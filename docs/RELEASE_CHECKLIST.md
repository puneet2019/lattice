# Release Checklist

## Pre-release

- [ ] All tests pass: `make test`
- [ ] Clippy clean: `make lint`
- [ ] Frontend builds: `cd frontend && npm run build`
- [ ] DMG builds: `make bundle`
- [ ] MCP responds: test with `--mcp-stdio`
- [ ] Version bumped: `make version-bump VERSION=X.Y.Z`

## Release

- [ ] Tag: `git tag v0.1.0`
- [ ] Push tag: `git push origin v0.1.0`
- [ ] GitHub Actions builds and creates release
- [ ] DMG attached to release
- [ ] Homebrew formula updated

## Post-release

- [ ] Test `brew install --cask lattice` from tap
- [ ] Update Claude Desktop config to point to installed binary
- [ ] Verify MCP tools work from Claude Desktop
