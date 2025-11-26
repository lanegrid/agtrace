//! agtrace - Unify session histories from AI coding agents
//!
//! This library provides tools to parse and analyze session histories
//! from various AI coding agents like Claude Code and Codex.
//!
//! agtrace reads directly from agent data directories on each invocation
//! without persisting parsed data. This ensures the source directories
//! remain the single source of truth.
//!
//! # Example
//!
//! ```no_run
//! use agtrace::{parser, storage};
//! use std::path::Path;
//!
//! // Parse Claude Code sessions from default directory (~/.claude)
//! let executions = parser::claude_code::parse_default_dir()?;
//!
//! // Or parse from a custom directory
//! let executions = parser::claude_code::parse_dir(Path::new("/custom/path"))?;
//!
//! // List all executions from all agents
//! let all_executions = storage::list_all_executions()?;
//!
//! // Find a specific execution
//! let execution = storage::find_execution("session-id")?;
//! # Ok::<(), agtrace::error::Error>(())
//! ```

pub mod error;
pub mod model;
pub mod parser;
pub mod storage;

// Re-export commonly used types
pub use error::{Error, Result};
pub use model::{Agent, Event, Execution, ExecutionMetrics};
