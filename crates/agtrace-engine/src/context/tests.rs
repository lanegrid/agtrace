//! Context resolver table tests (design §7 layer 5).

use super::*;
use agtrace_types::{
    AgentId, CompactionPayload, CompactionTrigger, EventOrigin, ModelChangePayload,
    ModelChangeSource, TokenInput, TokenOutput, TokenUsagePayload,
};
use chrono::Utc;
use uuid::Uuid;

/// Catalog with fixed answers per layer.
#[derive(Default)]
struct Cat {
    user: Option<u64>,
    cache: Option<u64>,
    table: Option<u64>,
}

impl ModelCatalog for Cat {
    fn user_override(&self, _: Provider, _: Option<&str>) -> Option<u64> {
        self.user
    }
    fn provider_cache(&self, _: Provider, _: &str) -> Option<u64> {
        self.cache
    }
    fn table(&self, _: Provider, _: &str) -> Option<u64> {
        self.table
    }
}

fn ev(payload: EventPayload) -> AgentEvent {
    AgentEvent {
        id: Uuid::nil(),
        session_id: Uuid::nil(),
        agent: AgentId::claude_session("s"),
        parent_id: None,
        timestamp: Utc::now(),
        origin: EventOrigin::default(),
        payload,
    }
}

fn usage(model: Option<&str>, uncached: u64, cache_read: u64, cache_write: u64) -> AgentEvent {
    ev(EventPayload::TokenUsage(
        TokenUsagePayload::new(
            TokenInput::new(uncached, cache_read, cache_write),
            TokenOutput::new(500, 0, 0),
        )
        .with_model(model.map(str::to_string)),
    ))
}

fn model_change(to: &str) -> AgentEvent {
    ev(EventPayload::ModelChange(ModelChangePayload {
        from: None,
        to: to.to_string(),
        source: ModelChangeSource::AssistantMessage,
    }))
}

fn explicit(tokens: u64) -> AgentEvent {
    ev(EventPayload::ContextWindowHint(
        ContextWindowHintPayload::Explicit {
            tokens,
            model: Some("gpt-5.6-sol".into()),
        },
    ))
}

fn marker(model: &str) -> AgentEvent {
    ev(EventPayload::ContextWindowHint(
        ContextWindowHintPayload::ExtendedMarker {
            model: model.to_string(),
            tokens: 1_000_000,
            evidence: "test".into(),
        },
    ))
}

fn compaction(pre: Option<u64>, post: Option<u64>) -> AgentEvent {
    ev(EventPayload::Compaction(CompactionPayload {
        trigger: CompactionTrigger::Auto,
        pre_tokens: pre,
        post_tokens: post,
        window_number: None,
        duration_ms: None,
    }))
}

fn fold(events: &[AgentEvent]) -> ContextEvidence {
    let mut e = ContextEvidence::new();
    for event in events {
        e.apply(event);
    }
    e
}

#[test]
fn nothing_known_resolves_to_none() {
    assert_eq!(
        resolve(
            Provider::ClaudeCode,
            &ContextEvidence::new(),
            &Cat::default()
        ),
        None
    );
}

#[test]
fn user_config_wins_and_skips_floor() {
    let e = fold(&[explicit(258_400), usage(Some("gpt-5.6-sol"), 300_000, 0, 0)]);
    let cat = Cat {
        user: Some(100_000),
        cache: Some(1),
        table: Some(2),
    };
    let w = resolve(Provider::Codex, &e, &cat).unwrap();
    assert_eq!(w.tokens, 100_000);
    assert_eq!(w.source, ContextSource::UserConfig);
    assert_eq!(w.provenance(), "cfg");
}

#[test]
fn log_beats_marker_cache_and_table() {
    let e = fold(&[explicit(258_400)]);
    let cat = Cat {
        user: None,
        cache: Some(272_000),
        table: Some(400_000),
    };
    let w = resolve(Provider::Codex, &e, &cat).unwrap();
    assert_eq!((w.tokens, w.source), (258_400, ContextSource::Log));
    assert_eq!(w.model.as_deref(), Some("gpt-5.6-sol"));
}

#[test]
fn latest_explicit_hint_wins() {
    let e = fold(&[explicit(258_400), explicit(128_000)]);
    assert_eq!(e.explicit, Some(128_000));
}

#[test]
fn marker_beats_cache_and_table() {
    let e = fold(&[
        marker("claude-opus-5-5[1m]"),
        usage(Some("claude-opus-5-5"), 10, 20_000, 30),
    ]);
    let cat = Cat {
        table: Some(200_000),
        ..Cat::default()
    };
    let w = resolve(Provider::ClaudeCode, &e, &cat).unwrap();
    assert_eq!(
        (w.tokens, w.source),
        (1_000_000, ContextSource::ModelMarker)
    );
    assert_eq!(w.provenance(), "1m");
}

#[test]
fn marker_for_other_family_does_not_apply() {
    // cost-state lists opus-5-5[1m] and haiku; the agent runs on haiku.
    let e = fold(&[
        marker("claude-opus-5-5[1m]"),
        usage(Some("claude-haiku-4-5-20251001"), 10, 1000, 0),
    ]);
    assert_eq!(e.marker, None);
    let cat = Cat {
        table: Some(200_000),
        ..Cat::default()
    };
    let w = resolve(Provider::ClaudeCode, &e, &cat).unwrap();
    assert_eq!((w.tokens, w.source), (200_000, ContextSource::ModelTable));
}

#[test]
fn marker_does_not_leak_to_similar_prefix_family() {
    // "claude-opus-5[1m]" must not apply to "claude-opus-5-5".
    let e = fold(&[marker("claude-opus-5[1m]"), model_change("claude-opus-5-5")]);
    assert_eq!(e.marker, None);
}

#[test]
fn marker_follows_model_switch() {
    let mut e = fold(&[
        model_change("claude-opus-5-5"),
        marker("claude-opus-5-5[1m]"),
    ]);
    assert_eq!(e.marker, Some(1_000_000));
    e.apply(&model_change("claude-haiku-4-5"));
    assert_eq!(e.marker, None);
    e.apply(&model_change("claude-opus-5-5"));
    assert_eq!(e.marker, Some(1_000_000));
}

#[test]
fn marketing_name_marker_binds_to_current_or_next_model() {
    // Marker before any model: binds to the first model seen.
    let e = fold(&[
        marker("Opus 5.5 (1M context)"),
        model_change("claude-opus-5-5"),
    ]);
    assert_eq!(e.marker, Some(1_000_000));

    // Marker after a model: binds to that model only.
    let mut e = fold(&[
        model_change("claude-opus-5-5"),
        marker("Opus 5.5 (1M context)"),
    ]);
    assert_eq!(e.marker, Some(1_000_000));
    e.apply(&model_change("claude-haiku-4-5"));
    assert_eq!(e.marker, None);
}

#[test]
fn model_with_1m_suffix_is_a_marker() {
    let e = fold(&[model_change("claude-opus-5-5[1m]")]);
    let w = resolve(Provider::ClaudeCode, &e, &Cat::default()).unwrap();
    assert_eq!(
        (w.tokens, w.source),
        (1_000_000, ContextSource::ModelMarker)
    );
}

#[test]
fn external_marker_from_side_state() {
    let mut e = fold(&[usage(Some("claude-opus-5-5"), 1, 1, 1)]);
    e.external_marker = Some(1_000_000);
    let cat = Cat {
        table: Some(200_000),
        ..Cat::default()
    };
    let w = resolve(Provider::ClaudeCode, &e, &cat).unwrap();
    assert_eq!(
        (w.tokens, w.source),
        (1_000_000, ContextSource::ModelMarker)
    );
}

#[test]
fn provider_cache_beats_table() {
    let e = fold(&[usage(Some("gpt-6-astra"), 1000, 0, 0)]);
    let cat = Cat {
        cache: Some(258_400),
        table: Some(400_000),
        ..Cat::default()
    };
    let w = resolve(Provider::Codex, &e, &cat).unwrap();
    assert_eq!(
        (w.tokens, w.source),
        (258_400, ContextSource::ProviderCache)
    );
    assert_eq!(w.provenance(), "cache");
}

#[test]
fn table_used_when_nothing_else() {
    let e = fold(&[usage(Some("claude-opus-5"), 1000, 0, 0)]);
    let cat = Cat {
        table: Some(1_000_000),
        ..Cat::default()
    };
    let w = resolve(Provider::ClaudeCode, &e, &cat).unwrap();
    assert_eq!((w.tokens, w.source), (1_000_000, ContextSource::ModelTable));
}

#[test]
fn floor_bumps_table_to_tier_when_usage_exceeds_it() {
    // Old 200k table entry, but the agent was observed at 387k context.
    let e = fold(&[usage(Some("claude-opus-5"), 7_000, 380_000, 0)]);
    let cat = Cat {
        table: Some(200_000),
        ..Cat::default()
    };
    let w = resolve(Provider::ClaudeCode, &e, &cat).unwrap();
    assert_eq!(w.tokens, 400_000);
    assert_eq!(w.source, ContextSource::Observed);
    assert_eq!(w.overruled, Some(ContextSource::ModelTable));
    assert_eq!(w.provenance(), "obs");
}

#[test]
fn floor_uses_compaction_pre_tokens() {
    let e = fold(&[
        usage(Some("claude-fable-5"), 1_000, 100_000, 0),
        compaction(Some(973_967), Some(50_000)),
    ]);
    let cat = Cat {
        table: Some(200_000),
        ..Cat::default()
    };
    let w = resolve(Provider::ClaudeCode, &e, &cat).unwrap();
    assert_eq!((w.tokens, w.source), (1_000_000, ContextSource::Observed));
}

#[test]
fn floor_applies_to_log_layer_too() {
    let e = fold(&[explicit(258_400), usage(None, 300_000, 0, 0)]);
    let w = resolve(Provider::Codex, &e, &Cat::default()).unwrap();
    assert_eq!(w.tokens, 400_000);
    assert_eq!(w.overruled, Some(ContextSource::Log));
}

#[test]
fn observed_only_is_marked_as_guess() {
    let e = fold(&[usage(Some("unknown-model"), 150_000, 0, 0)]);
    let w = resolve(Provider::ClaudeCode, &e, &Cat::default()).unwrap();
    assert_eq!((w.tokens, w.source), (200_000, ContextSource::Observed));
    assert_eq!(w.provenance(), "obs?");
}

#[test]
fn observed_above_all_tiers_uses_observed_value() {
    assert_eq!(tier_at_least(1_500_000), 1_500_000);
    assert_eq!(tier_at_least(258_401), 272_000);
    assert_eq!(tier_at_least(1), 200_000);
}

#[test]
fn context_tokens_exclude_output_and_include_cache_write() {
    let e = fold(&[usage(Some("m"), 10, 20, 30)]);
    assert_eq!(e.last_context_tokens, 60);
    assert_eq!(e.peak_context_tokens, 60);
}

#[test]
fn compaction_resets_occupancy_to_post_tokens_when_known() {
    let mut e = fold(&[usage(Some("m"), 0, 500_000, 0)]);
    e.apply(&compaction(Some(500_000), None));
    assert_eq!(
        e.last_context_tokens, 500_000,
        "unchanged without post_tokens"
    );
    e.apply(&compaction(Some(500_000), Some(40_000)));
    assert_eq!(e.last_context_tokens, 40_000);
    assert_eq!(e.peak_context_tokens, 500_000, "peak is not reset");
    e.apply(&usage(Some("m"), 0, 45_000, 0));
    assert_eq!(e.last_context_tokens, 45_000);
}

#[test]
fn usage_percent_has_no_compaction_buffer() {
    let e = fold(&[usage(Some("m"), 0, 100_000, 0)]);
    let w = ContextWindow {
        tokens: 200_000,
        source: ContextSource::ModelTable,
        model: None,
        overruled: None,
    };
    assert!((usage_percent(&e, &w) - 50.0).abs() < f64::EPSILON);
}

#[test]
fn latest_model_wins() {
    let e = fold(&[
        usage(Some("claude-opus-5"), 1, 1, 1),
        model_change("claude-opus-5-5"),
    ]);
    assert_eq!(e.model.as_deref(), Some("claude-opus-5-5"));
}
