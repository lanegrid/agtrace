use serde::{Deserialize, Serialize};

use super::common::DetailLevel;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSessionsArgs {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub cursor: Option<String>,
    pub provider: Option<String>,
    pub project_hash: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetSessionDetailsArgs {
    pub session_id: String,
    #[serde(default)]
    pub detail_level: Option<DetailLevel>,
    #[serde(default)]
    pub include_reasoning: Option<bool>,
}

impl GetSessionDetailsArgs {
    pub fn detail_level(&self) -> DetailLevel {
        self.detail_level.unwrap_or_default()
    }

    pub fn include_reasoning(&self) -> bool {
        self.include_reasoning.unwrap_or(false)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeSessionArgs {
    pub session_id: String,
    #[serde(default)]
    pub include_failures: Option<bool>,
    #[serde(default)]
    pub include_loops: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchEventsArgs {
    pub pattern: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub cursor: Option<String>,
    pub provider: Option<String>,
    pub event_type: Option<String>,
    #[serde(default)]
    pub include_full_payload: Option<bool>,
}
