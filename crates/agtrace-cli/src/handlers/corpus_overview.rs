use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::{build_spans, Span};
use agtrace_index::Database;
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;

pub fn handle(db: &Database, project_hash: Option<String>, all_projects: bool) -> Result<()> {
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    // Load sessions (limit to ~50 for overview)
    let sessions = db.list_sessions(effective_project_hash, 50)?;

    if sessions.is_empty() {
        println!("No sessions found.");
        println!("\nRun `agtrace index update` to scan for sessions.");
        return Ok(());
    }

    println!("# Corpus Overview\n");
    println!("Total sessions: {}\n", sessions.len());

    let mut digests = Vec::new();
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();

    // Build digests for each session
    for session in &sessions {
        if let Ok(events) = loader.load_events(&session.id, &options) {
            let spans = build_spans(&events);
            if !spans.is_empty() {
                let digest = CorpusDigest::new(&session.id, &session.provider, spans);
                digests.push(digest);
            }
        }
    }

    // Group by lens
    let lenses = group_by_lens(&digests);

    // Display each lens with count and 1 example
    display_lens("Failures", &lenses.failures, &digests);
    display_lens("Bottlenecks", &lenses.bottlenecks, &digests);
    display_lens("Toolchains", &lenses.toolchains, &digests);
    display_lens("Loops", &lenses.loops, &digests);

    println!("\nRun `agtrace pack` to generate LLM-ready context from important sessions.");

    Ok(())
}

#[derive(Debug)]
struct CorpusDigest {
    session_id: String,
    provider: String,
    spans: Vec<Span>,
    opening: Option<String>,
}

impl CorpusDigest {
    fn new(session_id: &str, provider: &str, spans: Vec<Span>) -> Self {
        let opening = spans
            .first()
            .and_then(|s| s.user.as_ref().map(|u| truncate_string(&u.text, 100)));

        Self {
            session_id: session_id.to_string(),
            provider: provider.to_string(),
            spans,
            opening,
        }
    }
}

#[derive(Default)]
struct LensGroups {
    failures: Vec<usize>,
    bottlenecks: Vec<usize>,
    toolchains: Vec<usize>,
    loops: Vec<usize>,
}

fn group_by_lens(digests: &[CorpusDigest]) -> LensGroups {
    let mut groups = LensGroups::default();

    for (i, digest) in digests.iter().enumerate() {
        // Failures: sessions with tool failures
        if digest.spans.iter().any(|s| s.stats.tool_failures > 0) {
            groups.failures.push(i);
        }

        // Bottlenecks: sessions with e2e > 30s
        if digest
            .spans
            .iter()
            .any(|s| s.stats.e2e_ms.unwrap_or(0) > 30_000)
        {
            groups.bottlenecks.push(i);
        }

        // Toolchains: sessions with many tool calls
        let total_tools: usize = digest.spans.iter().map(|s| s.stats.tool_calls).sum();
        if total_tools > 10 {
            groups.toolchains.push(i);
        }

        // Loops: sessions with repeated similar tool calls (simple heuristic)
        if has_potential_loops(&digest.spans) {
            groups.loops.push(i);
        }
    }

    groups
}

fn has_potential_loops(spans: &[Span]) -> bool {
    // Simple heuristic: if any span has > 5 tool calls, might be a loop
    spans.iter().any(|s| s.stats.tool_calls > 5)
}

fn display_lens(name: &str, indices: &[usize], digests: &[CorpusDigest]) {
    println!("## {}", name);
    println!("Count: {}", indices.len());

    if let Some(&idx) = indices.first() {
        if let Some(digest) = digests.get(idx) {
            let id_short = &digest.session_id[..8.min(digest.session_id.len())];
            println!("Example: Session {} ({})", id_short, digest.provider);
            if let Some(opening) = &digest.opening {
                println!("  {}", opening);
            }
        }
    }

    println!();
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let chars: Vec<char> = s.chars().take(max_len).collect();
        chars.iter().collect::<String>() + "..."
    }
}
