use std::path::{Path, PathBuf};

use super::decoder::CodexDecoder;
use super::discovery::{SESSION_INDEX_FILE, read_session_index};
use super::header::read_codex_header;
use super::mapper::CodexToolMapper;
use crate::Result;
use crate::provider::{
    DecodeOptions, DiscoveryScope, FileHeader, LogDecoder, Provider, ProviderId, discover_files,
};
use crate::traits::ToolMapper;

/// Codex provider (`~/.codex/sessions/**/rollout-*.jsonl`).
pub struct CodexProvider;

impl Provider for CodexProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Codex
    }

    fn default_roots(&self) -> Vec<PathBuf> {
        agtrace_core::codex_sessions_root().into_iter().collect()
    }

    fn probe(&self, path: &Path) -> bool {
        path.extension().is_some_and(|e| e == "jsonl")
            && path
                .file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|f| f.starts_with("rollout-"))
    }

    fn read_header(&self, path: &Path) -> Result<Option<FileHeader>> {
        read_codex_header(path)
    }

    fn decoder(&self, header: &FileHeader, opts: DecodeOptions) -> Box<dyn LogDecoder> {
        Box::new(CodexDecoder::new(header, opts))
    }

    /// Headers of all rollouts in scope; root threads get their title from
    /// `<codex home>/session_index.jsonl` (next to the `sessions` root).
    fn discover(&self, scope: &DiscoveryScope) -> Result<Vec<FileHeader>> {
        let roots = scope.roots.clone().unwrap_or_else(|| self.default_roots());
        let mut headers = discover_files(self, &roots, scope.project_root.as_deref())?;
        let mut titles = std::collections::HashMap::new();
        for root in &roots {
            if let Some(home) = root.parent() {
                titles.extend(read_session_index(&home.join(SESSION_INDEX_FILE)));
            }
        }
        for header in &mut headers {
            if header.title.is_none() && header.agent.parent.is_none() {
                header.title = titles.get(header.agent.native_session_id.as_str()).cloned();
            }
        }
        Ok(headers)
    }

    fn tool_mapper(&self) -> &dyn ToolMapper {
        &CodexToolMapper
    }
}
