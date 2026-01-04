//! Integration tests for agtrace-sdk
//!
//! These tests verify SDK functionality without going through the CLI layer.
//! They use the SDK's public API directly for faster, type-safe testing.

mod scenarios {
    mod filtering;
    mod isolation;
    mod spawn_context;
    mod watch;
}
