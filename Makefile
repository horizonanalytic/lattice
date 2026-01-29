# Horizon Lattice - Build and Release Makefile
#
# This Makefile provides local CI/CD functionality for development and releases.
# Run `make help` to see available targets.

.PHONY: help check build test clippy fmt fmt-check audit doc clean \
        release-check publish-dry-run publish version-check msrv-check \
        all quality pre-commit size-check bloat license-check

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
	@echo "  make license-check - Audit dependency licenses (requires cargo-deny)"
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
	@echo "Size Analysis:"
	@echo "  make size-check  - Show release binary sizes"
	@echo "  make bloat       - Analyze binary bloat (requires cargo-bloat)"
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

# Audit dependency licenses for compatibility with MIT OR Apache-2.0
license-check:
	@command -v cargo-deny >/dev/null 2>&1 || { echo "Installing cargo-deny..."; cargo install cargo-deny; }
	cargo deny check licenses

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
# Size Analysis
#------------------------------------------------------------------------------

# Show size of compiled library artifacts
size-check: build-release
	@echo "Release build sizes:"
	@echo ""
	@for lib in target/release/libhorizon_lattice*.rlib; do \
		if [ -f "$$lib" ]; then \
			SIZE=$$(ls -lh "$$lib" | awk '{print $$5}'); \
			NAME=$$(basename "$$lib"); \
			echo "  $$NAME: $$SIZE"; \
		fi; \
	done
	@echo ""
	@echo "Total target/release size:"
	@du -sh target/release 2>/dev/null || echo "  (build first with 'make build-release')"

# Analyze binary bloat using cargo-bloat
# This shows which crates and functions contribute most to binary size
bloat:
	@command -v cargo-bloat >/dev/null 2>&1 || { echo "Installing cargo-bloat..."; cargo install cargo-bloat; }
	@echo "Analyzing binary bloat for horizon-lattice..."
	@echo ""
	@echo "Top crates by size contribution:"
	cargo bloat --release --crates -p horizon-lattice --all-features 2>/dev/null || \
		echo "  Note: cargo-bloat works best with binary crates. For library analysis, build an example."
	@echo ""
	@echo "Tip: For detailed function-level analysis, run:"
	@echo "  cargo bloat --release -p horizon-lattice --all-features -n 20"

#------------------------------------------------------------------------------
# Utilities
#------------------------------------------------------------------------------

clean:
	cargo clean

pre-commit: quality test
	@echo "Pre-commit checks passed!"
