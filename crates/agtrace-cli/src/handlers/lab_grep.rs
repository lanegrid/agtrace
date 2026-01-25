use crate::args::{OutputFormat, ViewModeArgs};
use crate::presentation::presenters;
use crate::presentation::view_models::CommandResultViewModel;
use crate::presentation::{ConsoleRenderer, Renderer};
use agtrace_sdk::Client;
use agtrace_sdk::types::{AgentEvent, EventPayload, SessionFilter};
use anyhow::{Context, Result};
use regex::Regex;

// Rationale: lab grep with --raw option
//
// Purpose:
//   Verify normalization correctness by comparing raw provider schemas with normalized AgentEvent content.
//
// Design:
//   - Normal mode (--json): Shows normalized EventPayload via presenters (user-friendly, type-safe)
//   - Raw mode (--raw): Shows complete AgentEvent including metadata (debugging, verification)
//
// Why metadata is essential:
//   1. Validation: Compare provider-specific schemas (metadata.message, metadata.payload) with normalized content
//   2. Debugging: Inspect how normalize_tool_call() parses different providers (Claude, Codex, Gemini)
//   3. Investigation: Access original tool inputs when normalized arguments differ (e.g., Codex stringified JSON)
//
// Example workflow:
//   $ agtrace lab grep '"name":"Read"' --raw --limit 1
//   # Inspect content.arguments (normalized FileReadArgs) vs metadata.message.content[].input (raw Claude schema)
//
//   $ agtrace lab grep '"name":"mcp__o3__o3-search"' --raw --limit 1
//   # Verify Mcp variant parsing and McpArgs::parse_name() behavior
//
// Without --raw, users would need to:
//   - Navigate ~/.claude/projects/*.jsonl or ~/.codex/sessions/*.jsonl directly
//   - Manually correlate timestamps and event IDs
//   - Parse provider-specific log formats
//
// With --raw, verification is streamlined:
//   - Single command to inspect both normalized and raw data
//   - Session/stream context preserved
//   - No filesystem traversal required

/// Configuration for grep operation
pub struct GrepOptions {
    pub pattern: String,
    pub limit: Option<usize>,
    pub provider: Option<String>,
    pub json_output: bool,
    pub raw_output: bool,
    pub use_regex: bool,
    pub ignore_case: bool,
    pub event_type: Option<String>,
    pub tool_name: Option<String>,
}

/// Matcher for searching events with various filters
struct EventMatcher {
    pattern_regex: Option<Regex>,
    pattern_glob: Option<String>,
    pattern_str: String,
    ignore_case: bool,
    event_type_filter: Option<String>,
    tool_name_filter: Option<String>,
}

/// Check if pattern contains glob wildcards (* or ?)
fn is_glob_pattern(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?')
}

impl EventMatcher {
    fn new(
        pattern: String,
        use_regex: bool,
        ignore_case: bool,
        event_type: Option<String>,
        tool_name: Option<String>,
    ) -> Result<Self> {
        // Priority: --regex flag > auto-detected glob > substring
        let (pattern_regex, pattern_glob) = if use_regex {
            let regex_str = if ignore_case {
                format!("(?i){}", pattern)
            } else {
                pattern.clone()
            };
            (
                Some(Regex::new(&regex_str).context("Invalid regex pattern")?),
                None,
            )
        } else if is_glob_pattern(&pattern) {
            let glob_pattern = if ignore_case {
                pattern.to_lowercase()
            } else {
                pattern.clone()
            };
            (None, Some(glob_pattern))
        } else {
            (None, None)
        };

        let pattern_str = if ignore_case && !use_regex && !is_glob_pattern(&pattern) {
            pattern.to_lowercase()
        } else {
            pattern
        };

        Ok(Self {
            pattern_regex,
            pattern_glob,
            pattern_str,
            ignore_case,
            event_type_filter: event_type,
            tool_name_filter: tool_name,
        })
    }

    /// Check if event matches all filters
    fn matches(&self, event: &AgentEvent) -> Result<bool> {
        // Event type filter (early return for performance)
        if let Some(ref event_type) = self.event_type_filter
            && !self.matches_event_type(event, event_type)
        {
            return Ok(false);
        }

        // Tool name filter (for ToolCall events)
        if let Some(ref tool_name) = self.tool_name_filter
            && !self.matches_tool_name(event, tool_name)
        {
            return Ok(false);
        }

        // Pattern matching on payload
        // Priority: regex > glob > substring
        let payload_str = serde_json::to_string(&event.payload)?;

        if let Some(ref regex) = self.pattern_regex {
            Ok(regex.is_match(&payload_str))
        } else if let Some(ref glob_pattern) = self.pattern_glob {
            let target = if self.ignore_case {
                payload_str.to_lowercase()
            } else {
                payload_str
            };
            Ok(glob_match::glob_match(glob_pattern, &target))
        } else if self.ignore_case {
            Ok(payload_str.to_lowercase().contains(&self.pattern_str))
        } else {
            Ok(payload_str.contains(&self.pattern_str))
        }
    }

    fn matches_event_type(&self, event: &AgentEvent, event_type: &str) -> bool {
        let actual_type = match &event.payload {
            EventPayload::ToolCall(_) => "ToolCall",
            EventPayload::ToolResult(_) => "ToolResult",
            EventPayload::User(_) => "User",
            EventPayload::Message(_) => "Message",
            EventPayload::Reasoning(_) => "Reasoning",
            EventPayload::TokenUsage(_) => "TokenUsage",
            EventPayload::Notification(_) => "Notification",
            EventPayload::SlashCommand(_) => "SlashCommand",
            EventPayload::QueueOperation(_) => "QueueOperation",
            EventPayload::Summary(_) => "Summary",
        };
        actual_type.eq_ignore_ascii_case(event_type)
    }

    fn matches_tool_name(&self, event: &AgentEvent, tool_name: &str) -> bool {
        match &event.payload {
            EventPayload::ToolCall(tool_call_payload) => {
                let actual_name = tool_call_payload.name();
                if self.ignore_case {
                    actual_name.eq_ignore_ascii_case(tool_name)
                } else {
                    actual_name == tool_name
                }
            }
            _ => false,
        }
    }
}

pub fn handle(
    client: &Client,
    options: GrepOptions,
    output_format: OutputFormat,
    view_mode_args: &ViewModeArgs,
) -> Result<()> {
    // Build matcher with all filters
    let matcher = EventMatcher::new(
        options.pattern.clone(),
        options.use_regex,
        options.ignore_case,
        options.event_type,
        options.tool_name,
    )?;

    let mut filter = SessionFilter::all().limit(1000);
    if let Some(src) = options.provider {
        filter = filter.provider(src);
    }

    let sessions = client.sessions().list(filter)?;
    let max_matches = options.limit.unwrap_or(10);

    if options.raw_output {
        // Raw mode: output complete AgentEvent with metadata
        let mut count = 0;

        'outer: for session_summary in sessions {
            let session = client.sessions().get(&session_summary.id)?;
            let events = session.events()?;

            for event in &events {
                if matcher.matches(event)? {
                    if count == 0 {
                        println!(
                            "Searching for pattern '\x1b[36m{}\x1b[39m'...",
                            options.pattern
                        );
                        println!("Found matches:\n");
                    }

                    count += 1;
                    println!("\x1b[90m{}\x1b[39m", "=".repeat(80));
                    println!(
                        "Match #{} | Session: \x1b[33m{}\x1b[39m | Stream: {:?}",
                        count,
                        &session_summary.id.to_string()[..8],
                        event.stream_id
                    );

                    let json = serde_json::to_string_pretty(&event)?;
                    println!("{}", json);
                    println!("\x1b[90m{}\x1b[39m", "=".repeat(80));

                    if count >= max_matches {
                        break 'outer;
                    }
                }
            }
        }

        if count == 0 {
            println!(
                "Searching for pattern '\x1b[36m{}\x1b[39m'...",
                options.pattern
            );
            println!("Found 0 matches:");
        }

        Ok(())
    } else {
        // Normal mode: use presenters
        let mut matches = Vec::new();

        'outer: for session_summary in sessions {
            let session = client.sessions().get(&session_summary.id)?;
            let events = session.events()?;

            for event in events {
                if matcher.matches(&event)? {
                    let vm = presenters::present_event(&event);
                    matches.push(vm);

                    if matches.len() >= max_matches {
                        break 'outer;
                    }
                }
            }
        }

        let vm =
            presenters::present_lab_grep(options.pattern.clone(), matches, options.json_output);
        let result_vm = CommandResultViewModel::new(vm);
        let resolved_view_mode = view_mode_args.resolve();
        let renderer = ConsoleRenderer::new(output_format.into(), resolved_view_mode);
        renderer.render(result_vm)?;
        Ok(())
    }
}
