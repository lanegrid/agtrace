use std::path::{Path, PathBuf};

use super::decoder::ClaudeDecoder;
use super::discovery::{agent_files_in, is_agent_file_path, project_dirs};
use super::header::read_claude_header;
use super::mapper::ClaudeToolMapper;
use crate::Result;
use crate::provider::{
    DecodeOptions, DiscoveryScope, FileHeader, LogDecoder, Provider, ProviderId,
};
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
        is_agent_file_path(path)
    }

    fn read_header(&self, path: &Path) -> Result<Option<FileHeader>> {
        read_claude_header(path)
    }

    fn decoder(&self, header: &FileHeader, opts: DecodeOptions) -> Box<dyn LogDecoder> {
        Box::new(ClaudeDecoder::new(header, opts))
    }

    /// With a project root, only that project's dirs (and dirs below it / aliases) are
    /// listed instead of walking every project. Headers are still filtered by cwd.
    fn discover(&self, scope: &DiscoveryScope) -> Result<Vec<FileHeader>> {
        let Some(project_root) = &scope.project_root else {
            let roots = scope.roots.clone().unwrap_or_else(|| self.default_roots());
            return crate::provider::discover_files(self, &roots, None);
        };
        let roots = scope.roots.clone().unwrap_or_else(|| self.default_roots());
        let mut headers = Vec::new();
        for root in roots {
            for dir in project_dirs(&root, project_root) {
                for file in agent_files_in(&dir) {
                    let Ok(Some(header)) = self.read_header(&file) else {
                        continue;
                    };
                    let in_project = header
                        .project_cwd
                        .as_deref()
                        .is_some_and(|cwd| cwd.starts_with(project_root));
                    if in_project {
                        headers.push(header);
                    }
                }
            }
        }
        Ok(headers)
    }

    fn tool_mapper(&self) -> &dyn ToolMapper {
        &ClaudeToolMapper
    }
}
