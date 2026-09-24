use std::path::{Path, PathBuf};

use super::decoder::ClaudeDecoder;
use super::header::read_claude_header;
use super::mapper::ClaudeToolMapper;
use crate::Result;
use crate::provider::{DecodeOptions, FileHeader, LogDecoder, Provider, ProviderId};
use crate::traits::ToolMapper;

/// Claude Code provider (`~/.claude/projects/**.jsonl`).
pub struct ClaudeProvider;

impl Provider for ClaudeProvider {
    fn id(&self) -> ProviderId {
        ProviderId::ClaudeCode
    }

    fn default_roots(&self) -> Vec<PathBuf> {
        agtrace_core::claude_projects_root().into_iter().collect()
    }

    fn probe(&self, path: &Path) -> bool {
        path.extension().is_some_and(|e| e == "jsonl")
    }

    fn read_header(&self, path: &Path) -> Result<Option<FileHeader>> {
        read_claude_header(path)
    }

    fn decoder(&self, header: &FileHeader, opts: DecodeOptions) -> Box<dyn LogDecoder> {
        Box::new(ClaudeDecoder::new(header, opts))
    }

    fn tool_mapper(&self) -> &dyn ToolMapper {
        &ClaudeToolMapper
    }
}
