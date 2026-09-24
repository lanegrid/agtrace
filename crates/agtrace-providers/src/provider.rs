//! Provider contract: header reading, per-file stateful decoding, discovery.
//!
//! One agent = one file = one timeline. A [`LogDecoder`] is created per file and
//! fed complete lines in order (`line -> Vec<AgentEvent>`). Batch parsing
//! ([`decode_file`]) is "feed every line to a fresh decoder"; a live tailer feeds
//! only new lines to the same decoder.

use agtrace_types::{AgentEvent, AgentRef, ParseDiagnostics};
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::lenient::{LineReader, RawLine};
use crate::traits::ToolMapper;
use crate::{Error, Result};

/// Provider identifier (alias of [`agtrace_types::Provider`]).
pub type ProviderId = agtrace_types::Provider;

/// Stateful per-file decoder.
pub trait LogDecoder: Send {
    /// Decode one complete line. Never fails; problems go to diagnostics.
    fn decode_line(&mut self, line: RawLine<'_>) -> Vec<AgentEvent>;

    fn diagnostics(&self) -> &ParseDiagnostics;
}

/// Cheap, lenient header of an agent file.
#[derive(Debug, Clone, PartialEq)]
pub struct FileHeader {
    pub agent: AgentRef,
    /// Working directory of the project the agent ran in.
    pub project_cwd: Option<PathBuf>,
    pub title: Option<String>,
}

/// Options for creating a decoder.
#[derive(Debug, Clone, Default)]
pub struct DecodeOptions {
    /// Timestamp used for records before the first timestamped record
    /// (typically the file mtime). Records without a timestamp otherwise inherit
    /// the last seen one.
    pub fallback_timestamp: Option<DateTime<Utc>>,
}

/// Which files [`Provider::discover`] should consider.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryScope {
    /// Log roots to scan; `None` = the provider's default roots.
    pub roots: Option<Vec<PathBuf>>,
    /// Keep only agents whose project cwd is (under) this directory.
    pub project_root: Option<PathBuf>,
}

pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderId;

    /// Default log roots (`~/.claude/projects`, `~/.codex/sessions`, with env overrides).
    fn default_roots(&self) -> Vec<PathBuf>;

    /// Extension + location rules only (no content read).
    fn probe(&self, path: &Path) -> bool;

    /// Lenient header read. `Ok(None)` = not (yet) an agent file.
    fn read_header(&self, path: &Path) -> Result<Option<FileHeader>>;

    fn decoder(&self, header: &FileHeader, opts: DecodeOptions) -> Box<dyn LogDecoder>;

    /// Agent files in scope. Pure fs listing + headers.
    fn discover(&self, scope: &DiscoveryScope) -> Result<Vec<FileHeader>> {
        let roots = scope.roots.clone().unwrap_or_else(|| self.default_roots());
        discover_files(self, &roots, scope.project_root.as_deref())
    }

    fn tool_mapper(&self) -> &dyn ToolMapper;

    /// First user prompt of the file (truncated), for session lists. Bounded head read;
    /// `None` when there is none in the head or the provider has no notion of it.
    fn read_snippet(&self, _path: &Path) -> Option<String> {
        None
    }
}

/// Walk `roots`, keep probed files with a header, optionally filtered to agents whose
/// project cwd is under `project_root`. Shared by the default [`Provider::discover`].
pub fn discover_files<P: Provider + ?Sized>(
    provider: &P,
    roots: &[PathBuf],
    project_root: Option<&Path>,
) -> Result<Vec<FileHeader>> {
    let mut headers = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !entry.file_type().is_file() || !provider.probe(path) {
                continue;
            }
            let Ok(Some(header)) = provider.read_header(path) else {
                continue;
            };
            if let Some(project_root) = project_root {
                let in_project = header
                    .project_cwd
                    .as_deref()
                    .is_some_and(|cwd| cwd.starts_with(project_root));
                if !in_project {
                    continue;
                }
            }
            headers.push(header);
        }
    }
    Ok(headers)
}

/// File modification time as UTC.
pub(crate) fn file_mtime(path: &Path) -> Option<DateTime<Utc>> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .map(DateTime::<Utc>::from)
}

/// Decode a whole file with a fresh decoder.
///
/// Only I/O errors (and files that are not agent files) fail; content problems
/// are reported in the returned diagnostics.
pub fn decode_file(
    provider: &dyn Provider,
    path: &Path,
    mut opts: DecodeOptions,
) -> Result<(FileHeader, Vec<AgentEvent>, ParseDiagnostics)> {
    let header = provider.read_header(path)?.ok_or_else(|| {
        Error::Parse(format!(
            "Not a {} agent file: {}",
            provider.id(),
            path.display()
        ))
    })?;
    if opts.fallback_timestamp.is_none() {
        opts.fallback_timestamp = header.agent.started_at.or_else(|| file_mtime(path));
    }
    let mut decoder = provider.decoder(&header, opts);
    let reader = LineReader::new(BufReader::new(File::open(path)?), true);
    let mut events = Vec::new();
    for line in reader {
        let line = line?;
        events.extend(decoder.decode_line(line.as_raw()));
    }
    let diagnostics = decoder.diagnostics().clone();
    Ok((header, events, diagnostics))
}

/// Read the first `max_lines` complete lines of a file (header helpers).
pub(crate) fn read_head_lines(path: &Path, max_lines: usize) -> Result<Vec<String>> {
    let reader = LineReader::new(BufReader::new(File::open(path)?), true);
    let mut lines = Vec::new();
    for line in reader.take(max_lines) {
        lines.push(line?.text);
    }
    Ok(lines)
}
