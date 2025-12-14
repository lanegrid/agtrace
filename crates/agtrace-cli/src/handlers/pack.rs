use crate::output::{format_session_compact, CompactFormatOpts};
use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::{assemble_session_from_events, AgentSession};
use agtrace_index::{Database, SessionSummary};
use agtrace_types::resolve_effective_project_hash;
use anyhow::Result;
use std::collections::{HashMap, HashSet};

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

    // 1. Collect: Balance sessions by provider to avoid bias
    // Fetch enough raw sessions to ensure we can balance them
    let raw_sessions = db.list_sessions(effective_project_hash, 1000)?;
    let balanced_sessions = balance_sessions_by_provider(&raw_sessions, 200);

    println!(
        "# Packing Report (pool: {} sessions from {} raw candidates)\n",
        balanced_sessions.len(),
        raw_sessions.len()
    );

    // 2. Summarize: Calculate metrics for all candidates
    let mut digests = Vec::new();
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();

    for (i, session) in balanced_sessions.iter().enumerate() {
        if let Ok(events) = loader.load_events_v2(&session.id, &options) {
            if let Some(agent_session) = assemble_session_from_events(&events) {
                // Newer sessions get a small boost in scoring
                let recency_boost = (balanced_sessions.len() - i) as u32;
                let digest = SessionDigest::new(
                    &session.id,
                    &session.provider,
                    agent_session,
                    recency_boost,
                );
                digests.push(digest);
            }
        }
    }

    // Calculate dynamic thresholds (P90) for bottlenecks
    let thresholds = Thresholds::compute(&digests);

    // 3. Select: Apply lenses with deduplication
    // This ensures one session doesn't appear in multiple categories
    let selections = select_sessions_by_lenses(&digests, &thresholds, limit);

    // Output based on template
    match template {
        "compact" => output_compact(&selections),
        "diagnose" => output_diagnose(&selections),
        "tools" => output_tools(&selections),
        _ => output_compact(&selections),
    }

    Ok(())
}

// --- Data Structures ---

#[derive(Debug, Clone)]
struct SessionMetrics {
    tool_calls_total: usize,
    tool_failures_total: usize,
    max_e2e_ms: u64,
    max_tool_ms: u64,
    missing_tool_pairs: usize,
    loop_signals: usize,
    longest_chain: usize,
}

#[derive(Debug, Clone)]
struct SessionDigest {
    session_id: String,
    source: String,
    session: AgentSession,
    opening: Option<String>,
    activation: Option<String>,
    metrics: SessionMetrics,
    recency_boost: u32,
    // Why was this session selected? (Filled during selection)
    selection_reason: Option<String>,
}

struct Thresholds {
    p90_e2e_ms: u64,
    p90_tool_ms: u64,
    p90_tool_calls: usize,
}

// --- Lens Logic ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LensType {
    Failures,
    Bottlenecks,
    Toolchains,
    Loops,
}

type PredicateFn = Box<dyn Fn(&SessionMetrics, &Thresholds) -> bool>;
type ScoreFn = Box<dyn Fn(&SessionMetrics, u32) -> i64>;
type ReasonFn = Box<dyn Fn(&SessionMetrics) -> String>;

struct Lens {
    lens_type: LensType,
    // Predicate: Should we even consider this session for this lens?
    predicate: PredicateFn,
    // Score: How "interesting" is this session for this lens? (Higher is better)
    score: ScoreFn,
    // Formatter: Why did we pick this?
    reason: ReasonFn,
}

impl Lens {
    fn failures() -> Self {
        Self {
            lens_type: LensType::Failures,
            predicate: Box::new(|m, _| m.tool_failures_total > 0 || m.missing_tool_pairs > 0),
            score: Box::new(|m, boost| {
                (m.tool_failures_total as i64 * 100)
                    + (m.missing_tool_pairs as i64 * 50)
                    + (boost as i64)
            }),
            reason: Box::new(|m| {
                format!(
                    "fails={} missing={}",
                    m.tool_failures_total, m.missing_tool_pairs
                )
            }),
        }
    }

    fn bottlenecks() -> Self {
        Self {
            lens_type: LensType::Bottlenecks,
            predicate: Box::new(|m, t| {
                m.max_e2e_ms > t.p90_e2e_ms || m.max_tool_ms > t.p90_tool_ms
            }),
            score: Box::new(|m, _| (m.max_tool_ms as i64) + (m.max_e2e_ms as i64)),
            reason: Box::new(|m| {
                format!(
                    "max_tool={:.1}s max_e2e={:.1}s",
                    m.max_tool_ms as f64 / 1000.0,
                    m.max_e2e_ms as f64 / 1000.0
                )
            }),
        }
    }

    fn toolchains() -> Self {
        Self {
            lens_type: LensType::Toolchains,
            predicate: Box::new(|m, t| m.tool_calls_total > t.p90_tool_calls.max(5)),
            score: Box::new(|m, boost| (m.tool_calls_total as i64 * 10) + (boost as i64)),
            reason: Box::new(|m| {
                format!(
                    "tool_calls={} longest_chain={}",
                    m.tool_calls_total, m.longest_chain
                )
            }),
        }
    }

    fn loops() -> Self {
        Self {
            lens_type: LensType::Loops,
            predicate: Box::new(|m, _| m.loop_signals > 0),
            score: Box::new(|m, boost| (m.loop_signals as i64 * 100) + (boost as i64)),
            reason: Box::new(|m| format!("loop_signals={}", m.loop_signals)),
        }
    }
}

// --- Core Logic Implementation ---

fn balance_sessions_by_provider(
    sessions: &[SessionSummary],
    target_per_provider: usize,
) -> Vec<SessionSummary> {
    let mut by_provider: HashMap<String, Vec<SessionSummary>> = HashMap::new();
    for session in sessions {
        by_provider
            .entry(session.provider.clone())
            .or_default()
            .push(session.clone());
    }

    let mut balanced = Vec::new();
    for (_, mut list) in by_provider {
        // Keep most recent ones from each provider
        if list.len() > target_per_provider {
            list.truncate(target_per_provider);
        }
        balanced.extend(list);
    }

    // Sort by timestamp desc to keep overall chronological order slightly
    balanced.sort_by(|a, b| b.start_ts.cmp(&a.start_ts));
    balanced
}

impl Thresholds {
    fn compute(digests: &[SessionDigest]) -> Self {
        let mut e2e_times: Vec<u64> = digests.iter().map(|d| d.metrics.max_e2e_ms).collect();
        let mut tool_times: Vec<u64> = digests.iter().map(|d| d.metrics.max_tool_ms).collect();
        let mut call_counts: Vec<usize> =
            digests.iter().map(|d| d.metrics.tool_calls_total).collect();

        e2e_times.sort_unstable();
        tool_times.sort_unstable();
        call_counts.sort_unstable();

        let p90_idx = (digests.len() as f64 * 0.9) as usize;
        let idx = p90_idx.min(digests.len().saturating_sub(1));

        // Use defaults if empty to avoid crashes, but logic shouldn't allow empty here easily
        Self {
            p90_e2e_ms: *e2e_times.get(idx).unwrap_or(&5000),
            p90_tool_ms: *tool_times.get(idx).unwrap_or(&5000),
            p90_tool_calls: *call_counts.get(idx).unwrap_or(&10),
        }
    }
}

fn select_sessions_by_lenses(
    digests: &[SessionDigest],
    thresholds: &Thresholds,
    total_limit: usize,
) -> Vec<SessionDigest> {
    let lenses = vec![
        Lens::failures(),
        Lens::loops(), // Loops are high value to inspect
        Lens::bottlenecks(),
        Lens::toolchains(),
    ];

    let limit_per_lens = (total_limit / lenses.len()).max(1);
    let mut selected_sessions = Vec::new();
    let mut used_ids = HashSet::new();

    // Iterate through lenses in priority order
    for lens in lenses {
        let mut candidates: Vec<SessionDigest> = digests
            .iter()
            .filter(|d| !used_ids.contains(&d.session_id)) // Deduplication
            .filter(|d| (lens.predicate)(&d.metrics, thresholds))
            .cloned()
            .collect();

        // Sort by score
        candidates.sort_by(|a, b| {
            let score_a = (lens.score)(&a.metrics, a.recency_boost);
            let score_b = (lens.score)(&b.metrics, b.recency_boost);
            score_b.cmp(&score_a)
        });

        // Select top N
        for mut candidate in candidates.into_iter().take(limit_per_lens) {
            candidate.selection_reason = Some(format!(
                "{:?} ({})",
                lens.lens_type,
                (lens.reason)(&candidate.metrics)
            ));
            used_ids.insert(candidate.session_id.clone());
            selected_sessions.push(candidate);
        }
    }

    // Fill remaining slots with high activity sessions if needed
    if selected_sessions.len() < total_limit {
        let mut remaining: Vec<SessionDigest> = digests
            .iter()
            .filter(|d| !used_ids.contains(&d.session_id))
            .cloned()
            .collect();

        remaining.sort_by_key(|d| std::cmp::Reverse(d.metrics.tool_calls_total));

        for mut candidate in remaining
            .into_iter()
            .take(total_limit - selected_sessions.len())
        {
            candidate.selection_reason = Some("Activity (filler)".to_string());
            selected_sessions.push(candidate);
        }
    }

    selected_sessions
}

// --- Session Digest Construction & Cleaning ---

impl SessionDigest {
    fn new(session_id: &str, provider: &str, session: AgentSession, recency_boost: u32) -> Self {
        // Find the first meaningful user text (Opening)
        let opening = session
            .turns
            .first()
            .map(|t| clean_snippet(&t.user.content.text))
            .filter(|s| !s.is_empty())
            .map(|s| truncate_string(&s, 100));

        let metrics = compute_metrics(&session);
        let activation = find_activation(&session);

        Self {
            session_id: session_id.to_string(),
            source: provider.to_string(),
            session,
            opening,
            activation,
            metrics,
            recency_boost,
            selection_reason: None,
        }
    }
}

fn compute_metrics(session: &AgentSession) -> SessionMetrics {
    let mut tool_calls_total = 0;
    let mut tool_failures_total = 0;
    let mut missing_tool_pairs = 0;
    let mut max_tool_ms = 0i64;
    let mut longest_chain = 0;
    let mut tool_counts_per_turn = Vec::new();

    for turn in &session.turns {
        let mut turn_tool_count = 0;
        for step in &turn.steps {
            turn_tool_count += step.tools.len();
            tool_calls_total += step.tools.len();

            for tool_exec in &step.tools {
                if tool_exec.is_error {
                    tool_failures_total += 1;
                }
                if tool_exec.result.is_none() {
                    missing_tool_pairs += 1;
                }
                if let Some(duration_ms) = tool_exec.duration_ms {
                    if duration_ms > max_tool_ms {
                        max_tool_ms = duration_ms;
                    }
                }
            }

            if step.tools.len() > longest_chain {
                longest_chain = step.tools.len();
            }
        }
        tool_counts_per_turn.push(turn_tool_count);
    }

    // Calculate max e2e time (turn duration)
    let max_e2e_ms = session
        .turns
        .iter()
        .map(|turn| turn.stats.duration_ms)
        .max()
        .unwrap_or(0);

    // Heuristic: Loop detection (same turn has > 5 tool calls)
    let loop_signals = tool_counts_per_turn.iter().filter(|&&c| c > 5).count();

    SessionMetrics {
        tool_calls_total,
        tool_failures_total,
        max_e2e_ms: max_e2e_ms as u64,
        max_tool_ms: max_tool_ms as u64,
        missing_tool_pairs,
        loop_signals,
        longest_chain,
    }
}

// Extract the user prompt that triggered the most intense tool usage
fn find_activation(session: &AgentSession) -> Option<String> {
    if session.turns.is_empty() {
        return None;
    }

    // Find the turn with the most tool calls
    let (best_idx, max_tools) = session
        .turns
        .iter()
        .enumerate()
        .map(|(i, turn)| {
            let tool_count: usize = turn.steps.iter().map(|s| s.tools.len()).sum();
            (i, tool_count)
        })
        .max_by_key(|(_, count)| *count)
        .unwrap_or((0, 0));

    // Must have at least meaningful activity to be an "activation"
    if max_tools < 3 {
        return None;
    }

    session
        .turns
        .get(best_idx)
        .map(|turn| clean_snippet(&turn.user.content.text))
        .map(|s| truncate_string(&s, 120))
}

// Remove XML noise and environment dumps from snippets
fn clean_snippet(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Remove common large XML blocks (simple string finding)
    // In a real implementation with regex, this would be: Replace `<environment_context>.*?</environment_context>`
    let noise_tags = [
        ("<environment_context>", "</environment_context>"),
        ("<command_output>", "</command_output>"), // Often noisy if captured in user text
        ("<changed_files>", "</changed_files>"),
    ];

    for (start_tag, end_tag) in noise_tags {
        while let Some(start_idx) = cleaned.find(start_tag) {
            if let Some(end_idx) = cleaned[start_idx..].find(end_tag) {
                let absolute_end = start_idx + end_idx + end_tag.len();
                cleaned.replace_range(start_idx..absolute_end, " [..meta..] ");
            } else {
                break;
            }
        }
    }

    // Collapse whitespace
    let fields: Vec<&str> = cleaned.split_whitespace().collect();
    fields.join(" ")
}

// --- Output Formatters ---

fn output_diagnose(digests: &[SessionDigest]) {
    println!("## Selected Sessions (Diagnose Mode)\n");

    // Group by lens for display
    let mut by_reason: HashMap<String, Vec<&SessionDigest>> = HashMap::new();
    for d in digests {
        let key = d
            .selection_reason
            .as_deref()
            .unwrap_or("Other")
            .split(' ')
            .next()
            .unwrap_or("Other");
        by_reason.entry(key.to_string()).or_default().push(d);
    }

    for (category, list) in by_reason {
        println!("### {}\n", category);
        for digest in list {
            print_digest_summary(digest);
        }
        println!();
    }
}

fn output_tools(digests: &[SessionDigest]) {
    output_compact(digests);
}

fn output_compact(digests: &[SessionDigest]) {
    let opts = CompactFormatOpts {
        enable_color: false,
        relative_time: false,
    };

    for digest in digests {
        print_digest_summary(digest);

        println!("Work:");
        let lines = format_session_compact(&digest.session, &opts);
        // Show first few lines of work
        for line in lines.iter().take(15) {
            println!("  {}", line);
        }
        if lines.len() > 15 {
            println!("  ... ({} more lines)", lines.len() - 15);
        }
        println!();
    }
}

fn print_digest_summary(digest: &SessionDigest) {
    let id_short = &digest.session_id[..8.min(digest.session_id.len())];
    let reason = digest.selection_reason.as_deref().unwrap_or("");

    println!("Session {} ({}) -- {}", id_short, digest.source, reason);

    if let Some(opening) = &digest.opening {
        println!("  Opening: \"{}\"", opening);
    }
    if let Some(activation) = &digest.activation {
        if digest.opening.as_ref() != Some(activation) {
            println!("  Activation: \"{}\"", activation);
        }
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
