# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-22

### Added
- **MCP Server Implementation**: Added a Model Context Protocol (MCP) server for integration with AI assistants.
- **Chrome Lifecycle Management**: Implemented on-demand Chrome launching and automatic shutdown to prevent resource leaks.
- **Cross-platform Installation**: Added `install.sh` (macOS/Linux) and `install.ps1` (Windows) for easier installation.
- **Developer Experience**: Added `mise.toml` for task management and devcontainer support.
- **License**: Added MIT License.

### Changed
- Refined error handling using `thiserror`.
- Improved E2E test coverage and CI reliability.
- Aligned MCP and CLI arguments for consistency.

### Fixed
- Fixed tab leaks in the Chrome browser session.
- Fixed CI failures on GitHub Actions.

## [0.0.5] - 2026-02-15
- Initial public release of early prototype.
