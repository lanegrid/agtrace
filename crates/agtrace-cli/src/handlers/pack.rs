use crate::output::{format_spans_compact, CompactFormatOpts};
use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::{build_spans, Span};
use agtrace_index::{Database, SessionSummary};
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;
use std::collections::HashMap;

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

    // Load sessions with provider balance
    // Fetch more sessions to ensure diversity across providers
    let raw_sessions = db.list_sessions(effective_project_hash, limit * 10)?;

    // Balance sessions by provider (take up to limit*2 per provider)
    let balanced_sessions = balance_sessions_by_provider(&raw_sessions, limit * 2);

    let mut digests = Vec::new();
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();

    // Build digests for each session
    for (i, session) in balanced_sessions.iter().enumerate() {
        if let Ok(events) = loader.load_events(&session.id, &options) {
            let spans = build_spans(&events);
            if !spans.is_empty() {
                // Recency boost: newer sessions get higher score (simple: reverse index)
                let recency_boost = (balanced_sessions.len() - i) as u32;
                let digest =
                    SessionDigest::new(&session.id, &session.provider, spans, recency_boost);
                digests.push(digest);
            }
        }
    }

    // Select sessions per lens based on template
    let selected = select_sessions_for_template(template, &digests, limit);

    // Output based on template
    match template {
        "compact" => output_compact(&selected),
        "diagnose" => output_diagnose(&selected),
        "tools" => output_tools(&selected),
        _ => output_compact(&selected),
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lens {
    Failures,
    Bottlenecks,
    Toolchains,
    Loops,
}

impl Lens {
    fn predicate(&self, metrics: &SessionMetrics) -> bool {
        match self {
            Lens::Failures => metrics.tool_failures_total > 0 || metrics.missing_tool_pairs > 0,
            Lens::Bottlenecks => metrics.max_e2e_ms > 30_000 || metrics.max_tool_ms > 20_000,
            Lens::Toolchains => metrics.tool_calls_total >= 20,
            Lens::Loops => metrics.loop_signals > 0,
        }
    }

    fn score(&self, metrics: &SessionMetrics, recency_boost: u32) -> i64 {
        let base_score = match self {
            Lens::Failures => {
                (metrics.tool_failures_total as i64) * 100
                    + (metrics.missing_tool_pairs as i64) * 50
            }
            Lens::Bottlenecks => (metrics.max_tool_ms as i64) + (metrics.max_e2e_ms as i64) / 2,
            Lens::Toolchains => {
                (metrics.tool_calls_total as i64) * 10 + (metrics.loop_signals as i64) * 20
            }
            Lens::Loops => (metrics.loop_signals as i64) * 100 + (metrics.tool_calls_total as i64),
        };
        base_score + (recency_boost as i64)
    }
}

#[derive(Debug, Clone)]
struct SessionMetrics {
    tool_calls_total: usize,
    tool_failures_total: usize,
    max_e2e_ms: u64,
    max_tool_ms: u64,
    missing_tool_pairs: usize,
    loop_signals: usize,
}

#[derive(Debug, Clone)]
struct SessionDigest {
    session_id: String,
    source: String,
    spans: Vec<Span>,
    opening: Option<String>,
    activation: Option<String>,
    outcome: String,
    metrics: SessionMetrics,
    recency_boost: u32,
    importance_score: u32,
}

impl SessionDigest {
    fn new(session_id: &str, provider: &str, spans: Vec<Span>, recency_boost: u32) -> Self {
        let opening = spans
            .first()
            .and_then(|s| s.user.as_ref().map(|u| truncate_string(&u.text, 100)));

        let activation = find_activation(&spans);
        let outcome = compute_outcome(&spans);
        let metrics = compute_metrics(&spans);
        let importance_score = compute_importance_score_from_metrics(&metrics);

        Self {
            session_id: session_id.to_string(),
            source: provider.to_string(),
            spans,
            opening,
            activation,
            outcome,
            metrics,
            recency_boost,
            importance_score,
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

fn select_sessions_for_template(
    template: &str,
    digests: &[SessionDigest],
    limit: usize,
) -> Vec<SessionDigest> {
    match template {
        "compact" => {
            // Use importance score (general)
            let mut sorted = digests.to_vec();
            sorted.sort_by(|a, b| b.importance_score.cmp(&a.importance_score));
            sorted.truncate(limit);
            sorted
        }
        "diagnose" => {
            // Select top sessions per lens, deduplicate
            let lenses = vec![
                Lens::Failures,
                Lens::Bottlenecks,
                Lens::Toolchains,
                Lens::Loops,
            ];
            select_per_lens(&lenses, digests, limit)
        }
        "tools" => {
            // Emphasize Toolchains and Bottlenecks
            let lenses = vec![Lens::Toolchains, Lens::Bottlenecks];
            select_per_lens(&lenses, digests, limit)
        }
        _ => {
            // Default: use importance score
            let mut sorted = digests.to_vec();
            sorted.sort_by(|a, b| b.importance_score.cmp(&a.importance_score));
            sorted.truncate(limit);
            sorted
        }
    }
}

fn select_per_lens(
    lenses: &[Lens],
    digests: &[SessionDigest],
    total_limit: usize,
) -> Vec<SessionDigest> {
    use std::collections::HashSet;

    let per_lens_limit = (total_limit / lenses.len()).max(1);
    let mut selected = Vec::new();
    let mut used_ids = HashSet::new();

    for lens in lenses {
        // Filter by predicate
        let mut candidates: Vec<_> = digests
            .iter()
            .filter(|d| lens.predicate(&d.metrics))
            .collect();

        // Sort by lens-specific score
        candidates.sort_by(|a, b| {
            let score_a = lens.score(&a.metrics, a.recency_boost);
            let score_b = lens.score(&b.metrics, b.recency_boost);
            score_b.cmp(&score_a)
        });

        // Take top N, deduplicate
        for candidate in candidates.iter().take(per_lens_limit * 2) {
            if !used_ids.contains(&candidate.session_id) {
                used_ids.insert(candidate.session_id.clone());
                selected.push((*candidate).clone());
                if selected.len() >= total_limit {
                    return selected;
                }
            }
        }
    }

    selected
}

fn find_activation(spans: &[Span]) -> Option<String> {
    for i in 0..spans.len() {
        let end = (i + 5).min(spans.len());
        let tool_count: usize = spans[i..end].iter().map(|s| s.stats.tool_calls).sum();

        if tool_count >= 3 {
            return spans[i]
                .user
                .as_ref()
                .map(|u| truncate_string(&u.text, 100));
        }
    }

    spans
        .first()
        .and_then(|s| s.user.as_ref().map(|u| truncate_string(&u.text, 100)))
}

fn compute_outcome(spans: &[Span]) -> String {
    let total_failures: usize = spans.iter().map(|s| s.stats.tool_failures).sum();

    if total_failures > 0 {
        format!("completed with {} failures", total_failures)
    } else {
        "completed".to_string()
    }
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

    // missing_tool_pairs: count tools without matching results (v0.1: placeholder)
    let missing_tool_pairs = 0;

    // loop_signals: detect potential loops (v0.1: simple heuristic)
    let loop_signals = spans.iter().filter(|s| s.stats.tool_calls > 5).count();

    SessionMetrics {
        tool_calls_total,
        tool_failures_total,
        max_e2e_ms,
        max_tool_ms,
        missing_tool_pairs,
        loop_signals,
    }
}

fn compute_importance_score_from_metrics(metrics: &SessionMetrics) -> u32 {
    let mut score = 0u32;

    if metrics.tool_failures_total > 0 {
        score += 5;
    }

    if metrics.max_e2e_ms > 30_000 {
        score += 3;
    }

    if metrics.max_tool_ms > 20_000 {
        score += 3;
    }

    if metrics.tool_calls_total > 30 {
        score += 2;
    }

    if metrics.loop_signals > 0 {
        score += 2;
    }

    score
}

fn output_compact(digests: &[SessionDigest]) {
    let opts = CompactFormatOpts {
        enable_color: false,
        relative_time: false,
    };

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
        let lines = format_spans_compact(&digest.spans, &opts);
        for line in lines.iter().take(30) {
            println!("  {}", line);
        }

        println!("Outcome: {}", digest.outcome);

        println!(
            "Signals: failures={} bottleneck={}ms tool_wait={}ms",
            digest.metrics.tool_failures_total,
            digest.metrics.max_e2e_ms,
            digest.metrics.max_tool_ms
        );
        println!();
    }
}

fn output_diagnose(digests: &[SessionDigest]) {
    println!("# Diagnose Report\n");

    println!("## Failures");
    for digest in digests.iter().filter(|d| d.metrics.tool_failures_total > 0) {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        println!("### Session {}", id_short);
        if let Some(opening) = &digest.opening {
            println!("{}", opening);
        }
        println!();
    }

    println!("## Bottlenecks");
    for digest in digests.iter().filter(|d| d.metrics.max_e2e_ms > 30_000) {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        println!("### Session {}", id_short);
        if let Some(opening) = &digest.opening {
            println!("{}", opening);
        }
        println!();
    }

    println!("## Toolchains");
    for digest in digests.iter().filter(|d| d.metrics.tool_calls_total > 10) {
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
    for digest in digests.iter().filter(|d| d.metrics.tool_calls_total > 5) {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        println!("### Session {}", id_short);
        if let Some(opening) = &digest.opening {
            println!("{}", opening);
        }
        println!();
    }

    println!("## Bottlenecks");
    for digest in digests.iter().filter(|d| d.metrics.max_tool_ms > 10_000) {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        println!("### Session {}", id_short);
        if let Some(opening) = &digest.opening {
            println!("{}", opening);
        }
        println!();
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let chars: Vec<char> = s.chars().take(max_len).collect();
        chars.iter().collect::<String>() + "..."
    }
}
