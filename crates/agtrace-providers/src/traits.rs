use agtrace_types::{AgentEvent, ToolCallPayload, ToolKind, ToolOrigin};
use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Provider discovery and lifecycle management
///
/// Responsibilities:
/// - Identify provider from file paths/patterns
/// - Locate session files on filesystem
/// - Extract session metadata
pub trait LogProvider: Send + Sync {
    /// Unique provider ID (e.g., "claude", "codex", "gemini")
    fn id(&self) -> &'static str;

    /// Check if a file belongs to this provider
    fn probe(&self, path: &Path) -> ProbeResult;

    /// Resolve log root directory for a given project root
    /// Returns None if provider doesn't organize by project
    fn resolve_log_root(&self, project_root: &Path) -> Option<PathBuf>;

    /// Scan for sessions in the log root
    fn scan_sessions(&self, log_root: &Path) -> Result<Vec<SessionIndex>>;

    /// Extract session ID from file header (lightweight, no full parse)
    fn extract_session_id(&self, path: &Path) -> Result<String>;

    /// Find all files belonging to a session (main + sidechains)
    fn find_session_files(&self, log_root: &Path, session_id: &str) -> Result<Vec<PathBuf>>;
}

/// Session data normalization
///
/// Responsibilities:
/// - Parse raw log files into structured events
/// - Handle format differences (JSONL, JSON array, custom)
/// - Support streaming/incremental parsing
pub trait SessionParser: Send + Sync {
    /// Parse entire file into event stream
    fn parse_file(&self, path: &Path) -> Result<Vec<AgentEvent>>;

    /// Parse single record for streaming (e.g., tail -f mode)
    /// Returns None for malformed/incomplete lines (non-fatal)
    fn parse_record(&self, content: &str) -> Result<Option<AgentEvent>>;
}

/// Tool call semantic interpretation
///
/// Responsibilities:
/// - Classify tools by origin and kind
/// - Normalize provider-specific tool arguments to domain model
/// - Extract UI summaries for display
pub trait ToolMapper: Send + Sync {
    /// Classify tool by origin (System/Mcp) and kind (Read/Write/Execute/etc.)
    fn classify(&self, tool_name: &str) -> (ToolOrigin, ToolKind);

    /// Normalize provider-specific tool call to domain ToolCallPayload
    fn normalize_call(&self, name: &str, args: Value, call_id: Option<String>) -> ToolCallPayload;

    /// Extract short summary for UI display
    fn summarize(&self, kind: ToolKind, args: &Value) -> String;
}

// --- Helper types ---

/// Probe result with confidence score
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProbeResult {
    /// Provider can handle this file with given confidence (0.0 - 1.0)
    Confidence(f32),
    /// Provider cannot handle this file
    NoMatch,
}

impl ProbeResult {
    /// Create high confidence match (1.0)
    pub fn match_high() -> Self {
        ProbeResult::Confidence(1.0)
    }

    /// Create medium confidence match (0.5)
    pub fn match_medium() -> Self {
        ProbeResult::Confidence(0.5)
    }

    /// Create low confidence match (0.3)
    pub fn match_low() -> Self {
        ProbeResult::Confidence(0.3)
    }

    /// Check if this is a match (confidence > 0)
    pub fn is_match(&self) -> bool {
        matches!(self, ProbeResult::Confidence(c) if *c > 0.0)
    }

    /// Get confidence score (0.0 if NoMatch)
    pub fn confidence(&self) -> f32 {
        match self {
            ProbeResult::Confidence(c) => *c,
            ProbeResult::NoMatch => 0.0,
        }
    }
}

/// Session index metadata
#[derive(Debug, Clone)]
pub struct SessionIndex {
    pub session_id: String,
    pub timestamp: Option<String>,
    pub main_file: PathBuf,
    pub sidechain_files: Vec<PathBuf>,
}

// --- Provider Adapter ---

/// Adapter that bundles the three trait implementations
///
/// This provides a unified interface for working with provider functionality
/// while maintaining clean separation of concerns internally.
pub struct ProviderAdapter {
    pub discovery: Box<dyn LogProvider>,
    pub parser: Box<dyn SessionParser>,
    pub mapper: Box<dyn ToolMapper>,
}

impl ProviderAdapter {
    pub fn new(
        discovery: Box<dyn LogProvider>,
        parser: Box<dyn SessionParser>,
        mapper: Box<dyn ToolMapper>,
    ) -> Self {
        Self {
            discovery,
            parser,
            mapper,
        }
    }

    /// Create adapter for a provider by name
    pub fn from_name(provider_name: &str) -> Result<Self> {
        match provider_name {
            "claude_code" | "claude" => Ok(Self::claude()),
            "codex" => Ok(Self::codex()),
            "gemini" => Ok(Self::gemini()),
            _ => anyhow::bail!("Unknown provider: {}", provider_name),
        }
    }

    /// Create Claude provider adapter
    pub fn claude() -> Self {
        Self::new(
            Box::new(crate::claude::ClaudeDiscovery),
            Box::new(crate::claude::ClaudeParser),
            Box::new(crate::claude::ClaudeToolMapper),
        )
    }

    /// Create Codex provider adapter
    pub fn codex() -> Self {
        Self::new(
            Box::new(crate::codex::CodexDiscovery),
            Box::new(crate::codex::CodexParser),
            Box::new(crate::codex::CodexToolMapper),
        )
    }

    /// Create Gemini provider adapter
    pub fn gemini() -> Self {
        Self::new(
            Box::new(crate::gemini::GeminiDiscovery),
            Box::new(crate::gemini::GeminiParser),
            Box::new(crate::gemini::GeminiToolMapper),
        )
    }

    /// Get provider ID
    pub fn id(&self) -> &'static str {
        self.discovery.id()
    }

    /// Process a file through the adapter (convenience method)
    pub fn process_file(&self, path: &Path) -> Result<Vec<AgentEvent>> {
        if !self.discovery.probe(path).is_match() {
            anyhow::bail!(
                "Provider {} cannot handle file: {}",
                self.id(),
                path.display()
            );
        }
        self.parser.parse_file(path)
    }
}
