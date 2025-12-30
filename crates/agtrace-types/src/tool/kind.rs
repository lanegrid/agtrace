use serde::{Deserialize, Serialize};

/// Tool classification by semantic purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// Read operations (files, resources, data)
    Read,
    /// Write operations (edit, create, patch)
    Write,
    /// Execute operations (shell commands, scripts)
    Execute,
    /// Planning operations (todo, task management)
    Plan,
    /// Search operations (web, file search, grep)
    Search,
    /// User interaction (questions, prompts)
    Ask,
    /// Other/unknown operations
    Other,
}

/// Tool origin classification
///
/// Distinguishes between provider-native tools and MCP protocol tools.
///
/// # Important
/// The origin is determined by how the tool is invoked, not by what it operates on:
/// - `System`: Tool is built-in to the provider and invoked directly by the LLM
/// - `Mcp`: Tool is invoked via MCP protocol (typically prefixed with `mcp__`)
///
/// # Examples
/// - `Bash` (Claude Code) → System (provider-native tool)
/// - `read_mcp_resource` (Codex) → System (provider-native tool that happens to read MCP resources)
/// - `mcp__sqlite__query` → Mcp (external tool invoked via MCP protocol)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOrigin {
    /// System-provided tool (built-in to the provider)
    System,
    /// MCP (Model Context Protocol) tool invoked via MCP protocol
    Mcp,
}
