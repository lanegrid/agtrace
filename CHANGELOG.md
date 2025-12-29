# Changelog

All notable changes to this project will be documented in this file.

## [0.1.10] - 2025-12-29

### Documentation

- Split README into focused documentation structure (motivation, getting-started, commands, architecture, faq, providers)
- Consolidate provider documentation with accurate log paths
- Simplify documentation by removing redundant sections
- Add cargo install option to README

## [0.1.9] - 2025-12-29

### Bug Fixes

- Pass project_root to console mode handlers for correct display ([dc2c5c9](https://github.com/lanegrid/agtrace/commit/dc2c5c9751c7692049fa3b2dc99a5ecadbfb36b9))

- Watch should scan selected provider only, not all providers ([57a464f](https://github.com/lanegrid/agtrace/commit/57a464ff07246a08017ae16a0333bb5f93592a0e))

- Scan all providers before selecting most recent session for watch ([ec8c4b0](https://github.com/lanegrid/agtrace/commit/ec8c4b002efb5a4cc7a236882d85cff9dde92041))


### Features

- Display project_root and log_path in watch stream header ([678a606](https://github.com/lanegrid/agtrace/commit/678a6060661fab5a2a4ca28aaa5eaec093573da5))

- Enable cross-provider session switching in watch mode by tracking latest_mod_time ([3c40948](https://github.com/lanegrid/agtrace/commit/3c40948a2c98fb0dfe2ad4a4d4e46a37496f96c3))


### Miscellaneous Tasks

- Apply cargo fmt to demo.rs ([babfbde](https://github.com/lanegrid/agtrace/commit/babfbde99ba0c6bfb367193e0b3b79e610462ec0))


### Refactor

- Separate project_root and log_path in SessionState for accurate display ([298a649](https://github.com/lanegrid/agtrace/commit/298a649bd07dd13c2ecc6289aa033052ecd5156b))

- Unify console and TUI view models for watch mode ([bd6728d](https://github.com/lanegrid/agtrace/commit/bd6728da32c7e33a69048ab525e6e9cc12b128ef))

- Consolidate mod_time logic and add layer violation TODOs ([f104fe1](https://github.com/lanegrid/agtrace/commit/f104fe1fc4893c7d212920382bf00eebfc686090))


### Testing

- Add cross-provider session switching integration test ([200a00f](https://github.com/lanegrid/agtrace/commit/200a00f4631a695a92b1f6e4ca827ae4fec43d8c))


## [0.1.8] - 2025-12-28

### Documentation

- Rewrite README to emphasize observability layer and compaction behavior ([c8a669f](https://github.com/lanegrid/agtrace/commit/c8a669fac6db12214febc0796f3b32b62ce5d032))

- Rewrite README to emphasize observability layer and compaction behavior ([8fafcf4](https://github.com/lanegrid/agtrace/commit/8fafcf4680ba80e93decb634ed3d348cee8034a1))

- Clarify CWD-scoped monitoring and improve Quick Start workflow ([63e5c38](https://github.com/lanegrid/agtrace/commit/63e5c38e8d2c7d4e4d56576054055615c89237c7))

- Use GitHub raw content URLs for images and move demo.gif to docs/assets ([d4dbda6](https://github.com/lanegrid/agtrace/commit/d4dbda623e32e18cb8519bcced6dd22a70ec2e2d))


### Miscellaneous Tasks

- Remove PROGRESS.md ([0e728f5](https://github.com/lanegrid/agtrace/commit/0e728f5c627988d64e93cd640c6a30f73153c3bd))


## [0.1.7] - 2025-12-28

### Features

- Add demo mode to showcase TUI without requiring local logs ([a7f3261](https://github.com/lanegrid/agtrace/commit/a7f3261))

### Bug Fixes

- Change turn percentage display from cumulative to delta (incremental) ([65eeaa5](https://github.com/lanegrid/agtrace/commit/65eeaa5))
- Preserve all events in demo to prevent turn count reduction ([1bbe397](https://github.com/lanegrid/agtrace/commit/1bbe397))
- Link demo notifications to progress bar percentage instead of event index ([07f0577](https://github.com/lanegrid/agtrace/commit/07f0577))
- Unify progress bar calculation to include both input and output tokens ([49e0b5b](https://github.com/lanegrid/agtrace/commit/49e0b5b))
- Add context window limit enforcement to demo token generation ([f6771c5](https://github.com/lanegrid/agtrace/commit/f6771c5))
- Update demo model name and prevent context window overflow ([c708a9c](https://github.com/lanegrid/agtrace/commit/c708a9c))
- Assemble session from events to display turn data in demo mode ([7f77b87](https://github.com/lanegrid/agtrace/commit/7f77b87))
- Correct provider default log paths in help text ([0d6f5b8](https://github.com/lanegrid/agtrace/commit/0d6f5b8))

### Refactoring

- Unify --source option to --provider across CLI ([657cd40](https://github.com/lanegrid/agtrace/commit/657cd40))
- Rename source to provider in internal API ([b7ab5a4](https://github.com/lanegrid/agtrace/commit/b7ab5a4))
- Centralize CLI command hints to prevent duplication and typos ([6188a34](https://github.com/lanegrid/agtrace/commit/6188a34))
- Add scenario builder pattern and expand demo to 7 turns with 100+ events ([7917ffb](https://github.com/lanegrid/agtrace/commit/7917ffb))
- Unify token usage logic by using engine's extract_state_updates in demo ([a79f5b9](https://github.com/lanegrid/agtrace/commit/a79f5b9))
- Remove hardcoded context limit in demo, use configurable constant ([9277085](https://github.com/lanegrid/agtrace/commit/9277085))

### Documentation

- Add VHS demo gif and agtrace demo command documentation ([ea15513](https://github.com/lanegrid/agtrace/commit/ea15513))
- Regenerate demo.gif with cargo-installed agtrace v0.1.6 ([ef0b1c7](https://github.com/lanegrid/agtrace/commit/ef0b1c7))
- Reduce demo.gif size for better readability (1200x700) ([d328478](https://github.com/lanegrid/agtrace/commit/d328478))
- Organize demo generation scripts into scripts/demo directory ([acd1d46](https://github.com/lanegrid/agtrace/commit/acd1d46))
- Increase demo.gif font size for better readability (FontSize 18) ([508d1c1](https://github.com/lanegrid/agtrace/commit/508d1c1))
- Improve CLI help text and command descriptions for better UX ([0ae8b91](https://github.com/lanegrid/agtrace/commit/0ae8b91))
- Remove unnecessary documents ([98bb313](https://github.com/lanegrid/agtrace/commit/98bb313))
- Add centered logo to README header ([0327721](https://github.com/lanegrid/agtrace/commit/0327721))
- Add crates.io badge and cargo install instructions ([2908ebb](https://github.com/lanegrid/agtrace/commit/2908ebb))

## [0.1.6] - 2025-12-27

### Infrastructure

- Rename CLI package from `agtrace-cli` to `agtrace` for better discoverability on crates.io
- Add crates.io publishing automation to GitHub Actions release workflow
- Mark internal crates with `agtrace-internal` keyword to prevent accidental usage
- Add package metadata (categories, keywords, readme) for crates.io optimization

## [0.1.5] - 2025-12-27

### Bug Fixes

- Correct init command hints to suggest 'watch' and 'session list' instead of non-existent 'list' ([bf1ce4f](https://github.com/lanegrid/agtrace/commit/bf1ce4f))


### Documentation

- Reorder Quick Start to emphasize 'watch' workflow as primary use case ([bd9e03c](https://github.com/lanegrid/agtrace/commit/bd9e03c))

- Clarify Quick Start workflow with explicit agent launch steps and no-integration requirement ([09d7e29](https://github.com/lanegrid/agtrace/commit/09d7e29))

- Update screenshot to use Claude-specific dashboard image ([e504ee4](https://github.com/lanegrid/agtrace/commit/e504ee4))


## [0.1.4] - 2025-12-27

### Bug Fixes

- Watch command now selects provider with most recent session (issue #6) ([6802b22](https://github.com/lanegrid/agtrace/commit/6802b22))

- Perform session indexing during init before counting sessions (issue #5) ([7cf2bae](https://github.com/lanegrid/agtrace/commit/7cf2bae))

- Implement provider filtering in index commands ([72cd3af](https://github.com/lanegrid/agtrace/commit/72cd3af))

- Canonicalize paths in project_hash_from_root and add comprehensive integration tests ([834564c](https://github.com/lanegrid/agtrace/commit/834564c))


### Testing

- Add failing test that documents issue #5 bug (init reports 0 sessions before indexing) ([2b1a3d2](https://github.com/lanegrid/agtrace/commit/2b1a3d2))

- Add provider filtering tests with provider-agnostic test infrastructure ([ae99ec1](https://github.com/lanegrid/agtrace/commit/ae99ec1))


### Documentation

- Add test-driven bug fix strategy to AGENTS.md ([9d03968](https://github.com/lanegrid/agtrace/commit/9d03968))

- Update progress and bug status - all 21 integration tests passing ([5c51beb](https://github.com/lanegrid/agtrace/commit/5c51beb))


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
