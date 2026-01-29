# Contributing to Horizon Lattice

Thank you for your interest in contributing to Horizon Lattice.

## Development Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/horizon-analytic-studios/horizon-lattice.git
   cd horizon-lattice
   ```

2. Ensure you have Rust 1.85.0 or later installed:
   ```bash
   rustup update stable
   ```

3. Build the project:
   ```bash
   cargo build --workspace --all-features
   ```

## Development Workflow

Use the Makefile for common development tasks:

```bash
make help          # Show all available commands
make quality       # Run all checks (check, clippy, fmt-check)
make test          # Run all tests
make pre-commit    # Run before committing (quality + test)
```

### Code Style

- Run `make fmt` before committing to ensure consistent formatting
- All code must pass `make clippy` without warnings
- Write tests for new functionality

## Pull Request Process

1. Fork the repository and create a feature branch
2. Make your changes with clear, descriptive commits
3. Ensure `make pre-commit` passes
4. Submit a pull request with a description of your changes

## Release Process

Releases are managed by maintainers. The process is:

1. Update version in `Cargo.toml` (workspace version)
2. Update `CHANGELOG.md` with release notes
3. Run `make release-check` to validate
4. Tag and push: `git tag v1.x.x && git push origin main --tags`
5. GitHub Actions will publish to crates.io automatically

### MSRV Policy

- Minimum Supported Rust Version: **1.85.0** (Edition 2024)
- MSRV bumps require a minor version increment
- Test MSRV locally with `make msrv-check`

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).
