export PATH := $(HOME)/.cargo/bin:$(PATH)

# Detect Apple Developer identity for code signing (override via APPLE_SIGNING_IDENTITY env var)
APPLE_SIGNING_IDENTITY ?= $(shell security find-identity -v -p codesigning 2>/dev/null | grep "Developer ID Application" | head -1 | sed 's/.*"\(.*\)".*/\1/' || echo "")

.PHONY: dev build test test-mcp test-e2e lint fmt clean bench bundle sign notarize release docker-dev version-bump homebrew-sha

dev:
	cargo tauri dev

build:
	cd frontend && npm install && npm run build
	cargo tauri build

test:
	cargo test --workspace

test-mcp:
	cargo test -p lattice-mcp

test-e2e:
	cd frontend && npm install
	cd frontend && npm run build
	cargo build
	cd frontend && npm run test:e2e

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all

clean:
	cargo clean
	cd frontend && rm -rf node_modules dist
	rm -f target/release/bundle/macos/rw.*.dmg

bench:
	cargo bench --workspace

# bundle: Build release .app and .dmg
# CI=true causes tauri bundler to pass --skip-jenkins to bundle_dmg.sh,
# bypassing the Finder AppleScript that times out in non-GUI / macOS 26+ environments.
bundle:
	cd frontend && npm install && npm run build
	CI=true cargo tauri build

# sign: Code sign the .app bundle with Apple Developer certificate
sign:
	@if [ -z "$(APPLE_SIGNING_IDENTITY)" ]; then \
		echo "Error: no Apple Developer ID Application certificate found."; \
		echo "Set APPLE_SIGNING_IDENTITY=<identity> or install a signing certificate."; \
		exit 1; \
	fi
	codesign --force --deep --sign "$(APPLE_SIGNING_IDENTITY)" \
		--entitlements src-tauri/entitlements.plist \
		--options runtime \
		target/release/bundle/macos/Lattice.app
	@echo "Signed: target/release/bundle/macos/Lattice.app"

# notarize: Submit .dmg to Apple notarization service
# Requires: APPLE_ID, APPLE_TEAM_ID, APPLE_APP_PASSWORD env vars
notarize:
	@if [ -z "$(APPLE_ID)" ] || [ -z "$(APPLE_TEAM_ID)" ] || [ -z "$(APPLE_APP_PASSWORD)" ]; then \
		echo "Error: set APPLE_ID, APPLE_TEAM_ID, APPLE_APP_PASSWORD before notarizing."; \
		exit 1; \
	fi
	$(eval DMG := $(wildcard target/release/bundle/dmg/Lattice_*.dmg))
	@if [ -z "$(DMG)" ]; then echo "Error: no DMG found. Run 'make bundle' first."; exit 1; fi
	xcrun notarytool submit "$(DMG)" \
		--apple-id "$(APPLE_ID)" \
		--team-id "$(APPLE_TEAM_ID)" \
		--password "$(APPLE_APP_PASSWORD)" \
		--wait
	xcrun stapler staple "$(DMG)"
	@echo "Notarized and stapled: $(DMG)"

# version-bump: Bump version in all manifests
# Usage: make version-bump VERSION=0.2.0
version-bump:
	@if [ -z "$(VERSION)" ]; then echo "Usage: make version-bump VERSION=<semver>"; exit 1; fi
	./scripts/version-bump.sh "$(VERSION)"

# homebrew-sha: Compute SHA-256 of the built aarch64 DMG and print it
# Useful for manually updating homebrew/lattice.rb
homebrew-sha:
	$(eval DMG := $(wildcard target/aarch64-apple-darwin/release/bundle/dmg/Lattice_*.dmg))
	$(eval LOCAL_DMG := $(wildcard target/release/bundle/dmg/Lattice_*.dmg))
	@if [ -n "$(DMG)" ]; then \
		shasum -a 256 $(DMG); \
	elif [ -n "$(LOCAL_DMG)" ]; then \
		shasum -a 256 $(LOCAL_DMG); \
	else \
		echo "Error: no DMG found. Run 'make bundle' first."; exit 1; \
	fi

# release: Full release pipeline (bundle + sign + notarize + tag reminder)
# Prerequisites: Apple Developer certificate and notarization credentials
# NOTE: Tagging requires explicit approval — this target does NOT create a tag.
release: bundle sign notarize
	@echo "Release complete."
	@echo "DMG: $(wildcard target/release/bundle/dmg/Lattice_*.dmg)"
	@echo ""
	@echo "Next: get approval, then tag with: git tag v<version> && git push origin v<version>"

docker-dev:
	docker build -t lattice-dev -f Dockerfile .
	docker run --rm -it \
		-v $(PWD):/workspace \
		-w /workspace \
		lattice-dev
