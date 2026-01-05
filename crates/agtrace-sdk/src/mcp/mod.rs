//! MCP (Model Context Protocol) server implementation.
//!
//! This module provides an embeddable MCP server that allows AI agents
//! to query their own execution history via JSON-RPC over stdio.

pub mod error;
mod server;
mod tools;

pub use error::{ErrorCode, McpError};
pub use server::{AgTraceServer, run_server};
