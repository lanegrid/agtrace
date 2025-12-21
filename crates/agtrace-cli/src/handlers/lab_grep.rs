use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use agtrace_runtime::{AgTrace, SessionFilter};
use anyhow::Result;

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
//   2. Debugging: Inspect how ToolCallPayload::from_raw() parses different providers (Claude, Codex, Gemini)
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
//   - Navigate ~/.claude/sessions/*.jsonl or ~/.codex/sessions/*.jsonl directly
//   - Manually correlate timestamps and event IDs
//   - Parse provider-specific log formats
//
// With --raw, verification is streamlined:
//   - Single command to inspect both normalized and raw data
//   - Session/stream context preserved
//   - No filesystem traversal required

pub fn handle(
    workspace: &AgTrace,
    pattern: String,
    limit: Option<usize>,
    source: Option<String>,
    json_output: bool,
    raw_output: bool,
    view: &dyn TraceView,
) -> Result<()> {
    let mut filter = SessionFilter::new().limit(1000);
    if let Some(src) = source {
        filter = filter.source(src);
    }

    let sessions = workspace.sessions().list(filter)?;
    let max_matches = limit.unwrap_or(50);

    if raw_output {
        // Raw mode: output complete AgentEvent with metadata
        let mut count = 0;

        'outer: for session_summary in sessions {
            let session = workspace.sessions().find(&session_summary.id)?;
            let events = session.events()?;

            for event in &events {
                let payload_str = serde_json::to_string(&event.payload)?;

                if payload_str.contains(&pattern) {
                    if count == 0 {
                        println!("Searching for pattern '\x1b[36m{}\x1b[39m'...", pattern);
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

        Ok(())
    } else {
        // Normal mode: use presenters
        let mut matches = Vec::new();

        'outer: for session_summary in sessions {
            let session = workspace.sessions().find(&session_summary.id)?;
            let events = session.events()?;

            for event in events {
                let payload_str = serde_json::to_string(&event.payload)?;

                if payload_str.contains(&pattern) {
                    let vm = presenters::present_event(&event);
                    matches.push(vm);

                    if matches.len() >= max_matches {
                        break 'outer;
                    }
                }
            }
        }

        view.render_lab_grep(&matches, &pattern, json_output)?;
        Ok(())
    }
}
