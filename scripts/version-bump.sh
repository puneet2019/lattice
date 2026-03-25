#!/usr/bin/env bash
# version-bump.sh — Bump version across Cargo.toml, tauri.conf.json, and frontend/package.json
# Usage: ./scripts/version-bump.sh <new-version>
# Example: ./scripts/version-bump.sh 0.2.0

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

NEW_VERSION="$1"

# Validate semver format (major.minor.patch)
if ! echo "$NEW_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "Error: version must be in semver format (e.g. 0.2.0)"
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

CARGO_TOML="$REPO_ROOT/Cargo.toml"
TAURI_CONF="$REPO_ROOT/src-tauri/tauri.conf.json"
PKG_JSON="$REPO_ROOT/frontend/package.json"

# --- Cargo.toml: update [workspace.package] version (first occurrence) ---
OLD_CARGO_VERSION=$(grep -m1 '^version = ' "$CARGO_TOML" | sed 's/version = "\(.*\)"/\1/')
sed -i '' "0,/^version = \"$OLD_CARGO_VERSION\"/s/^version = \"$OLD_CARGO_VERSION\"/version = \"$NEW_VERSION\"/" "$CARGO_TOML"

# --- src-tauri/tauri.conf.json: update top-level "version" field ---
OLD_TAURI_VERSION=$(grep -m1 '"version"' "$TAURI_CONF" | sed 's/.*"version": "\(.*\)".*/\1/')
sed -i '' "s/\"version\": \"$OLD_TAURI_VERSION\"/\"version\": \"$NEW_VERSION\"/" "$TAURI_CONF"

# --- frontend/package.json: update "version" field ---
OLD_PKG_VERSION=$(grep -m1 '"version"' "$PKG_JSON" | sed 's/.*"version": "\(.*\)".*/\1/')
sed -i '' "s/\"version\": \"$OLD_PKG_VERSION\"/\"version\": \"$NEW_VERSION\"/" "$PKG_JSON"

echo "Version bump complete:"
echo "  Cargo.toml:                $OLD_CARGO_VERSION -> $NEW_VERSION"
echo "  src-tauri/tauri.conf.json: $OLD_TAURI_VERSION -> $NEW_VERSION"
echo "  frontend/package.json:     $OLD_PKG_VERSION -> $NEW_VERSION"
echo ""
echo "Next steps:"
echo "  1. Review: git diff"
echo "  2. Commit: git add Cargo.toml src-tauri/tauri.conf.json frontend/package.json"
echo "             git commit -m \"build(devops): bump version to $NEW_VERSION\""
echo "  3. Tag (after explicit release approval): git tag v$NEW_VERSION"
