use super::digest::SessionDigest;
use super::metrics::SessionMetrics;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LensType {
    Failures,
    Bottlenecks,
    Toolchains,
    Loops,
}

type PredicateFn = Box<dyn Fn(&SessionMetrics, &Thresholds) -> bool>;
type ScoreFn = Box<dyn Fn(&SessionMetrics, u32) -> i64>;
type ReasonFn = Box<dyn Fn(&SessionMetrics) -> String>;

pub struct Lens {
    pub lens_type: LensType,
    predicate: PredicateFn,
    score: ScoreFn,
    reason: ReasonFn,
}

impl Lens {
    pub fn failures() -> Self {
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

    pub fn bottlenecks() -> Self {
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

    pub fn toolchains() -> Self {
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

    pub fn loops() -> Self {
        Self {
            lens_type: LensType::Loops,
            predicate: Box::new(|m, _| m.loop_signals > 0),
            score: Box::new(|m, boost| (m.loop_signals as i64 * 100) + (boost as i64)),
            reason: Box::new(|m| format!("loop_signals={}", m.loop_signals)),
        }
    }

    pub fn matches(&self, metrics: &SessionMetrics, thresholds: &Thresholds) -> bool {
        (self.predicate)(metrics, thresholds)
    }

    pub fn score(&self, metrics: &SessionMetrics, recency_boost: u32) -> i64 {
        (self.score)(metrics, recency_boost)
    }

    pub fn reason(&self, metrics: &SessionMetrics) -> String {
        (self.reason)(metrics)
    }
}

#[derive(Debug, Clone)]
pub struct Thresholds {
    pub p90_e2e_ms: u64,
    pub p90_tool_ms: u64,
    pub p90_tool_calls: usize,
}

impl Thresholds {
    pub fn compute(digests: &[SessionDigest]) -> Self {
        let mut e2e_times: Vec<u64> = digests.iter().map(|d| d.metrics.max_e2e_ms).collect();
        let mut tool_times: Vec<u64> = digests.iter().map(|d| d.metrics.max_tool_ms).collect();
        let mut call_counts: Vec<usize> =
            digests.iter().map(|d| d.metrics.tool_calls_total).collect();

        e2e_times.sort_unstable();
        tool_times.sort_unstable();
        call_counts.sort_unstable();

        let p90_idx = (digests.len() as f64 * 0.9) as usize;
        let idx = p90_idx.min(digests.len().saturating_sub(1));

        Self {
            p90_e2e_ms: *e2e_times.get(idx).unwrap_or(&5000),
            p90_tool_ms: *tool_times.get(idx).unwrap_or(&5000),
            p90_tool_calls: *call_counts.get(idx).unwrap_or(&10),
        }
    }
}

pub fn select_sessions_by_lenses(
    digests: &[SessionDigest],
    thresholds: &Thresholds,
    total_limit: usize,
) -> Vec<SessionDigest> {
    let lenses = vec![
        Lens::failures(),
        Lens::loops(),
        Lens::bottlenecks(),
        Lens::toolchains(),
    ];

    let limit_per_lens = (total_limit / lenses.len()).max(1);
    let mut selected_sessions = Vec::new();
    let mut used_ids = HashSet::new();

    for lens in lenses {
        let mut candidates: Vec<SessionDigest> = digests
            .iter()
            .filter(|d| !used_ids.contains(&d.session_id))
            .filter(|d| lens.matches(&d.metrics, thresholds))
            .cloned()
            .collect();

        candidates.sort_by(|a, b| {
            let score_a = lens.score(&a.metrics, a.recency_boost);
            let score_b = lens.score(&b.metrics, b.recency_boost);
            score_b.cmp(&score_a)
        });

        for mut candidate in candidates.into_iter().take(limit_per_lens) {
            candidate.selection_reason = Some(format!(
                "{:?} ({})",
                lens.lens_type,
                lens.reason(&candidate.metrics)
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
