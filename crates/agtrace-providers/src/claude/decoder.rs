//! Line-oriented Claude Code decoder (`LogDecoder`).
//!
//! Each line is decoded leniently into a [`ClaudeRecord`] and mapped to events by
//! the stateful [`ClaudeRecordMapper`]. Problems are counted in diagnostics.

use agtrace_types::{AgentEvent, ParseDiagnostics};

use super::parser::ClaudeRecordMapper;
use super::schema::ClaudeRecord;
use crate::lenient::{RawLine, decode_typed, kind_label_of};
use crate::provider::{DecodeOptions, FileHeader, LogDecoder};

pub struct ClaudeDecoder {
    mapper: ClaudeRecordMapper,
    diagnostics: ParseDiagnostics,
}

impl ClaudeDecoder {
    pub fn new(header: &FileHeader, opts: DecodeOptions) -> Self {
        Self {
            mapper: ClaudeRecordMapper::new(
                &header.agent.native_session_id,
                header.agent.id.clone(),
                opts.fallback_timestamp,
            ),
            diagnostics: ParseDiagnostics::default(),
        }
    }
}

impl LogDecoder for ClaudeDecoder {
    fn decode_line(&mut self, line: RawLine<'_>) -> Vec<AgentEvent> {
        let Some(record) = decode_typed::<ClaudeRecord>(&line, &mut self.diagnostics) else {
            return Vec::new();
        };
        match &record {
            ClaudeRecord::Unknown => {
                self.diagnostics.record_unknown(&kind_label_of(line.text));
                return Vec::new();
            }
            ClaudeRecord::FileHistorySnapshot(_) => {
                self.diagnostics.record_ignored("file-history-snapshot");
                return Vec::new();
            }
            _ => {}
        }
        self.mapper.begin_line(line.line, line.byte_offset);
        let mut events = Vec::new();
        self.mapper.map_record(record, &mut events);
        events
    }

    fn diagnostics(&self) -> &ParseDiagnostics {
        &self.diagnostics
    }
}
