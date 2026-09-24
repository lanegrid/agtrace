// Error types
pub mod error;

// Trait-based architecture (public API)
pub mod traits;

// Provider contract (header + per-file lenient decoder)
pub mod lenient;
pub mod provider;

// Provider implementations
pub mod claude;
pub mod codex;

// Event builder
pub mod builder;

// Provider registry
pub mod registry;

// Model catalog (context window tables + provider caches)
pub mod model_catalog;
pub use model_catalog::BuiltinModelCatalog;

// Tool analysis
pub mod tool_analyzer;

// Tool specification
pub(crate) mod tool_spec;

// Traits
pub use traits::{
    LogDiscovery, ProbeResult, ProviderAdapter, SessionIndex, ToolMapper,
    get_latest_mod_time_rfc3339,
};

// Provider contract
pub use agtrace_types::{LineError, ParseDiagnostics};
pub use lenient::{LineReader, OwnedLine, RawLine};
pub use provider::{
    DecodeOptions, DiscoveryScope, FileHeader, LogDecoder, Provider, ProviderId, decode_file,
};

// Provider normalize functions
pub use claude::{ClaudeProvider, normalize_claude_file};
pub use codex::{CodexProvider, normalize_codex_file};

// MCP utilities (provider-specific namespaces)
pub mod mcp {
    /// Claude Code MCP utilities
    pub mod claude {
        pub use crate::claude::{mcp_server_name, mcp_tool_name, parse_mcp_name};
    }
    /// Codex MCP utilities
    pub mod codex {
        pub use crate::codex::{mcp_server_name, mcp_tool_name, parse_mcp_name};
    }
}

// Registry
pub use registry::{
    create_adapter, create_all_adapters, detect_adapter_from_path, get_all_providers,
    get_default_log_paths, get_provider_metadata, get_provider_names,
};

// Tool analyzer
pub use tool_analyzer::{classify_common, extract_common_summary, truncate};

// Error types
pub use error::{Error, Result};
