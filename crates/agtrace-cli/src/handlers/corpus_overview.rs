use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::{build_spans, Span};
use agtrace_index::{Database, SessionSummary};
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;
use std::collections::HashMap;

pub fn handle(db: &Database, project_hash: Option<String>, all_projects: bool) -> Result<()> {
    let (effective_hash_string, _all_projects) =
        resolve_effective_project_hash(project_hash.as_deref(), all_projects)?;
    let effective_project_hash = effective_hash_string.as_deref();

    // Load sessions with provider balance
    let raw_sessions = db.list_sessions(effective_project_hash, 300)?;

    if raw_sessions.is_empty() {
        println!("No sessions found.");
        println!("\nRun `agtrace index update` to scan for sessions.");
        return Ok(());
    }

    // Balance sessions by provider (take up to 100 per provider for overview)
    let sessions = balance_sessions_by_provider(&raw_sessions, 100);

    println!("# Corpus Overview\n");
    println!("Total sessions: {}\n", sessions.len());

    let mut digests = Vec::new();
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();

    // Build digests for each session
    for (i, session) in sessions.iter().enumerate() {
        if let Ok(events) = loader.load_events(&session.id, &options) {
            let spans = build_spans(&events);
            if !spans.is_empty() {
                let recency_boost = (sessions.len() - i) as u32;
                let digest =
                    CorpusDigest::new(&session.id, &session.provider, spans, recency_boost);
                digests.push(digest);
            }
        }
    }

    // Group by lens
    let lenses = group_by_lens(&digests);

    // Display each lens with count and 1 example
    display_lens("Failures", Lens::Failures, &lenses.failures, &digests);
    display_lens(
        "Bottlenecks",
        Lens::Bottlenecks,
        &lenses.bottlenecks,
        &digests,
    );
    display_lens("Toolchains", Lens::Toolchains, &lenses.toolchains, &digests);
    display_lens("Loops", Lens::Loops, &lenses.loops, &digests);

    println!("\nRun `agtrace pack` to generate LLM-ready context from important sessions.");

    Ok(())
}

#[derive(Debug, Clone)]
struct SessionMetrics {
    tool_calls_total: usize,
    tool_failures_total: usize,
    max_e2e_ms: u64,
    max_tool_ms: u64,
    loop_signals: usize,
}

#[derive(Debug, Clone, Copy)]
enum Lens {
    Failures,
    Bottlenecks,
    Toolchains,
    Loops,
}

impl Lens {
    fn predicate(&self, metrics: &SessionMetrics) -> bool {
        match self {
            Lens::Failures => metrics.tool_failures_total > 0,
            Lens::Bottlenecks => metrics.max_e2e_ms > 30_000 || metrics.max_tool_ms > 20_000,
            Lens::Toolchains => metrics.tool_calls_total >= 20,
            Lens::Loops => metrics.loop_signals > 0,
        }
    }

    fn score(&self, metrics: &SessionMetrics, recency_boost: u32) -> i64 {
        let base_score = match self {
            Lens::Failures => (metrics.tool_failures_total as i64) * 100,
            Lens::Bottlenecks => (metrics.max_tool_ms as i64) + (metrics.max_e2e_ms as i64) / 2,
            Lens::Toolchains => {
                (metrics.tool_calls_total as i64) * 10 + (metrics.loop_signals as i64) * 20
            }
            Lens::Loops => (metrics.loop_signals as i64) * 100 + (metrics.tool_calls_total as i64),
        };
        base_score + (recency_boost as i64)
    }
}

#[derive(Debug)]
struct CorpusDigest {
    session_id: String,
    provider: String,
    spans: Vec<Span>,
    opening: Option<String>,
    metrics: SessionMetrics,
    recency_boost: u32,
}

impl CorpusDigest {
    fn new(session_id: &str, provider: &str, spans: Vec<Span>, recency_boost: u32) -> Self {
        let opening = spans
            .first()
            .and_then(|s| s.user.as_ref().map(|u| truncate_string(&u.text, 100)));

        let metrics = compute_metrics(&spans);

        Self {
            session_id: session_id.to_string(),
            provider: provider.to_string(),
            spans,
            opening,
            metrics,
            recency_boost,
        }
    }
}

fn balance_sessions_by_provider(
    sessions: &[SessionSummary],
    per_provider_limit: usize,
) -> Vec<SessionSummary> {
    let mut by_provider: HashMap<String, Vec<SessionSummary>> = HashMap::new();

    // Group sessions by provider
    for session in sessions {
        by_provider
            .entry(session.provider.clone())
            .or_default()
            .push(session.clone());
    }

    // Take up to per_provider_limit from each provider
    let mut balanced = Vec::new();
    for (_, mut provider_sessions) in by_provider {
        provider_sessions.truncate(per_provider_limit);
        balanced.extend(provider_sessions);
    }

    balanced
}

fn compute_metrics(spans: &[Span]) -> SessionMetrics {
    let tool_calls_total: usize = spans.iter().map(|s| s.stats.tool_calls).sum();
    let tool_failures_total: usize = spans.iter().map(|s| s.stats.tool_failures).sum();

    let max_e2e_ms = spans
        .iter()
        .filter_map(|s| s.stats.e2e_ms)
        .max()
        .unwrap_or(0);

    let max_tool_ms = spans
        .iter()
        .filter_map(|s| s.stats.tool_ms)
        .max()
        .unwrap_or(0);

    let loop_signals = spans.iter().filter(|s| s.stats.tool_calls > 5).count();

    SessionMetrics {
        tool_calls_total,
        tool_failures_total,
        max_e2e_ms,
        max_tool_ms,
        loop_signals,
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
        // Use lens predicates
        if Lens::Failures.predicate(&digest.metrics) {
            groups.failures.push(i);
        }

        if Lens::Bottlenecks.predicate(&digest.metrics) {
            groups.bottlenecks.push(i);
        }

        if Lens::Toolchains.predicate(&digest.metrics) {
            groups.toolchains.push(i);
        }

        if Lens::Loops.predicate(&digest.metrics) {
            groups.loops.push(i);
        }
    }

    groups
}

fn display_lens(name: &str, lens: Lens, indices: &[usize], digests: &[CorpusDigest]) {
    println!("## {}", name);
    println!("Count: {}", indices.len());

    if !indices.is_empty() {
        // Select best example by lens-specific score
        let best_idx = indices
            .iter()
            .max_by_key(|&&i| {
                if let Some(digest) = digests.get(i) {
                    lens.score(&digest.metrics, digest.recency_boost)
                } else {
                    0
                }
            })
            .copied();

        if let Some(idx) = best_idx {
            if let Some(digest) = digests.get(idx) {
                let id_short = &digest.session_id[..8.min(digest.session_id.len())];
                println!("Example: Session {} ({})", id_short, digest.provider);
                if let Some(opening) = &digest.opening {
                    println!("  {}", opening);
                }
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
