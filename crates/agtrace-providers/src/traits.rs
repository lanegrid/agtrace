use agtrace_types::{AgentEvent, ParseDiagnostics, ToolCallPayload, ToolKind, ToolOrigin};
use serde_json::Value;
use std::path::Path;

use crate::provider::{DecodeOptions, FileHeader, Provider};
use crate::{Error, Result};

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

// --- Provider Adapter ---

/// Adapter bundling the provider contract ([`Provider`]: header, decoder, discovery)
/// with its tool mapper.
pub struct ProviderAdapter {
    pub provider: Box<dyn Provider>,
    pub mapper: Box<dyn ToolMapper>,
}

impl ProviderAdapter {
    pub fn new(provider: Box<dyn Provider>, mapper: Box<dyn ToolMapper>) -> Self {
        Self { provider, mapper }
    }

    /// Create adapter for a provider by name
    pub fn from_name(provider_name: &str) -> Result<Self> {
        match provider_name {
            "claude_code" | "claude" => Ok(Self::claude()),
            "codex" => Ok(Self::codex()),
            _ => Err(Error::Provider(format!(
                "Unknown provider: {}",
                provider_name
            ))),
        }
    }

    /// Create Claude provider adapter
    pub fn claude() -> Self {
        Self::new(
            Box::new(crate::claude::ClaudeProvider),
            Box::new(crate::claude::ClaudeToolMapper),
        )
    }

    /// Create Codex provider adapter
    pub fn codex() -> Self {
        Self::new(
            Box::new(crate::codex::CodexProvider),
            Box::new(crate::codex::CodexToolMapper),
        )
    }

    /// Provider name (`claude_code`, `codex`).
    pub fn id(&self) -> &'static str {
        self.provider.id().as_str()
    }

    /// Location rule of the provider (no content read).
    pub fn probe(&self, path: &Path) -> bool {
        self.provider.probe(path)
    }

    /// Decode a whole file (lenient per line), returning header, events and diagnostics.
    pub fn decode_file(
        &self,
        path: &Path,
    ) -> Result<(FileHeader, Vec<AgentEvent>, ParseDiagnostics)> {
        crate::provider::decode_file(self.provider.as_ref(), path, DecodeOptions::default())
    }

    /// Parse a whole file into events (lenient per line; only I/O errors fail).
    pub fn parse_file(&self, path: &Path) -> Result<Vec<AgentEvent>> {
        self.decode_file(path).map(|(_, events, _)| events)
    }

    /// Process a file through the adapter (convenience method)
    pub fn process_file(&self, path: &Path) -> Result<Vec<AgentEvent>> {
        if !self.probe(path) {
            return Err(Error::Provider(format!(
                "Provider {} cannot handle file: {}",
                self.id(),
                path.display()
            )));
        }
        self.parse_file(path)
    }
}
