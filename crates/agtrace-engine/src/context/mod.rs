//! Context window resolver (design §3): the single place where an agent's
//! context window is resolved from in-log evidence (`ContextWindowHint`,
//! `TokenUsage`, `ModelChange`, `Compaction`) plus an injected [`ModelCatalog`].
//!
//! Callers fold every event of one agent into a [`ContextEvidence`] (O(1) per event)
//! and call [`resolve`] whenever they need the window. Nothing else in the workspace
//! computes context limits.

use std::collections::BTreeMap;

use agtrace_types::{
    AgentEvent, ContextSource, ContextWindow, ContextWindowHintPayload, EventPayload, ModelCatalog,
    Provider, has_extended_context_suffix, normalize_model_id,
};

/// Implied window of an extended-context marker (`[1m]`).
pub const EXTENDED_CONTEXT_TOKENS: u64 = 1_000_000;

/// Known window sizes used only by the observed floor / observed fallback.
pub const TIERS: &[u64] = &[200_000, 258_400, 272_000, 400_000, 1_000_000];

/// Marker key used when a marker arrives before any model is known.
const UNBOUND: &str = "";

/// Per-agent context-window evidence, folded from events.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ContextEvidence {
    /// Latest model (ModelChange / TokenUsage.model / ContextWindowHint model).
    pub model: Option<String>,
    /// Latest explicit window from the log (`ContextWindowHint::Explicit`).
    pub explicit: Option<u64>,
    /// Max extended-context marker applying to the current model family.
    pub marker: Option<u64>,
    /// Marker from side-state outside the log (team config / subagent meta.json model
    /// with `[1m]`). Set by the agent graph, never by [`ContextEvidence::apply`].
    pub external_marker: Option<u64>,
    /// Max `TokenUsage::context_tokens()` seen.
    pub peak_context_tokens: u64,
    /// Max `Compaction.pre_tokens` seen.
    pub max_compaction_pre_tokens: u64,
    /// Current occupancy: context tokens of the latest usage (or compaction post_tokens).
    pub last_context_tokens: u64,
    /// Markers keyed by normalized model id (`UNBOUND` = model not known yet).
    markers: BTreeMap<String, u64>,
}

impl ContextEvidence {
    pub fn new() -> Self {
        Self::default()
    }

    /// Evidence with only a known model (as if a `ModelChange` to `model` was applied).
    pub fn with_model(model: &str) -> Self {
        let mut ev = Self::default();
        ev.set_model(model);
        ev
    }

    /// Fold one event. O(1) (marker map is tiny: one entry per model family seen).
    pub fn apply(&mut self, event: &AgentEvent) {
        match &event.payload {
            EventPayload::TokenUsage(usage) => {
                if let Some(model) = &usage.model {
                    self.set_model(model);
                }
                let tokens = usage.context_tokens();
                self.peak_context_tokens = self.peak_context_tokens.max(tokens);
                self.last_context_tokens = tokens;
            }
            EventPayload::ModelChange(change) => self.set_model(&change.to),
            EventPayload::ContextWindowHint(ContextWindowHintPayload::Explicit {
                tokens,
                model,
            }) => {
                if let Some(model) = model {
                    self.set_model(model);
                }
                self.explicit = Some(*tokens);
            }
            EventPayload::ContextWindowHint(ContextWindowHintPayload::ExtendedMarker {
                model,
                tokens,
                ..
            }) => self.add_marker(model, *tokens),
            EventPayload::Compaction(compaction) => {
                if let Some(pre) = compaction.pre_tokens {
                    self.max_compaction_pre_tokens = self.max_compaction_pre_tokens.max(pre);
                }
                if let Some(post) = compaction.post_tokens {
                    self.last_context_tokens = post;
                }
            }
            _ => {}
        }
    }

    /// Largest usage-derived lower bound on the window.
    pub fn observed_floor(&self) -> u64 {
        self.peak_context_tokens.max(self.max_compaction_pre_tokens)
    }

    fn set_model(&mut self, model: &str) {
        if model.trim().is_empty() {
            return;
        }
        if has_extended_context_suffix(model) {
            self.markers
                .entry(normalize_model_id(model))
                .and_modify(|t| *t = (*t).max(EXTENDED_CONTEXT_TOKENS))
                .or_insert(EXTENDED_CONTEXT_TOKENS);
        }
        // Markers that arrived before any model was known belong to the first model.
        if let Some(unbound) = self.markers.remove(UNBOUND) {
            let key = normalize_model_id(model);
            let entry = self.markers.entry(key).or_insert(unbound);
            *entry = (*entry).max(unbound);
        }
        self.model = Some(model.to_string());
        self.recompute_marker();
    }

    fn add_marker(&mut self, model: &str, tokens: u64) {
        // A marker names a model id ("claude-opus-5-5[1m]") or a marketing name
        // ("Opus 5.5 (1M context)"). Marketing names cannot be matched against ids,
        // so they bind to the current model (or to the next model seen).
        let key = if looks_like_model_id(model) {
            normalize_model_id(model)
        } else {
            self.model
                .as_deref()
                .map(normalize_model_id)
                .unwrap_or_else(|| UNBOUND.to_string())
        };
        let entry = self.markers.entry(key).or_insert(tokens);
        *entry = (*entry).max(tokens);
        self.recompute_marker();
    }

    fn recompute_marker(&mut self) {
        self.marker = match &self.model {
            Some(model) => {
                let key = normalize_model_id(model);
                self.markers.get(&key).copied()
            }
            None => self.markers.values().copied().max(),
        };
    }
}

fn looks_like_model_id(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty() && !s.contains(char::is_whitespace)
}

/// Smallest known tier ≥ `tokens` (or `tokens` itself when above every tier).
pub fn tier_at_least(tokens: u64) -> u64 {
    TIERS
        .iter()
        .copied()
        .find(|&t| t >= tokens)
        .unwrap_or(tokens)
}

/// Resolve the context window of one agent.
///
/// Order (first hit wins): user config → log → model marker → provider cache →
/// model table → observed. For every layer but the user config, a window smaller than
/// the observed usage is bumped to the smallest tier that fits (`source = Observed`).
pub fn resolve(
    provider: Provider,
    ev: &ContextEvidence,
    cat: &dyn ModelCatalog,
) -> Option<ContextWindow> {
    let model = ev.model.clone();

    if let Some(tokens) = cat.user_override(provider, model.as_deref()) {
        return Some(ContextWindow {
            tokens,
            source: ContextSource::UserConfig,
            model,
            overruled: None,
        });
    }

    let marker = match (ev.marker, ev.external_marker) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };

    let resolved = ev
        .explicit
        .map(|t| (t, ContextSource::Log))
        .or_else(|| marker.map(|t| (t, ContextSource::ModelMarker)))
        .or_else(|| {
            model
                .as_deref()
                .and_then(|m| cat.provider_cache(provider, m))
                .map(|t| (t, ContextSource::ProviderCache))
        })
        .or_else(|| {
            model
                .as_deref()
                .and_then(|m| cat.table(provider, m))
                .map(|t| (t, ContextSource::ModelTable))
        });

    let observed = ev.observed_floor();
    match resolved {
        Some((tokens, source)) if observed <= tokens => Some(ContextWindow {
            tokens,
            source,
            model,
            overruled: None,
        }),
        Some((_, source)) => Some(ContextWindow {
            tokens: tier_at_least(observed),
            source: ContextSource::Observed,
            model,
            overruled: Some(source),
        }),
        None if observed > 0 => Some(ContextWindow {
            tokens: tier_at_least(observed),
            source: ContextSource::Observed,
            model,
            overruled: None,
        }),
        None => None,
    }
}

/// Context occupancy of the evidence against a resolved window, in percent (not clamped).
pub fn usage_percent(ev: &ContextEvidence, window: &ContextWindow) -> f64 {
    window.usage_ratio(ev.last_context_tokens) * 100.0
}

#[cfg(test)]
mod tests;
