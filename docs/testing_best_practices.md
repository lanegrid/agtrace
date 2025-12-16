# Rust Integration Tests: Common Code Best Practices

## Problem
Integration tests in `tests/` often need shared utilities (fixtures, helpers), but Clippy reports `dead_code` warnings because it cannot track cross-file usage within integration tests.

## Solutions (from Rust community best practices)

### Option A: `tests/common.rs` module (recommended for < 200 lines)

**Structure:**
```
tests/
├── common.rs          # Shared helpers
├── api_tests.rs
└── auth_tests.rs
```

**Implementation:**
```rust
// tests/common.rs
#![cfg(test)]  // Only compile in test builds

pub(crate) fn create_test_fixture() -> TestFixture {
    // ...
}
```

**Usage in test files:**
```rust
// tests/api_tests.rs
mod common;  // Import as module (NOT `use common;`)
use common::create_test_fixture;

#[test]
fn my_test() {
    let fixture = create_test_fixture();
    // ...
}
```

**Key points:**
- Use `mod common;` (module declaration), not `use`
- Mark with `#![cfg(test)]` to exclude from production builds
- Use `pub(crate)` instead of `pub` for better encapsulation

### Option B: Dedicated `test-utils` crate (for larger projects)

**When to use:**
- Shared code exceeds ~200 lines
- Multiple crates need the same test utilities
- Want proper dependency tracking by Clippy

**Structure:**
```
workspace/
├── crates/
│   ├── mylib/
│   │   ├── Cargo.toml
│   │   └── tests/
│   └── test-utils/
│       ├── Cargo.toml
│       └── src/lib.rs
```

**Setup:**
```toml
# crates/test-utils/Cargo.toml
[package]
name = "test-utils"
edition = "2021"

[dependencies]
assert_cmd = "2.0"
tempfile = "3.0"

# crates/mylib/Cargo.toml
[dev-dependencies]
test-utils = { path = "../test-utils" }
```

**Benefits:**
- Clippy correctly tracks usage across crates
- No `dead_code` warnings
- Explicit dependency management

## Handling Clippy Warnings

### Approach 1: Suppress at module level (for tests/common.rs)
```rust
// tests/common.rs
#![cfg(test)]
#![allow(dead_code)]  // Clippy can't track cross-test-file usage

pub(crate) fn helper() { /* ... */ }
```

### Approach 2: Conditional suppression
```rust
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn test_only_helper() { /* ... */ }
```

### Approach 3: CI configuration
Run Clippy with `--tests` flag to include test targets:
```bash
cargo clippy --all-targets --tests -- -D warnings
```

## Our Decision

For `agtrace-cli`:

**Current state:**
- Small amount of shared code (~100 lines)
- Used only within `agtrace-cli` tests

**Chosen approach: Option A** (`tests/common.rs`)
- Simple, idiomatic Rust
- No additional crate overhead
- Use `#![allow(dead_code)]` with clear documentation

**When to migrate to Option B:**
- Shared code exceeds 200 lines
- Multiple crates need the same test utilities
- Team size grows and wants explicit dependency tracking

## Implementation

```rust
// tests/common.rs
//! Common test utilities shared across integration tests.
//!
//! Note: Clippy cannot track usage across integration test files,
//! hence the `allow(dead_code)` annotation.
#![cfg(test)]
#![allow(dead_code)]

use assert_cmd::Command;
use tempfile::TempDir;

pub(crate) struct TestFixture {
    _temp_dir: TempDir,
    data_dir: PathBuf,
}

impl TestFixture {
    pub(crate) fn new() -> Self {
        // ...
    }
}
```

## References
- Rust Book: [Integration Tests](https://doc.rust-lang.org/book/ch11-03-test-organization.html#integration-tests)
- Clippy: [dead_code lint](https://rust-lang.github.io/rust-clippy/master/index.html#dead_code)
- Community: [Zero To Production In Rust](https://www.zero2prod.com/) (Chapter 3: Testing)
