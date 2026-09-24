//! Seam to the context-window resolver (design §3, built in a sibling PR).
//!
//! The workspace fold only needs two things from the resolver:
//! - per-agent evidence that is updated in O(1) per event ([`ContextEvidence`]), and
//! - a function turning that evidence into a window ([`WindowResolver`]).
//!
//! Until `crate::context` lands, [`ContextEvidence`] and [`ContextWindow`] below are
//! minimal stand-ins with the design's field names. Wiring the real resolver means:
//! delete the two stand-in types, re-export `crate::context::ContextEvidence` and
//! `agtrace_types::ContextWindow` from here, and implement [`WindowResolver`] for a
//! wrapper around `&dyn ModelCatalog` that calls `context::resolve`.

use agtrace_types::{AgentEvent, AgentRef, ContextWindowHintPayload, EventPayload};

/// Resolves an agent's context window from its accumulated evidence.
pub trait WindowResolver {
    fn resolve(&self, agent: &AgentRef, evidence: &ContextEvidence) -> Option<ContextWindow>;
}

/// Resolver that never knows the window (tests, or before the catalog is available).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoWindow;

impl WindowResolver for NoWindow {
    fn resolve(&self, _agent: &AgentRef, _evidence: &ContextEvidence) -> Option<ContextWindow> {
        None
    }
}

impl<F> WindowResolver for F
where
    F: Fn(&AgentRef, &ContextEvidence) -> Option<ContextWindow>,
{
    fn resolve(&self, agent: &AgentRef, evidence: &ContextEvidence) -> Option<ContextWindow> {
        self(agent, evidence)
    }
}

/// Stand-in for `agtrace_types::ContextWindow` (resolved window size).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextWindow {
    pub tokens: u64,
    pub model: Option<String>,
}

/// Stand-in for `crate::context::ContextEvidence` (same field names as the design).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContextEvidence {
    pub model: Option<String>,
    pub explicit: Option<u64>,
    pub marker: Option<u64>,
    pub external_marker: Option<u64>,
    pub peak_context_tokens: u64,
    pub max_compaction_pre_tokens: u64,
    pub last_context_tokens: u64,
}

impl ContextEvidence {
    /// Fold one event of the agent's own log. Returns true if anything changed.
    pub fn apply(&mut self, ev: &AgentEvent) -> bool {
        let before = self.clone();
        match &ev.payload {
            EventPayload::TokenUsage(u) => {
                if let Some(m) = &u.model {
                    self.model = Some(m.clone());
                }
                let ctx = u.context_tokens();
                self.last_context_tokens = ctx;
                self.peak_context_tokens = self.peak_context_tokens.max(ctx);
            }
            EventPayload::ModelChange(m) => self.model = Some(m.to.clone()),
            EventPayload::ContextWindowHint(ContextWindowHintPayload::Explicit {
                tokens, ..
            }) => self.explicit = Some(*tokens),
            EventPayload::ContextWindowHint(ContextWindowHintPayload::ExtendedMarker {
                tokens,
                ..
            }) => self.marker = Some(self.marker.unwrap_or(0).max(*tokens)),
            EventPayload::Compaction(c) => {
                if let Some(pre) = c.pre_tokens {
                    self.max_compaction_pre_tokens = self.max_compaction_pre_tokens.max(pre);
                }
                if let Some(post) = c.post_tokens {
                    self.last_context_tokens = post;
                }
            }
            _ => {}
        }
        *self != before
    }

    /// Evidence from outside the agent's own log (team config member model, subagent
    /// meta model, spawn `resolved_model`). Returns true if anything changed.
    pub fn apply_external_model(&mut self, model: &str) -> bool {
        if model.ends_with("[1m]") && self.external_marker != Some(1_000_000) {
            self.external_marker = Some(1_000_000);
            return true;
        }
        false
    }
}
