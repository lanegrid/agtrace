//! Integration Tests for agtrace-cli
//!
//! This module contains comprehensive integration tests that verify
//! the CLI's behavior in realistic scenarios using the agtrace-testing
//! infrastructure.
//!
//! Test Organization:
//! - `project_isolation.rs`: Project hash isolation and data segregation
//! - `init_configuration.rs`: Initialization and configuration workflows
//! - `list_filtering.rs`: Session list filtering and queries
//! - `watch_command.rs`: Watch command and live session monitoring
//! - `provider_filtering.rs`: Provider-specific filtering behavior (--provider)
//!
//! All tests should pass with the current implementation.

mod init_configuration;
mod list_filtering;
mod project_isolation;
mod provider_filtering;
mod watch_command;
