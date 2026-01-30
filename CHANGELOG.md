# Changelog

All notable changes to Horizon Lattice will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Versioning Policy

Horizon Lattice follows Semantic Versioning:

- **Major** (X.0.0): Breaking API changes
- **Minor** (0.X.0): New features, backwards compatible
- **Patch** (0.0.X): Bug fixes, backwards compatible

### Pre-release Versions

During initial development (0.x.y), the versioning scheme is:

- **Alpha** (`0.x.0-alpha.N`): Active development, API unstable
- **Beta** (`0.x.0-beta.N`): Feature complete for release, testing phase
- **RC** (`0.x.0-rc.N`): Release candidate, final testing

### Breaking Changes

Breaking changes will only occur in major version bumps (e.g., 1.x to 2.x).

## [Unreleased]

## [1.0.1] - 2026-01-30

### Fixed

- Added README.md to all crate packages for crates.io display
- Fixed repository URLs in documentation (book.toml)
- Increased crates.io publish delays to avoid rate limiting

### Added

- GitHub Pages deployment script (`scripts/deploy-docs.sh`)
- Makefile targets for documentation: `book`, `book-serve`, `book-deploy`

## [1.0.0] - 2026-01-28

### Added

- Core crate (`horizon-lattice-core`) with event loop, object model, and signals
- Macros crate (`horizon-lattice-macros`) with `#[derive(Object)]`, `#[property]`, and `#[signal]`
- Render crate (`horizon-lattice-render`) with wgpu backend
- Style crate (`horizon-lattice-style`) with CSS-like styling and theming
- Network crate (`horizon-lattice-net`) with HTTP, WebSocket, and TCP/UDP support
- Multimedia crate (`horizon-lattice-multimedia`) with audio playback
- Main crate (`horizon-lattice`) with widget library and platform integration
- Comprehensive widget set: buttons, labels, text inputs, containers, dialogs
- Layout system: HBox, VBox, Grid, Form layouts
- Platform integration: clipboard, file dialogs, system tray, notifications
- Accessibility support via accesskit
- Documentation with tutorials and examples

[Unreleased]: https://github.com/horizonanalytic/lattice/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/horizonanalytic/lattice/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/horizonanalytic/lattice/releases/tag/v1.0.0
