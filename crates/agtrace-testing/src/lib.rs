//! Testing infrastructure for agtrace integration tests.
//!
//! This crate provides utilities for writing robust integration tests:
//! - `TestWorld`: Fluent interface for declarative test setup
//! - `assertions`: Custom assertions for agtrace-specific validation
//! - `fixtures`: Sample data generation and placement
//! - `live_fixture`: Writable copy of the v2026_09 fixture workspace (watcher tests)
//! - `providers`: Provider-specific testing utilities
//! - `synth`: Small builders for synthetic agents / events (unit tests)

pub mod assertions;
pub mod fixtures;
pub mod live_fixture;
pub mod providers;
pub mod synth;
pub mod world;

pub use world::TestWorld;
