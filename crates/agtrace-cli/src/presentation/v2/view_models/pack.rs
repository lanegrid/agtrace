use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportTemplate {
    Compact,
    Diagnose,
    Tools,
}

impl std::str::FromStr for ReportTemplate {
    type Err = std::convert::Infallible;

    fn from_str(template: &str) -> Result<Self, Self::Err> {
        Ok(match template {
            "diagnose" => Self::Diagnose,
            "tools" => Self::Tools,
            _ => Self::Compact,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionDigest {
    pub session_id: String,
    pub source: String,
    pub opening: Option<String>,
    pub activation: Option<String>,
    pub tool_calls_total: usize,
    pub tool_failures_total: usize,
    pub max_e2e_ms: u64,
    pub max_tool_ms: u64,
    pub missing_tool_pairs: usize,
    pub loop_signals: usize,
    pub longest_chain: usize,
    pub recency_boost: u32,
    pub selection_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackReportViewModel {
    pub template: ReportTemplate,
    pub pool_size: usize,
    pub candidate_count: usize,
    pub digests: Vec<SessionDigest>,
}
