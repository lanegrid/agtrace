use crate::Result;
use crate::storage::{LoadOptions, SessionRepository};
use agtrace_engine::assemble_session;
use agtrace_index::Database;
use agtrace_providers::create_adapter;
use agtrace_types::EventPayload;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone)]
pub struct CorpusStats {
    pub sample_size: usize,
    pub total_tool_calls: usize,
    pub total_failures: usize,
    pub max_duration_ms: i64,
}

pub fn get_corpus_overview(
    db: &Database,
    project_hash: Option<&agtrace_types::ProjectHash>,
    limit: usize,
) -> Result<CorpusStats> {
    let raw_sessions = db.list_sessions(project_hash, Some(limit))?;

    let loader = SessionRepository::new(db);
    let options = LoadOptions::default();

    let mut total_tool_calls = 0;
    let mut total_failures = 0;
    let mut max_duration = 0i64;

    for session in &raw_sessions {
        if let Ok(events) = loader.load_events(&session.id, &options)
            && let Some(agent_session) = assemble_session(&events)
        {
            for turn in &agent_session.turns {
                for step in &turn.steps {
                    total_tool_calls += step.tools.len();
                    for tool_exec in &step.tools {
                        if tool_exec.is_error {
                            total_failures += 1;
                        }
                    }
                }
                if turn.stats.duration_ms > max_duration {
                    max_duration = turn.stats.duration_ms;
                }
            }
        }
    }

    Ok(CorpusStats {
        sample_size: raw_sessions.len(),
        total_tool_calls,
        total_failures,
        max_duration_ms: max_duration,
    })
}

#[derive(Debug, Clone)]
pub struct ToolSample {
    pub arguments: String,
    pub result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub tool_name: String,
    pub origin: Option<String>,
    pub kind: Option<String>,
}

pub type ProviderStats =
    BTreeMap<String, (BTreeMap<String, (usize, Option<ToolSample>)>, Vec<ToolInfo>)>;

pub struct StatsResult {
    pub total_sessions: usize,
    pub provider_stats: ProviderStats,
}

pub fn collect_tool_stats(
    db: &Database,
    limit: Option<usize>,
    provider: Option<String>,
) -> Result<StatsResult> {
    let sessions = db.list_sessions(None, limit)?;
    let sessions: Vec<_> = sessions
        .into_iter()
        .filter(|s| {
            if let Some(ref src) = provider {
                &s.provider == src
            } else {
                true
            }
        })
        .collect();

    let total_sessions = sessions.len();

    let loader = SessionRepository::new(db);
    let options = LoadOptions::default();

    let mut stats: HashMap<String, HashMap<String, (usize, Option<ToolSample>)>> = HashMap::new();

    for session in &sessions {
        let events = match loader.load_events(&session.id, &options) {
            Ok(events) => events,
            Err(_) => continue,
        };

        let mut tool_results = HashMap::new();
        for event in &events {
            if let EventPayload::ToolResult(result) = &event.payload {
                tool_results.insert(result.tool_call_id, result.output.clone());
            }
        }

        let provider = &session.provider;
        for event in events {
            if let EventPayload::ToolCall(tool_call) = &event.payload {
                let provider_stats = stats.entry(provider.clone()).or_default();
                let tool_entry = provider_stats
                    .entry(tool_call.name().to_string())
                    .or_insert((0, None));

                tool_entry.0 += 1;

                if tool_entry.1.is_none() {
                    let result = tool_results.get(&event.id).cloned();
                    // Serialize entire tool_call to extract arguments field
                    let arguments = serde_json::to_value(tool_call)
                        .ok()
                        .and_then(|v| v.get("arguments").cloned())
                        .and_then(|v| serde_json::to_string(&v).ok())
                        .unwrap_or_else(|| String::from("(failed to serialize)"));
                    tool_entry.1 = Some(ToolSample { arguments, result });
                }
            }
        }
    }

    let provider_stats: ProviderStats = stats
        .into_iter()
        .map(|(provider_name, tools)| {
            let sorted_tools: BTreeMap<_, _> = tools.into_iter().collect();

            let classifications: Vec<ToolInfo> = sorted_tools
                .keys()
                .map(|tool_name| {
                    let (origin, kind) = if let Ok(adapter) = create_adapter(&provider_name) {
                        let (o, k) = adapter.mapper.classify(tool_name);
                        (Some(format!("{:?}", o)), Some(format!("{:?}", k)))
                    } else {
                        (None, None)
                    };

                    ToolInfo {
                        tool_name: tool_name.clone(),
                        origin,
                        kind,
                    }
                })
                .collect();

            (provider_name, (sorted_tools, classifications))
        })
        .collect();

    Ok(StatsResult {
        total_sessions,
        provider_stats,
    })
}
