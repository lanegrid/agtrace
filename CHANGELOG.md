# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Documentation

- Improve README ([ee66d7a](https://github.com/lanegrid/agtrace/commit/ee66d7abb984fc6a9abb0823138887b3ae52301e))

## [0.1.1] - 2024-12-24

### Added

- Initial public release on crates.io
- Core library APIs for AI agent log analysis
  - Multi-provider normalization (Claude Code, Codex, Gemini)
  - Session parsing and event stream processing
  - SQLite-based indexing with schema-on-read architecture
- CLI commands:
  - `init` - Initialize workspace and detect providers
  - `list` - Show session history
  - `show` - Display session details with token usage
  - `watch` - Real-time TUI dashboard for live sessions
  - `doctor` - Verify configuration and database integrity
  - `lab grep` - Search across sessions with regex and JSON output
- Features:
  - Live context window monitoring
  - Token usage tracking (input/output/cache/reasoning)
  - Provider-agnostic event normalization
  - Local-only processing (no cloud dependencies)
  - Zero-overhead monitoring with Rust performance

### Fixed

- Prevent panic when session_id is shorter than 8 characters in watch mode

## [0.1.0] - 2024-12-24

_Internal development release - not published to crates.io_
