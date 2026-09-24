//! Line-oriented Codex rollout decoder (`LogDecoder`).

use agtrace_types::{AgentEvent, ParseDiagnostics};

use super::parser::CodexRecordMapper;
use super::schema::{CodexRecord, EventMsgPayload, ResponseItemPayload};
use crate::lenient::{RawLine, decode_typed, kind_label_of};
use crate::provider::{DecodeOptions, FileHeader, LogDecoder};

/// Record kinds that are intentionally not modeled (counted as ignored, not unknown).
const IGNORED_KINDS: &[&str] = &["response_item/ghost_snapshot"];

pub struct CodexDecoder {
    mapper: CodexRecordMapper,
    diagnostics: ParseDiagnostics,
}

impl CodexDecoder {
    pub fn new(header: &FileHeader, opts: DecodeOptions) -> Self {
        Self {
            mapper: CodexRecordMapper::new(
                &header.agent.native_session_id,
                header.agent.id.clone(),
                opts.fallback_timestamp,
            ),
            diagnostics: ParseDiagnostics::default(),
        }
    }
}

impl LogDecoder for CodexDecoder {
    fn decode_line(&mut self, line: RawLine<'_>) -> Vec<AgentEvent> {
        let Some(record) = decode_typed::<CodexRecord>(&line, &mut self.diagnostics) else {
            return Vec::new();
        };
        let unknown = match &record {
            CodexRecord::Unknown => true,
            CodexRecord::ResponseItem(r) => matches!(r.payload, ResponseItemPayload::Unknown),
            CodexRecord::EventMsg(e) => matches!(e.payload, EventMsgPayload::Unknown),
            _ => false,
        };
        if unknown {
            let kind = kind_label_of(line.text);
            if IGNORED_KINDS.contains(&kind.as_str()) {
                self.diagnostics.record_ignored(&kind);
            } else {
                self.diagnostics.record_unknown(&kind);
            }
            return Vec::new();
        }
        let mut events = Vec::new();
        self.mapper
            .map_record(&record, line.line, line.byte_offset, &mut events);
        events
    }

    fn diagnostics(&self) -> &ParseDiagnostics {
        &self.diagnostics
    }
}
