# Changelog

All notable changes to this project will be documented in this file.

## [0.1.3] - 2025-12-25

### Bug Fixes

- Compute project_hash from SessionIndex.project_root instead of hardcoded 'unknown' (fixes #1) (#2) ([aca561a](https://github.com/lanegrid/agtrace/commit/aca561a282968cce9163e48fc7bedc0fe0fb938c))

- Ensure_index_is_fresh derives project_hash from cwd and respects --all-projects flag ([5615036](https://github.com/lanegrid/agtrace/commit/5615036a49692f72941d5a352f5663cb1c759339))


### Testing

- Add comprehensive integration tests for edge cases and project isolation ([bf75867](https://github.com/lanegrid/agtrace/commit/bf75867643157744a060a6f8f00f3af16b9a30f8))

- Fix project isolation tests with proper cwd/sessionId replacement to catch real bugs ([dacfc2f](https://github.com/lanegrid/agtrace/commit/dacfc2faca4fc0b33c833ddd3d3deeba90265397))

- Fix compilation errors in scan_legacy_project_hash_test and improve test helper formatting ([0d474a4](https://github.com/lanegrid/agtrace/commit/0d474a437a9bb26dabda3403887759b5ac035faf))


The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2025-12-25

### Added

- Initial public release on crates.io and npm
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

## [0.1.1] - 2025-12-25

_Internal development release - not published to crates.io or npm_

## [0.1.0] - 2025-12-25

_Internal development release - not published to crates.io or npm_
