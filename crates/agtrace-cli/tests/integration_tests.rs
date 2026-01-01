//! Integration Tests for agtrace-cli
//!
//! This module contains CLI-specific integration tests that verify
//! command-line interface behavior including argument parsing, output formatting,
//! and TUI rendering.
//!
//! Test Organization:
//! - `init_configuration.rs`: Init command output and configuration workflows
//! - `watch_command.rs`: Watch command TUI and console mode
//! - `provider_filtering.rs`: CLI provider filtering (index/watch/lab commands)
//! - `debug_session_discovery.rs`: Debug utilities
//!
//! Note: Core business logic tests (session filtering, project isolation)
//! have been moved to agtrace-sdk/tests for faster, type-safe testing.
//!
//! Note: `help_snapshots.rs` is a separate test file to maintain stable snapshot paths.

mod init_configuration;
mod provider_filtering;
mod watch_command;
