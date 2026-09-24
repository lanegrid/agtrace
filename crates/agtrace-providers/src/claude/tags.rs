//! Tolerant scanner for the XML-ish tags Claude Code embeds in user text
//! (`<teammate-message>`, `<task-notification>`, `<agent-message>`, `<command-name>`,
//! `<local-command-stdout>`). Not an XML parser: attributes are `key="value"`
//! pairs, a missing closing tag means "until the end of the text", nesting of the
//! same tag is not supported (it never occurs).

/// One occurrence of `<name attrs>inner</name>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tag<'a> {
    attrs: &'a str,
    pub inner: &'a str,
}

impl<'a> Tag<'a> {
    /// Attribute value (entity-decoded), if present.
    pub fn attr(&self, key: &str) -> Option<String> {
        parse_attrs(self.attrs)
            .into_iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    }
}

/// Byte index of the `>` closing an opening tag whose attributes start at `from`
/// (quotes are respected, so `summary="a > b"` is fine).
fn open_tag_end(text: &str, from: usize) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (i, b) in text.as_bytes()[from..].iter().enumerate() {
        match (quote, *b) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, b'"') | (None, b'\'') => quote = Some(*b),
            (None, b'>') => return Some(from + i),
            (None, _) => {}
        }
    }
    None
}

/// All occurrences of `<name ...>...</name>` in document order.
pub(crate) fn find_tags<'a>(text: &'a str, name: &str) -> Vec<Tag<'a>> {
    let open = format!("<{name}");
    let close = format!("</{name}>");
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(rel) = text[pos..].find(&open) {
        let start = pos + rel;
        let after_name = start + open.len();
        // Must be followed by whitespace, '>' or '/' (not a longer tag name).
        match text.as_bytes().get(after_name) {
            Some(b'>') | Some(b' ') | Some(b'\n') | Some(b'\t') | Some(b'\r') | Some(b'/') => {}
            _ => {
                pos = after_name;
                continue;
            }
        }
        let Some(gt) = open_tag_end(text, after_name) else {
            break;
        };
        let attrs = text[after_name..gt].trim_end_matches('/');
        let body_start = gt + 1;
        let (inner, next) = match text[body_start..].find(&close) {
            Some(r) => (
                &text[body_start..body_start + r],
                body_start + r + close.len(),
            ),
            None => (&text[body_start..], text.len()),
        };
        out.push(Tag { attrs, inner });
        pos = next;
    }
    out
}

/// First occurrence of `<name>` (convenience for single-valued child tags).
pub(crate) fn find_tag<'a>(text: &'a str, name: &str) -> Option<Tag<'a>> {
    find_tags(text, name).into_iter().next()
}

/// Trimmed inner text of the first `<name>` child, if non-empty.
pub(crate) fn child_text(text: &str, name: &str) -> Option<String> {
    find_tag(text, name)
        .map(|t| t.inner.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_attrs(attrs: &str) -> Vec<(&str, String)> {
    let mut out = Vec::new();
    let mut rest = attrs;
    loop {
        rest = rest.trim_start();
        let Some(eq) = rest.find('=') else { break };
        let key = rest[..eq].trim();
        let after = rest[eq + 1..].trim_start();
        let Some(q) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            // Unquoted value: up to the next whitespace.
            let end = after.find(char::is_whitespace).unwrap_or(after.len());
            out.push((key, decode_entities(&after[..end])));
            rest = &after[end..];
            continue;
        };
        let body = &after[1..];
        let end = body.find(q).unwrap_or(body.len());
        out.push((key, decode_entities(&body[..end])));
        rest = body.get(end + 1..).unwrap_or("");
    }
    out
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Does `text` (ignoring leading whitespace) start with the tag `<name`?
pub(crate) fn starts_with_tag(text: &str, name: &str) -> bool {
    let t = text.trim_start();
    t.strip_prefix('<')
        .and_then(|r| r.strip_prefix(name))
        .is_some_and(|r| r.starts_with([' ', '>', '\n', '\t', '\r', '/']))
}

/// Strip a one-line delivery preamble such as "Another Claude session sent a message:"
/// that precedes agent tags. Only a short first line ending with ':' is removed.
pub(crate) fn strip_delivery_preamble(text: &str) -> &str {
    let t = text.trim_start();
    match t.split_once('\n') {
        Some((first, rest))
            if first.trim_end().ends_with(':') && first.len() <= 120 && !first.contains('<') =>
        {
            rest.trim_start()
        }
        _ => t,
    }
}

/// Slash command from `<command-name>/x</command-name>[<command-args>..]`.
/// Only names starting with `/` count (documentation text mentioning the tag does not).
pub(crate) fn slash_command(text: &str) -> Option<(String, Option<String>)> {
    let name = child_text(text, "command-name")?;
    if !name.starts_with('/') {
        return None;
    }
    let args = child_text(text, "command-args");
    Some((name, args))
}

/// Model from `/model` output: "Set model to `Opus 5.5 (1M context) (default)`".
/// Returns the display name with the `(default)` marker removed.
pub(crate) fn set_model_output(text: &str) -> Option<String> {
    let stdout = find_tag(text, "local-command-stdout")?.inner;
    let rest = stdout.split("Set model to").nth(1)?;
    let name = match rest.find('`') {
        Some(open) => {
            let body = &rest[open + 1..];
            &body[..body.find('`').unwrap_or(body.len())]
        }
        None => rest.lines().next().unwrap_or(""),
    };
    let name = strip_ansi(name);
    let name = name.trim().trim_end_matches("(default)").trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // ESC [ ... final byte in @..~
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// `<task-notification>` fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TaskNotification {
    pub task_id: Option<String>,
    pub tool_use_id: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub result: Option<String>,
    pub task_type: Option<String>,
    pub total_tokens: Option<u64>,
    pub tool_uses: Option<u64>,
    pub duration_ms: Option<u64>,
}

impl TaskNotification {
    pub fn parse(inner: &str) -> Self {
        let usage = find_tag(inner, "usage").map(|t| t.inner).unwrap_or("");
        let num = |name: &str| child_text(usage, name).and_then(|v| v.parse().ok());
        Self {
            task_id: child_text(inner, "task-id"),
            tool_use_id: child_text(inner, "tool-use-id"),
            status: child_text(inner, "status"),
            summary: child_text(inner, "summary"),
            result: child_text(inner, "result"),
            task_type: child_text(inner, "task-type"),
            total_tokens: num("subagent_tokens"),
            tool_uses: num("tool_uses"),
            duration_ms: num("duration_ms"),
        }
    }

    /// Agent tasks have ids of the form `a` + 16 hex digits (bash / monitor tasks: `b…`).
    pub fn is_agent_task(&self) -> bool {
        self.task_id.as_deref().is_some_and(is_subagent_id)
    }
}

/// Claude subagent id: `a` followed by 16 hex digits.
pub(crate) fn is_subagent_id(s: &str) -> bool {
    s.len() == 17 && s.starts_with('a') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// Truncate to at most `max` bytes on a char boundary, appending `…` when cut.
pub(crate) fn truncate_bytes(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_teammate_messages_with_attributes() {
        let text = "<teammate-message teammate_id=\"audit-A\" color=\"blue\" summary=\"a > b &amp; c\">\nhello\n</teammate-message>\n\n<teammate-message teammate_id=\"audit-B\">{\"type\":\"idle_notification\"}</teammate-message>";
        let tags = find_tags(text, "teammate-message");
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].attr("teammate_id").as_deref(), Some("audit-A"));
        assert_eq!(tags[0].attr("summary").as_deref(), Some("a > b & c"));
        assert_eq!(tags[0].inner.trim(), "hello");
        assert_eq!(tags[1].attr("teammate_id").as_deref(), Some("audit-B"));
        assert_eq!(tags[1].attr("color"), None);
        assert_eq!(tags[1].inner, "{\"type\":\"idle_notification\"}");
    }

    #[test]
    fn unclosed_tag_runs_to_end_and_longer_names_are_skipped() {
        let tags = find_tags(
            "<agent-messages>x</agent-messages><agent-message from=\"a1\">body",
            "agent-message",
        );
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].attr("from").as_deref(), Some("a1"));
        assert_eq!(tags[0].inner, "body");
    }

    #[test]
    fn task_notification_fields() {
        let n = TaskNotification::parse(
            "\n<task-id>a0123456789abcdef</task-id>\n<status>completed</status>\n<summary>Agent \"x\" finished</summary>\n<result>done</result>\n<usage><subagent_tokens>114364</subagent_tokens><tool_uses>50</tool_uses><duration_ms>356473</duration_ms></usage>",
        );
        assert!(n.is_agent_task());
        assert_eq!(n.status.as_deref(), Some("completed"));
        assert_eq!(n.result.as_deref(), Some("done"));
        assert_eq!(n.total_tokens, Some(114_364));
        assert_eq!(n.tool_uses, Some(50));
        assert_eq!(n.duration_ms, Some(356_473));
        assert!(!TaskNotification::parse("<task-id>b8deakek1</task-id>").is_agent_task());
    }

    #[test]
    fn slash_command_requires_leading_slash() {
        assert_eq!(
            slash_command(
                "<command-name>/exit</command-name>\n<command-args>--force</command-args>"
            ),
            Some(("/exit".to_string(), Some("--force".to_string())))
        );
        assert_eq!(
            slash_command("<command-name>/model</command-name><command-args></command-args>"),
            Some(("/model".to_string(), None))
        );
        assert_eq!(slash_command("mentions `<command-name>` tags"), None);
        assert_eq!(slash_command("<command-name>commit</command-name>"), None);
        assert_eq!(slash_command("<command-name></command-name>"), None);
    }

    #[test]
    fn set_model_output_variants() {
        assert_eq!(
            set_model_output("<local-command-stdout>Set model to `Opus 5.5 (1M context) (default)`</local-command-stdout>").as_deref(),
            Some("Opus 5.5 (1M context)")
        );
        assert_eq!(
            set_model_output("<local-command-stdout>Set model to \u{1b}[1mFable 5.1\u{1b}[22m</local-command-stdout>").as_deref(),
            Some("Fable 5.1")
        );
        assert_eq!(
            set_model_output(
                "<local-command-stdout>Kept model as `Fable 5.1`</local-command-stdout>"
            ),
            None
        );
    }

    #[test]
    fn delivery_preamble_is_stripped() {
        let t = "Another Claude session sent a message:\n<teammate-message teammate_id=\"x\">hi</teammate-message>";
        assert!(starts_with_tag(
            strip_delivery_preamble(t),
            "teammate-message"
        ));
        assert_eq!(
            strip_delivery_preamble("plain text\nmore"),
            "plain text\nmore"
        );
        assert_eq!(
            strip_delivery_preamble("note: see <x>:\nrest"),
            "note: see <x>:\nrest"
        );
    }

    #[test]
    fn starts_with_tag_checks_name_boundary() {
        assert!(starts_with_tag(
            "  <task-notification>\n",
            "task-notification"
        ));
        assert!(!starts_with_tag(
            "<task-notifications>",
            "task-notification"
        ));
        assert!(!starts_with_tag(
            "see <task-notification>",
            "task-notification"
        ));
    }

    #[test]
    fn truncate_bytes_respects_char_boundaries() {
        assert_eq!(truncate_bytes("abc", 3), "abc");
        assert_eq!(truncate_bytes("abcd", 3), "abc…");
        assert_eq!(truncate_bytes("日本語", 4), "日…");
    }

    #[test]
    fn subagent_ids() {
        assert!(is_subagent_id("a7ac2e1017cbaefa5"));
        assert!(!is_subagent_id("b8deakek1"));
        assert!(!is_subagent_id("a7ac2e1017cbaefz5"));
    }
}
