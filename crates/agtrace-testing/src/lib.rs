//! Testing infrastructure for agtrace integration tests.
//!
//! This crate provides utilities for writing robust integration tests:
//! - `TestWorld`: Fluent interface for declarative test setup
//! - `assertions`: Custom assertions for agtrace-specific validation
//! - `fixtures`: Sample data generation and placement
//! - `process`: Background process management for long-running commands
//! - `providers`: Provider-specific testing utilities

pub mod assertions;
pub mod fixtures;
pub mod process;
pub mod providers;
pub mod world;

pub use world::TestWorld;
