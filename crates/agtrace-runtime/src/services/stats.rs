use agtrace_index::Database;
use agtrace_providers::create_provider;
use agtrace_types::EventPayload;
use anyhow::Result;
use std::collections::{BTreeMap, HashMap};

use crate::session_repository::{LoadOptions, SessionRepository};

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

pub struct StatsService<'a> {
    db: &'a Database,
}

impl<'a> StatsService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn collect_tool_stats(
        &self,
        limit: Option<usize>,
        source: Option<String>,
    ) -> Result<StatsResult> {
        let sessions = self.db.list_sessions(None, limit.unwrap_or(100000))?;
        let sessions: Vec<_> = sessions
            .into_iter()
            .filter(|s| {
                if let Some(ref src) = source {
                    &s.provider == src
                } else {
                    true
                }
            })
            .collect();

        let total_sessions = sessions.len();

        let loader = SessionRepository::new(self.db);
        let options = LoadOptions::default();

        let mut stats: HashMap<String, HashMap<String, (usize, Option<ToolSample>)>> =
            HashMap::new();

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
                        .entry(tool_call.name.clone())
                        .or_insert((0, None));

                    tool_entry.0 += 1;

                    if tool_entry.1.is_none() {
                        let result = tool_results.get(&event.id).cloned();
                        let arguments = serde_json::to_string(&tool_call.arguments)
                            .unwrap_or_else(|_| String::from("(failed to serialize)"));
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
                        let (origin, kind) = if let Ok(provider) = create_provider(&provider_name) {
                            provider
                                .classify_tool(tool_name)
                                .map(|(o, k)| (Some(format!("{:?}", o)), Some(format!("{:?}", k))))
                                .unwrap_or((None, None))
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
}
