use serde::{Deserialize, Serialize};

/// Git repository context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dirty: Option<bool>,
}

/// Execution environment context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
}

/// Agent control policy and constraints
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<String>,
}
