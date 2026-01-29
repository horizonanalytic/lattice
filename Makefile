# Horizon Lattice - Build and Release Makefile
#
# This Makefile provides local CI/CD functionality for development and releases.
# Run `make help` to see available targets.

.PHONY: help check build test clippy fmt fmt-check audit doc clean \
        release-check publish-dry-run publish version-check msrv-check \
        all quality pre-commit

# Default target
all: quality test

#------------------------------------------------------------------------------
# Help
#------------------------------------------------------------------------------

help:
	@echo "Horizon Lattice Build System"
	@echo ""
	@echo "Quality Checks:"
	@echo "  make check       - Run cargo check on all crates"
	@echo "  make clippy      - Run clippy lints"
	@echo "  make fmt         - Format code with rustfmt"
	@echo "  make fmt-check   - Check code formatting (no changes)"
	@echo "  make audit       - Audit dependencies for vulnerabilities"
	@echo "  make quality     - Run all quality checks (check, clippy, fmt-check)"
	@echo ""
	@echo "Building & Testing:"
	@echo "  make build       - Build all crates in debug mode"
	@echo "  make build-release - Build all crates in release mode"
	@echo "  make test        - Run all tests"
	@echo "  make test-doc    - Run documentation tests only"
	@echo "  make doc         - Build documentation"
	@echo "  make doc-open    - Build and open documentation"
	@echo ""
	@echo "Release Process:"
	@echo "  make version-check   - Verify versions are consistent across crates"
	@echo "  make msrv-check      - Check build with MSRV (Rust 1.85.0)"
	@echo "  make release-check   - Full pre-release validation"
	@echo "  make publish-dry-run - Test crates.io publish (no upload)"
	@echo "  make publish         - Publish all crates to crates.io"
	@echo ""
	@echo "Utilities:"
	@echo "  make clean       - Remove build artifacts"
	@echo "  make pre-commit  - Run before committing (quality + test)"
	@echo ""

#------------------------------------------------------------------------------
# Quality Checks
#------------------------------------------------------------------------------

check:
	cargo check --workspace --all-features

clippy:
	cargo clippy --workspace --all-features -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

audit:
	@command -v cargo-audit >/dev/null 2>&1 || { echo "Installing cargo-audit..."; cargo install cargo-audit; }
	cargo audit

quality: check clippy fmt-check
	@echo "All quality checks passed!"

#------------------------------------------------------------------------------
# Building & Testing
#------------------------------------------------------------------------------

build:
	cargo build --workspace --all-features

build-release:
	cargo build --workspace --all-features --release

test:
	cargo test --workspace --all-features

test-doc:
	cargo test --workspace --all-features --doc

doc:
	cargo doc --workspace --all-features --no-deps

doc-open:
	cargo doc --workspace --all-features --no-deps --open

#------------------------------------------------------------------------------
# Release Process
#------------------------------------------------------------------------------

# Check that all crate versions match the workspace version
version-check:
	@echo "Checking version consistency..."
	@VERSION=$$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'); \
	echo "Workspace version: $$VERSION"; \
	for crate in crates/*/Cargo.toml; do \
		CRATE_NAME=$$(basename $$(dirname $$crate)); \
		if grep -q 'version.workspace = true' $$crate; then \
			echo "  $$CRATE_NAME: OK (uses workspace version)"; \
		else \
			CRATE_VERSION=$$(grep '^version = ' $$crate | head -1 | sed 's/.*"\(.*\)"/\1/'); \
			if [ "$$CRATE_VERSION" = "$$VERSION" ]; then \
				echo "  $$CRATE_NAME: OK ($$CRATE_VERSION)"; \
			else \
				echo "  $$CRATE_NAME: MISMATCH ($$CRATE_VERSION != $$VERSION)"; \
				exit 1; \
			fi; \
		fi; \
	done
	@echo "Version check passed!"

# Check build with MSRV (requires rustup)
msrv-check:
	@echo "Checking MSRV compatibility (Rust 1.85.0)..."
	@command -v rustup >/dev/null 2>&1 || { echo "Error: rustup required for MSRV check"; exit 1; }
	rustup run 1.85.0 cargo check --workspace --all-features
	@echo "MSRV check passed!"

# Full pre-release validation
release-check: quality test version-check
	@echo ""
	@echo "Checking for required files..."
	@test -f LICENSE-MIT || { echo "Error: LICENSE-MIT missing"; exit 1; }
	@test -f LICENSE-APACHE || { echo "Error: LICENSE-APACHE missing"; exit 1; }
	@test -f NOTICE || { echo "Error: NOTICE missing"; exit 1; }
	@test -f CHANGELOG.md || { echo "Error: CHANGELOG.md missing"; exit 1; }
	@echo "  LICENSE-MIT: OK"
	@echo "  LICENSE-APACHE: OK"
	@echo "  NOTICE: OK"
	@echo "  CHANGELOG.md: OK"
	@echo ""
	@echo "Release check passed! Ready for publish-dry-run."

# Dry run publish to crates.io (validates package metadata)
publish-dry-run:
	@echo "Running publish dry-run for all crates..."
	@echo "Publishing order: macros -> core -> render -> style -> net -> multimedia -> main"
	cd crates/horizon-lattice-macros && cargo publish --dry-run
	cd crates/horizon-lattice-core && cargo publish --dry-run
	cd crates/horizon-lattice-render && cargo publish --dry-run
	cd crates/horizon-lattice-style && cargo publish --dry-run
	cd crates/horizon-lattice-net && cargo publish --dry-run
	cd crates/horizon-lattice-multimedia && cargo publish --dry-run
	cd crates/horizon-lattice && cargo publish --dry-run
	@echo ""
	@echo "Dry run successful! Ready for actual publish."

# Publish all crates to crates.io
# IMPORTANT: Crates must be published in dependency order with delays for index updates
publish:
	@echo "Publishing to crates.io..."
	@echo "WARNING: This will publish all crates. Press Ctrl+C to cancel."
	@echo "Publishing order: macros -> core -> render -> style -> net -> multimedia -> main"
	@read -p "Continue? [y/N] " confirm && [ "$$confirm" = "y" ] || exit 1
	cd crates/horizon-lattice-macros && cargo publish
	@echo "Waiting for crates.io index update..."; sleep 30
	cd crates/horizon-lattice-core && cargo publish
	@echo "Waiting for crates.io index update..."; sleep 30
	cd crates/horizon-lattice-render && cargo publish
	@echo "Waiting for crates.io index update..."; sleep 30
	cd crates/horizon-lattice-style && cargo publish
	@echo "Waiting for crates.io index update..."; sleep 30
	cd crates/horizon-lattice-net && cargo publish
	@echo "Waiting for crates.io index update..."; sleep 30
	cd crates/horizon-lattice-multimedia && cargo publish
	@echo "Waiting for crates.io index update..."; sleep 30
	cd crates/horizon-lattice && cargo publish
	@echo ""
	@echo "All crates published successfully!"

#------------------------------------------------------------------------------
# Utilities
#------------------------------------------------------------------------------

clean:
	cargo clean

pre-commit: quality test
	@echo "Pre-commit checks passed!"
