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

// Tool analysis
pub mod tool_analyzer;

// Tool specification
pub(crate) mod tool_spec;

// Legacy types and backward compatibility
pub mod legacy;

// Traits
pub use traits::{
    LogDiscovery, ProbeResult, ProviderAdapter, SessionIndex, SessionParser, ToolMapper,
    get_latest_mod_time_rfc3339,
};

// Provider normalize functions
pub use claude::normalize_claude_file;
pub use codex::normalize_codex_file;
pub use gemini::normalize_gemini_file;

// Registry
pub use registry::{
    create_adapter, create_all_adapters, detect_adapter_from_path, get_all_providers,
    get_default_log_paths, get_provider_metadata, get_provider_names,
};

// Tool analyzer
pub use tool_analyzer::{classify_common, extract_common_summary, truncate};

// Normalization
pub use normalization::normalize_tool_call;

// Legacy types
pub use legacy::{LogFileMetadata, ScanContext, SessionMetadata};
