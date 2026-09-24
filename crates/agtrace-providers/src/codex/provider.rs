use std::path::{Path, PathBuf};

use super::decoder::CodexDecoder;
use super::header::read_codex_header;
use super::mapper::CodexToolMapper;
use crate::Result;
use crate::provider::{DecodeOptions, FileHeader, LogDecoder, Provider, ProviderId};
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

    fn tool_mapper(&self) -> &dyn ToolMapper {
        &CodexToolMapper
    }
}
