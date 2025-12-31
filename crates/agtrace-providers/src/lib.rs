// Error types
pub mod error;

// Trait-based architecture (public API)
pub mod traits;

// Provider implementations
pub mod claude;
pub mod codex;
pub mod gemini;

// Event builder
pub mod builder;

// Tool call normalization
pub mod normalization;

// Provider registry
pub mod registry;

// Token limits resolution
pub mod token_limits;
pub use token_limits::ProviderModelLimitResolver;

// Tool analysis
pub mod tool_analyzer;

// Tool specification
pub(crate) mod tool_spec;

// Traits
pub use traits::{
    LogDiscovery, ProbeResult, ProviderAdapter, SessionIndex, SessionParser, ToolMapper,
    get_latest_mod_time_rfc3339,
};

// Provider normalize functions
pub use claude::normalize_claude_file;
pub use codex::normalize_codex_file;
pub use gemini::normalize_gemini_file;

// Claude Code MCP utilities
pub use claude::{mcp_server_name, mcp_tool_name, parse_mcp_name};

// Registry
pub use registry::{
    create_adapter, create_all_adapters, detect_adapter_from_path, get_all_providers,
    get_default_log_paths, get_provider_metadata, get_provider_names,
};

// Tool analyzer
pub use tool_analyzer::{classify_common, extract_common_summary, truncate};

// Normalization
pub use normalization::normalize_tool_call;

// Error types
pub use error::{Error, Result};
