//! Per-file decode diagnostics.
//!
//! Decoders are lenient per line: a line that is not JSON, has an unknown kind,
//! or fails a typed deserialize is *counted* here and skipped. A file never
//! fails to decode because of its content.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Maximum number of [`LineError`] samples kept per diagnostics instance.
pub const MAX_DIAGNOSTIC_SAMPLES: usize = 20;

/// A single problematic line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineError {
    pub line: u64,
    pub byte_offset: u64,
    /// Record kind label (e.g. "assistant", "event_msg/token_count"), if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub message: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseDiagnostics {
    /// Lines handed to the decoder.
    pub lines: u64,
    /// Lines decoded into a typed record.
    pub decoded: u64,
    /// Lines that are not valid JSON.
    pub invalid_json: u64,
    /// kind -> count (typed deserialize failed)
    pub schema_mismatch: BTreeMap<String, u64>,
    /// kind -> count (not modeled)
    pub unknown_kinds: BTreeMap<String, u64>,
    /// kind -> count (modeled as intentionally ignored)
    pub ignored_kinds: BTreeMap<String, u64>,
    /// First [`MAX_DIAGNOSTIC_SAMPLES`] errors.
    pub samples: Vec<LineError>,
}

impl ParseDiagnostics {
    fn push_sample(&mut self, sample: LineError) {
        if self.samples.len() < MAX_DIAGNOSTIC_SAMPLES {
            self.samples.push(sample);
        }
    }

    pub fn record_invalid_json(&mut self, line: u64, byte_offset: u64, message: impl Into<String>) {
        self.invalid_json += 1;
        self.push_sample(LineError {
            line,
            byte_offset,
            kind: None,
            message: message.into(),
        });
    }

    pub fn record_schema_mismatch(
        &mut self,
        kind: &str,
        line: u64,
        byte_offset: u64,
        message: impl Into<String>,
    ) {
        *self.schema_mismatch.entry(kind.to_string()).or_default() += 1;
        self.push_sample(LineError {
            line,
            byte_offset,
            kind: Some(kind.to_string()),
            message: message.into(),
        });
    }

    pub fn record_unknown(&mut self, kind: &str) {
        *self.unknown_kinds.entry(kind.to_string()).or_default() += 1;
    }

    pub fn record_ignored(&mut self, kind: &str) {
        *self.ignored_kinds.entry(kind.to_string()).or_default() += 1;
    }

    /// Total number of typed-deserialize failures.
    pub fn schema_mismatch_total(&self) -> u64 {
        self.schema_mismatch.values().sum()
    }

    /// Lines that could not be decoded at all (invalid JSON or schema mismatch).
    pub fn error_count(&self) -> u64 {
        self.invalid_json + self.schema_mismatch_total()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    /// Accumulate another diagnostics instance into this one.
    pub fn merge(&mut self, other: &ParseDiagnostics) {
        self.lines += other.lines;
        self.decoded += other.decoded;
        self.invalid_json += other.invalid_json;
        for (map, other_map) in [
            (&mut self.schema_mismatch, &other.schema_mismatch),
            (&mut self.unknown_kinds, &other.unknown_kinds),
            (&mut self.ignored_kinds, &other.ignored_kinds),
        ] {
            for (k, v) in other_map {
                *map.entry(k.clone()).or_default() += v;
            }
        }
        for s in &other.samples {
            self.push_sample(s.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_samples() {
        let mut d = ParseDiagnostics::default();
        d.record_invalid_json(0, 0, "bad");
        d.record_schema_mismatch("assistant", 1, 10, "missing field");
        d.record_schema_mismatch("assistant", 2, 20, "missing field");
        d.record_unknown("mystery");
        d.record_ignored("mode");
        assert_eq!(d.error_count(), 3);
        assert_eq!(d.schema_mismatch["assistant"], 2);
        assert_eq!(d.samples.len(), 3);

        let mut total = ParseDiagnostics::default();
        total.merge(&d);
        total.merge(&d);
        assert_eq!(total.schema_mismatch["assistant"], 4);
        assert_eq!(total.unknown_kinds["mystery"], 2);
        assert_eq!(total.invalid_json, 2);
    }

    #[test]
    fn samples_are_capped() {
        let mut d = ParseDiagnostics::default();
        for i in 0..50 {
            d.record_invalid_json(i, i, "bad");
        }
        assert_eq!(d.invalid_json, 50);
        assert_eq!(d.samples.len(), MAX_DIAGNOSTIC_SAMPLES);
    }
}
