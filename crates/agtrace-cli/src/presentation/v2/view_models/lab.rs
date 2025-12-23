use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LabExportViewModel {
    pub exported_count: usize,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCallSample {
    pub arguments: String,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolClassification {
    pub tool_name: String,
    pub origin: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolStatsEntry {
    pub tool_name: String,
    pub count: usize,
    pub sample: Option<ToolCallSample>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStats {
    pub provider_name: String,
    pub tools: Vec<ToolStatsEntry>,
    pub classifications: Vec<ToolClassification>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabStatsViewModel {
    pub total_sessions: usize,
    pub providers: Vec<ProviderStats>,
}
