//! Lenient line handling shared by all decoders.
//!
//! - [`LineReader`] yields physical lines with byte-accurate offsets.
//! - [`decode_typed`] implements the two-stage decode: a line that is not JSON is
//!   counted as `invalid_json`; a JSON line whose typed deserialize fails is counted
//!   as `schema_mismatch[kind]`. Neither ever fails the file.

use agtrace_types::ParseDiagnostics;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::io::{self, BufRead};

/// A single physical line handed to a decoder.
#[derive(Debug, Clone, Copy)]
pub struct RawLine<'a> {
    /// Line content without the trailing `\n` / `\r\n`.
    pub text: &'a str,
    /// 0-based line number within the file.
    pub line: u64,
    /// Byte offset of the first byte of the line.
    pub byte_offset: u64,
}

impl<'a> RawLine<'a> {
    pub fn new(text: &'a str, line: u64, byte_offset: u64) -> Self {
        Self {
            text,
            line,
            byte_offset,
        }
    }
}

/// An owned line produced by [`LineReader`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedLine {
    pub text: String,
    pub line: u64,
    pub byte_offset: u64,
}

impl OwnedLine {
    pub fn as_raw(&self) -> RawLine<'_> {
        RawLine::new(&self.text, self.line, self.byte_offset)
    }
}

/// Reads newline-terminated lines with byte-accurate offsets.
///
/// `\r` before `\n` is stripped. A trailing line without `\n` is yielded only when
/// `at_eof_complete` is set (batch reads of a finished file); otherwise it is left
/// unconsumed and [`LineReader::offset`] points at its first byte.
pub struct LineReader<R: BufRead> {
    reader: R,
    offset: u64,
    line: u64,
    at_eof_complete: bool,
    buf: Vec<u8>,
    done: bool,
}

impl<R: BufRead> LineReader<R> {
    pub fn new(reader: R, at_eof_complete: bool) -> Self {
        Self::starting_at(reader, 0, 0, at_eof_complete)
    }

    /// Start reading at a known position (the reader must already be positioned there).
    pub fn starting_at(reader: R, offset: u64, line: u64, at_eof_complete: bool) -> Self {
        Self {
            reader,
            offset,
            line,
            at_eof_complete,
            buf: Vec::new(),
            done: false,
        }
    }

    /// First byte not yet consumed.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Next line number.
    pub fn line(&self) -> u64 {
        self.line
    }

    /// Read the next line. `Ok(None)` at EOF (or at an incomplete trailing line).
    pub fn next_line(&mut self) -> io::Result<Option<OwnedLine>> {
        if self.done {
            return Ok(None);
        }
        self.buf.clear();
        let n = self.reader.read_until(b'\n', &mut self.buf)?;
        if n == 0 {
            self.done = true;
            return Ok(None);
        }
        let terminated = self.buf.last() == Some(&b'\n');
        if !terminated && !self.at_eof_complete {
            // Incomplete trailing line: leave it for a later read.
            self.done = true;
            return Ok(None);
        }
        let mut end = self.buf.len();
        if terminated {
            end -= 1;
            if end > 0 && self.buf[end - 1] == b'\r' {
                end -= 1;
            }
        }
        let text = String::from_utf8_lossy(&self.buf[..end]).into_owned();
        let line = OwnedLine {
            text,
            line: self.line,
            byte_offset: self.offset,
        };
        self.offset += n as u64;
        self.line += 1;
        Ok(Some(line))
    }
}

impl<R: BufRead> Iterator for LineReader<R> {
    type Item = io::Result<OwnedLine>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_line().transpose()
    }
}

/// Kind label of a JSON record: `type` plus `payload.type` when present
/// (e.g. `"event_msg/token_count"`), `"<untyped>"` when there is no `type`.
pub fn kind_label(value: &Value) -> String {
    let Some(kind) = value.get("type").and_then(Value::as_str) else {
        return "<untyped>".to_string();
    };
    match value
        .get("payload")
        .and_then(|p| p.get("type"))
        .and_then(Value::as_str)
    {
        Some(sub) => format!("{kind}/{sub}"),
        None => kind.to_string(),
    }
}

/// Kind label of a raw line, parsing it only as far as needed.
pub fn kind_label_of(text: &str) -> String {
    serde_json::from_str::<Value>(text)
        .map(|v| kind_label(&v))
        .unwrap_or_else(|_| "<invalid>".to_string())
}

/// Two-stage lenient decode of one line into `T`.
///
/// Fast path: typed deserialize straight from the text. On failure the line is
/// re-parsed as untyped JSON to classify the problem:
/// not JSON ⇒ `invalid_json`; JSON ⇒ `schema_mismatch[kind]`.
/// Returns `None` (and records the problem) on failure. Empty lines return `None`
/// without recording anything.
pub fn decode_typed<T: DeserializeOwned>(
    line: &RawLine<'_>,
    diagnostics: &mut ParseDiagnostics,
) -> Option<T> {
    diagnostics.lines += 1;
    let text = line.text.trim();
    if text.is_empty() {
        return None;
    }
    match serde_json::from_str::<T>(text) {
        Ok(v) => {
            diagnostics.decoded += 1;
            Some(v)
        }
        Err(typed_err) => {
            match serde_json::from_str::<Value>(text) {
                Err(json_err) => {
                    diagnostics.record_invalid_json(
                        line.line,
                        line.byte_offset,
                        json_err.to_string(),
                    );
                }
                Ok(value) => {
                    diagnostics.record_schema_mismatch(
                        &kind_label(&value),
                        line.line,
                        line.byte_offset,
                        typed_err.to_string(),
                    );
                }
            }
            None
        }
    }
}

/// Flatten a tool output that may be a string or an array of content items.
///
/// Text items are joined with `\n`, images become `[image]`, tool references
/// `[tool_reference <name>]`; other items are skipped.
pub fn flatten_tool_output(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(s) => Some(s.clone()),
                Value::Object(obj) => {
                    let ty = obj.get("type").and_then(Value::as_str).unwrap_or("");
                    if let Some(text) = obj.get("text").and_then(Value::as_str) {
                        Some(text.to_string())
                    } else if ty.contains("image") {
                        Some("[image]".to_string())
                    } else if ty == "tool_reference" {
                        let name = obj
                            .get("tool_name")
                            .or_else(|| obj.get("name"))
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        Some(format!("[tool_reference {name}]"))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Serde helper: deserialize a tool output (string or content array) into a flat string.
pub fn deserialize_flat_output<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let value = Value::deserialize(deserializer)?;
    Ok(flatten_tool_output(&value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    fn read_all(input: &str, at_eof_complete: bool) -> (Vec<OwnedLine>, u64) {
        let mut reader = LineReader::new(input.as_bytes(), at_eof_complete);
        let mut out = Vec::new();
        while let Some(line) = reader.next_line().unwrap() {
            out.push(line);
        }
        (out, reader.offset())
    }

    #[test]
    fn line_reader_offsets_and_crlf() {
        let (lines, offset) = read_all("ab\r\ncde\n\nf\n", false);
        let got: Vec<_> = lines
            .iter()
            .map(|l| (l.text.as_str(), l.line, l.byte_offset))
            .collect();
        assert_eq!(
            got,
            vec![("ab", 0, 0), ("cde", 1, 4), ("", 2, 8), ("f", 3, 9)]
        );
        assert_eq!(offset, 11);
    }

    #[test]
    fn line_reader_leaves_partial_tail() {
        let (lines, offset) = read_all("one\ntw", false);
        assert_eq!(lines.len(), 1);
        assert_eq!(offset, 4, "partial line must stay unconsumed");

        let (lines, offset) = read_all("one\ntw", true);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].text, "tw");
        assert_eq!(offset, 6);
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum Rec {
        Known {
            #[allow(dead_code)]
            value: u32,
        },
        #[serde(other)]
        Unknown,
    }

    #[test]
    fn invalid_json_is_counted_not_fatal() {
        let mut d = ParseDiagnostics::default();
        let r: Option<Rec> = decode_typed(&RawLine::new("{not json", 3, 42), &mut d);
        assert!(r.is_none());
        assert_eq!(d.invalid_json, 1);
        assert_eq!(d.samples[0].line, 3);
        assert_eq!(d.samples[0].byte_offset, 42);
    }

    #[test]
    fn schema_mismatch_is_counted_by_kind() {
        let mut d = ParseDiagnostics::default();
        let r: Option<Rec> = decode_typed(
            &RawLine::new(r#"{"type":"known","value":"str"}"#, 0, 0),
            &mut d,
        );
        assert!(r.is_none());
        assert_eq!(d.schema_mismatch.get("known"), Some(&1));
        assert_eq!(d.invalid_json, 0);
    }

    #[test]
    fn unknown_kind_decodes_to_catch_all() {
        let mut d = ParseDiagnostics::default();
        let r: Option<Rec> = decode_typed(&RawLine::new(r#"{"type":"mystery"}"#, 0, 0), &mut d);
        assert!(matches!(r, Some(Rec::Unknown)));
        assert_eq!(d.decoded, 1);
        assert!(!d.has_errors());
    }

    #[test]
    fn empty_line_is_skipped_silently() {
        let mut d = ParseDiagnostics::default();
        let r: Option<Rec> = decode_typed(&RawLine::new("   ", 0, 0), &mut d);
        assert!(r.is_none());
        assert!(!d.has_errors());
        assert_eq!(d.lines, 1);
    }

    #[test]
    fn kind_label_includes_payload_type() {
        let v: Value = serde_json::json!({"type":"event_msg","payload":{"type":"token_count"}});
        assert_eq!(kind_label(&v), "event_msg/token_count");
        assert_eq!(kind_label(&serde_json::json!({"type":"user"})), "user");
        assert_eq!(kind_label(&serde_json::json!([1])), "<untyped>");
    }

    #[test]
    fn flatten_array_output() {
        let v = serde_json::json!([
            {"type":"input_text","text":"line one"},
            {"type":"input_image","image_url":"data:..."},
            {"type":"tool_reference","tool_name":"Read"},
            {"type":"text","text":"line two"}
        ]);
        assert_eq!(
            flatten_tool_output(&v),
            "line one\n[image]\n[tool_reference Read]\nline two"
        );
        assert_eq!(flatten_tool_output(&serde_json::json!("plain")), "plain");
    }
}
