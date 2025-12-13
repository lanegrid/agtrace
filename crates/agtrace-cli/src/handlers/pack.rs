use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::{build_spans, Span};
use agtrace_index::Database;
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;

pub fn handle(
    db: &Database,
    template: &str,
    limit: usize,
    project_hash: Option<String>,
    all_projects: bool,
) -> Result<()> {
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    // Load sessions (use higher limit for scoring, then filter)
    let sessions = db.list_sessions(effective_project_hash, limit * 3)?;

    let mut digests = Vec::new();
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();

    // Build digests for each session
    for session in &sessions {
        if let Ok(events) = loader.load_events(&session.id, &options) {
            let spans = build_spans(&events);
            if !spans.is_empty() {
                let digest = SessionDigest::new(&session.id, &session.provider, spans);
                digests.push(digest);
            }
        }
    }

    // Score and sort by importance
    digests.sort_by(|a, b| b.importance_score.cmp(&a.importance_score));

    // Take top N
    digests.truncate(limit);

    // Output based on template
    match template {
        "compact" => output_compact(&digests),
        "diagnose" => output_diagnose(&digests),
        "tools" => output_tools(&digests),
        _ => output_compact(&digests),
    }

    Ok(())
}

#[derive(Debug)]
struct SessionDigest {
    session_id: String,
    source: String,
    spans: Vec<Span>,
    opening: Option<String>,
    activation: Option<String>,
    outcome: String,
    importance_score: u32,
}

impl SessionDigest {
    fn new(session_id: &str, provider: &str, spans: Vec<Span>) -> Self {
        let opening = spans.first().and_then(|s| {
            s.user
                .as_ref()
                .map(|u| truncate_string(&u.text, 100).to_string())
        });

        let activation = find_activation(&spans);
        let outcome = compute_outcome(&spans);
        let importance_score = compute_importance_score(&spans);

        Self {
            session_id: session_id.to_string(),
            source: provider.to_string(),
            spans,
            opening,
            activation,
            outcome,
            importance_score,
        }
    }
}

fn find_activation(spans: &[Span]) -> Option<String> {
    for i in 0..spans.len() {
        let end = (i + 5).min(spans.len());
        let tool_count: usize = spans[i..end].iter().map(|s| s.stats.tool_calls).sum();

        if tool_count >= 3 {
            return spans[i]
                .user
                .as_ref()
                .map(|u| truncate_string(&u.text, 100).to_string());
        }
    }

    spans.first().and_then(|s| {
        s.user
            .as_ref()
            .map(|u| truncate_string(&u.text, 100).to_string())
    })
}

fn compute_outcome(spans: &[Span]) -> String {
    let total_failures: usize = spans.iter().map(|s| s.stats.tool_failures).sum();

    if total_failures > 0 {
        format!("completed with {} failures", total_failures)
    } else {
        "completed".to_string()
    }
}

fn compute_importance_score(spans: &[Span]) -> u32 {
    let mut score = 0u32;

    let total_failures: usize = spans.iter().map(|s| s.stats.tool_failures).sum();
    if total_failures > 0 {
        score += 5;
    }

    let max_e2e_ms = spans
        .iter()
        .filter_map(|s| s.stats.e2e_ms)
        .max()
        .unwrap_or(0);
    if max_e2e_ms > 30_000 {
        score += 3;
    }

    let max_tool_ms = spans
        .iter()
        .filter_map(|s| s.stats.tool_ms)
        .max()
        .unwrap_or(0);
    if max_tool_ms > 20_000 {
        score += 3;
    }

    let total_tool_calls: usize = spans.iter().map(|s| s.stats.tool_calls).sum();
    if total_tool_calls > 30 {
        score += 2;
    }

    score
}

fn output_compact(digests: &[SessionDigest]) {
    for digest in digests {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        println!("## Session {} ({})", id_short, digest.source);

        if let Some(opening) = &digest.opening {
            println!("Opening: {}", opening);
        }
        if let Some(activation) = &digest.activation {
            println!("Activation: {}", activation);
        }

        println!("Work:");
        for span in &digest.spans {
            print_span_compact(span);
        }

        println!("Outcome: {}", digest.outcome);

        let total_failures: usize = digest.spans.iter().map(|s| s.stats.tool_failures).sum();
        let max_e2e = digest
            .spans
            .iter()
            .filter_map(|s| s.stats.e2e_ms)
            .max()
            .unwrap_or(0);
        let max_tool = digest
            .spans
            .iter()
            .filter_map(|s| s.stats.tool_ms)
            .max()
            .unwrap_or(0);

        println!(
            "Signals: failures={} bottleneck={}ms tool_wait={}ms",
            total_failures, max_e2e, max_tool
        );
        println!();
    }
}

fn output_diagnose(digests: &[SessionDigest]) {
    println!("# Diagnose Report\n");

    println!("## Failures");
    for digest in digests
        .iter()
        .filter(|d| d.spans.iter().any(|s| s.stats.tool_failures > 0))
    {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        println!("### Session {}", id_short);
        if let Some(opening) = &digest.opening {
            println!("{}", opening);
        }
        println!();
    }

    println!("## Bottlenecks");
    for digest in digests
        .iter()
        .filter(|d| d.spans.iter().any(|s| s.stats.e2e_ms.unwrap_or(0) > 30_000))
    {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        println!("### Session {}", id_short);
        if let Some(opening) = &digest.opening {
            println!("{}", opening);
        }
        println!();
    }

    println!("## Toolchains");
    for digest in digests
        .iter()
        .filter(|d| d.spans.iter().map(|s| s.stats.tool_calls).sum::<usize>() > 10)
    {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        println!("### Session {}", id_short);
        if let Some(opening) = &digest.opening {
            println!("{}", opening);
        }
        println!();
    }
}

fn output_tools(digests: &[SessionDigest]) {
    println!("# Tools Report\n");

    println!("## Toolchains");
    for digest in digests
        .iter()
        .filter(|d| d.spans.iter().map(|s| s.stats.tool_calls).sum::<usize>() > 5)
    {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        println!("### Session {}", id_short);
        if let Some(opening) = &digest.opening {
            println!("{}", opening);
        }
        println!();
    }

    println!("## Bottlenecks");
    for digest in digests.iter().filter(|d| {
        d.spans
            .iter()
            .any(|s| s.stats.tool_ms.unwrap_or(0) > 10_000)
    }) {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        println!("### Session {}", id_short);
        if let Some(opening) = &digest.opening {
            println!("{}", opening);
        }
        println!();
    }
}

fn print_span_compact(span: &Span) {
    if let Some(user) = &span.user {
        println!("  User: {}", truncate_string(&user.text, 80));
    }

    for tool in &span.tools {
        let status = match &tool.status {
            Some(s) => format!("{:?}", s),
            None => "unknown".to_string(),
        };
        println!(
            "    Tool {}: {} ({})",
            tool.tool_name, tool.input_summary, status
        );
    }
}

fn truncate_string(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len.min(s.len())]
    }
}
