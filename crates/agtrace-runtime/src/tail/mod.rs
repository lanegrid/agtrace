//! Offset-based incremental tailing (design §4.1).
//!
//! A [`FileCursor`] owns one agent file and the provider's stateful
//! [`LogDecoder`] for it. Each [`FileCursor::poll`] reads only the bytes appended
//! since the previous poll and feeds the newly completed lines to the decoder.
//! Batch decoding is the same thing started from offset 0, so
//! `decode_file(path) == tail(path)` for every file.
//!
//! - An incomplete trailing line is buffered and decoded once its `\n` arrives.
//!   If it has not grown for [`STALE_PARTIAL_AFTER`] it is decoded as-is (the
//!   writer crashed mid-line); a bad line is then counted in the diagnostics.
//! - Truncation (file shorter than what was consumed) or a change of file
//!   identity (the path now names a different file) resets the cursor: a fresh
//!   decoder re-reads the file from offset 0 and the poll reports
//!   [`TailOutcome::Reset`].
//! - Headers are read leniently from the head of the file, so a header read
//!   while the file is still being born can lack information that later lines
//!   provide (session id, cwd, start time). Until the file has more than
//!   [`HEADER_RECHECK_LINES`] lines the header is re-read after new lines were
//!   decoded; if it changed, the cursor restarts with the new header (a
//!   [`TailOutcome::Reset`]). This keeps tailing equivalent to batch decoding.

use agtrace_providers::{
    ClaudeProvider, CodexProvider, DecodeOptions, FileHeader, LineReader, LogDecoder,
    ParseDiagnostics, Provider, ProviderId,
};
use agtrace_types::AgentEvent;
use chrono::{DateTime, Utc};
use std::fs::{File, Metadata};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Size of a single read from the file.
pub(crate) const READ_CHUNK: usize = 256 * 1024;

/// A partial last line that has not grown for this long is decoded as-is.
pub(crate) const STALE_PARTIAL_AFTER: Duration = Duration::from_secs(5);

/// Providers derive the header from at most this many head lines; once the
/// header has been read with more complete lines available it is final.
pub(crate) const HEADER_RECHECK_LINES: u64 = 64;

/// Upper bound of header-change restarts within one poll (the file can only be
/// re-read so often while it is being born).
const MAX_RESTARTS_PER_POLL: usize = 3;

/// The provider implementation for a provider id.
pub(crate) fn provider_for(id: ProviderId) -> Arc<dyn Provider> {
    match id {
        ProviderId::ClaudeCode => Arc::new(ClaudeProvider),
        ProviderId::Codex => Arc::new(CodexProvider),
    }
}

/// Identity of the file behind a path, used to detect replacement (rotation,
/// delete + recreate) that keeps or grows the size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileIdentity {
    /// `(dev, ino)` on unix.
    #[cfg(unix)]
    Inode { dev: u64, ino: u64 },
    /// Creation time where inodes are not available.
    #[cfg_attr(unix, allow(dead_code))]
    Created(Option<std::time::SystemTime>),
}

impl FileIdentity {
    pub(crate) fn of(meta: &Metadata) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            FileIdentity::Inode {
                dev: meta.dev(),
                ino: meta.ino(),
            }
        }
        #[cfg(not(unix))]
        {
            FileIdentity::Created(meta.created().ok())
        }
    }
}

/// Result of one [`FileCursor::poll`].
#[derive(Debug)]
pub(crate) enum TailOutcome {
    /// Nothing new was decoded.
    Unchanged,
    /// Events decoded from newly appended lines.
    Appended(Vec<AgentEvent>),
    /// The file was truncated or replaced, or its header changed while the file
    /// was being born; the cursor restarted from offset 0 and these are all
    /// events of the file as it is now. Previously delivered events of this file
    /// must be discarded.
    Reset(Vec<AgentEvent>),
}

impl TailOutcome {
    #[cfg(test)]
    pub(crate) fn events(&self) -> &[AgentEvent] {
        match self {
            TailOutcome::Unchanged => &[],
            TailOutcome::Appended(e) | TailOutcome::Reset(e) => e,
        }
    }
}

/// Incremental reader of one agent file.
pub(crate) struct FileCursor {
    path: PathBuf,
    provider: Arc<dyn Provider>,
    /// Options given by the caller; `fallback_timestamp: None` means "derive it
    /// like `decode_file` does" (header start time, else file mtime).
    opts: DecodeOptions,
    stale_after: Duration,
    /// Identity of the file the current state was read from.
    file_id: Option<FileIdentity>,
    /// First byte not yet consumed (= start of `partial`).
    offset: u64,
    /// Next line number.
    line: u64,
    /// Bytes after the last consumed `\n` (incomplete line, or lines buffered
    /// while the header is not readable yet).
    partial: Vec<u8>,
    /// When `partial` last grew.
    partial_since: Option<Instant>,
    header: Option<FileHeader>,
    /// Lines consumed when the header was last (re)read.
    header_lines: u64,
    decoder: Option<Box<dyn LogDecoder>>,
}

impl FileCursor {
    /// Create a cursor positioned at offset 0. Nothing is read until [`poll`](Self::poll).
    pub(crate) fn new(provider: Arc<dyn Provider>, path: PathBuf, opts: DecodeOptions) -> Self {
        Self {
            path,
            provider,
            opts,
            stale_after: STALE_PARTIAL_AFTER,
            file_id: None,
            offset: 0,
            line: 0,
            partial: Vec::new(),
            partial_since: None,
            header: None,
            header_lines: 0,
            decoder: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_stale_after(mut self, stale_after: Duration) -> Self {
        self.stale_after = stale_after;
        self
    }

    /// Header of the file, once it has been readable.
    pub(crate) fn header(&self) -> Option<&FileHeader> {
        self.header.as_ref()
    }

    /// Diagnostics of the current decoder (reset together with the cursor).
    pub(crate) fn diagnostics(&self) -> Option<&ParseDiagnostics> {
        self.decoder.as_ref().map(|d| d.diagnostics())
    }

    /// End of the bytes read so far (consumed + buffered partial). A file whose
    /// length differs has new (or truncated) content.
    pub(crate) fn read_end(&self) -> u64 {
        self.offset + self.partial.len() as u64
    }

    /// True while an incomplete line is buffered (it must be polled again so the
    /// stale-partial timeout can fire).
    pub(crate) fn has_partial(&self) -> bool {
        !self.partial.is_empty()
    }

    /// First byte not yet consumed.
    #[cfg(test)]
    pub(crate) fn offset(&self) -> u64 {
        self.offset
    }

    /// Next line number.
    #[cfg(test)]
    pub(crate) fn line(&self) -> u64 {
        self.line
    }

    /// Read `[offset, EOF)` and decode the newly completed lines.
    pub(crate) fn poll(&mut self) -> io::Result<TailOutcome> {
        self.poll_at(Instant::now())
    }

    /// [`poll`](Self::poll) with an explicit clock (stale-partial timeout).
    pub(crate) fn poll_at(&mut self, now: Instant) -> io::Result<TailOutcome> {
        let mut file = File::open(&self.path)?;
        let meta = file.metadata()?;
        let identity = FileIdentity::of(&meta);
        let len = meta.len();
        let buffered_end = self.offset + self.partial.len() as u64;

        let mut reset = match self.file_id {
            Some(prev) => prev != identity || len < buffered_end,
            None => false,
        };
        if reset {
            self.restart();
        }
        self.file_id = Some(identity);

        let mut events = Vec::new();
        let mut restarts = 0;
        loop {
            self.read_appended(&mut file, len, &meta, now, &mut events)?;
            if restarts >= MAX_RESTARTS_PER_POLL || !self.header_may_have_changed() {
                break;
            }
            match self.provider.read_header(&self.path) {
                Ok(h) if h.as_ref() == self.header.as_ref() => {
                    self.header_lines = self.line;
                    break;
                }
                Ok(_) => {
                    // The header learned something from the new lines: decode
                    // the file again from the start with the final header.
                    self.restart();
                    self.file_id = Some(identity);
                    events.clear();
                    reset = true;
                    restarts += 1;
                }
                Err(agtrace_providers::Error::Io(e)) => return Err(e),
                Err(_) => break,
            }
        }

        Ok(if reset {
            TailOutcome::Reset(events)
        } else if events.is_empty() {
            TailOutcome::Unchanged
        } else {
            TailOutcome::Appended(events)
        })
    }

    /// Read `[offset + partial, EOF)`, decode the completed lines and handle a
    /// stale partial line.
    fn read_appended(
        &mut self,
        file: &mut File,
        len: u64,
        meta: &Metadata,
        now: Instant,
        events: &mut Vec<AgentEvent>,
    ) -> io::Result<()> {
        let read_from = self.offset + self.partial.len() as u64;
        if len > read_from {
            file.seek(SeekFrom::Start(read_from))?;
            let mut chunk = vec![0u8; READ_CHUNK];
            loop {
                let n = file.read(&mut chunk)?;
                if n == 0 {
                    break;
                }
                self.partial.extend_from_slice(&chunk[..n]);
                self.partial_since = Some(now);
                self.drain(meta, false, events)?;
            }
        } else if !self.partial.is_empty() {
            // Nothing new; the header may have become readable meanwhile (it is
            // read from disk), or the partial line may have gone stale.
            self.drain(meta, false, events)?;
            let stale = self
                .partial_since
                .is_some_and(|since| now.saturating_duration_since(since) >= self.stale_after);
            if stale && !self.partial.is_empty() {
                self.drain(meta, true, events)?;
            }
        }
        Ok(())
    }

    /// Whether the header was read from a head that has grown since.
    fn header_may_have_changed(&self) -> bool {
        self.header.is_some()
            && self.header_lines < HEADER_RECHECK_LINES
            && self.line > self.header_lines
    }

    /// Forget everything read so far (truncation / identity change).
    fn restart(&mut self) {
        self.file_id = None;
        self.offset = 0;
        self.line = 0;
        self.partial.clear();
        self.partial_since = None;
        self.header = None;
        self.header_lines = 0;
        self.decoder = None;
    }

    /// Decode the complete lines buffered in `partial` (and, with
    /// `flush_incomplete`, the trailing incomplete line too).
    fn drain(
        &mut self,
        meta: &Metadata,
        flush_incomplete: bool,
        out: &mut Vec<AgentEvent>,
    ) -> io::Result<()> {
        let has_complete_line = self.partial.contains(&b'\n');
        if !has_complete_line && !flush_incomplete {
            return Ok(());
        }
        if self.decoder.is_none() && !self.open_decoder(meta)? {
            // Not (yet) an agent file: keep the bytes and retry on a later poll.
            return Ok(());
        }
        let decoder = self.decoder.as_mut().expect("decoder opened above");

        let mut reader = LineReader::starting_at(
            self.partial.as_slice(),
            self.offset,
            self.line,
            flush_incomplete,
        );
        while let Some(line) = reader.next_line()? {
            out.extend(decoder.decode_line(line.as_raw()));
        }
        let consumed = (reader.offset() - self.offset) as usize;
        self.offset = reader.offset();
        self.line = reader.line();
        self.partial.drain(..consumed);
        if self.partial.is_empty() {
            self.partial_since = None;
        }
        Ok(())
    }

    /// Read the header and create the decoder. `Ok(false)` when the file is not
    /// (yet) an agent file.
    fn open_decoder(&mut self, meta: &Metadata) -> io::Result<bool> {
        let header = match self.provider.read_header(&self.path) {
            Ok(Some(h)) => h,
            Ok(None) => return Ok(false),
            Err(agtrace_providers::Error::Io(e)) => return Err(e),
            // Content problems in the head are not fatal: retry later.
            Err(_) => return Ok(false),
        };
        let mut opts = self.opts.clone();
        if opts.fallback_timestamp.is_none() {
            opts.fallback_timestamp = header
                .agent
                .started_at
                .or_else(|| meta.modified().ok().map(DateTime::<Utc>::from));
        }
        self.decoder = Some(self.provider.decoder(&header, opts));
        self.header = Some(header);
        self.header_lines = self.line;
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
