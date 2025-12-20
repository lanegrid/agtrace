use chrono::Duration;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ProviderConfigSummary {
    pub name: String,
    pub enabled: bool,
    pub log_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ProviderSetResult {
    pub provider: String,
    pub enabled: bool,
    pub log_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ProjectSummary {
    pub hash: String,
    pub root_path: Option<String>,
    pub session_count: usize,
    pub last_scanned: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CorpusStats {
    pub sample_size: usize,
    pub total_tool_calls: usize,
    pub total_failures: usize,
    pub max_duration_ms: i64,
}

#[derive(Debug, Clone)]
pub struct GuidanceContext {
    pub config_exists: bool,
    pub db_exists: bool,
    pub session_count: usize,
}

#[derive(Debug, Clone)]
pub struct RawFileContent {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct InspectLine {
    pub number: usize,
    pub content: InspectContent,
}

#[derive(Debug, Clone)]
pub enum InspectContent {
    Raw(String),
    Json(Value),
}

#[derive(Debug, Clone)]
pub struct InspectDisplay {
    pub file_path: String,
    pub total_lines: usize,
    pub shown_lines: usize,
    pub lines: Vec<InspectLine>,
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub default_log_path: String,
}

#[derive(Debug, Clone)]
pub enum Step1Result {
    DetectedProviders {
        providers: HashMap<String, PathBuf>,
        config_saved: bool,
    },
    LoadedConfig {
        config_path: PathBuf,
    },
    NoProvidersDetected {
        available_providers: Vec<ProviderInfo>,
    },
}

#[derive(Debug, Clone)]
pub enum Step3Result {
    Scanned {
        success: bool,
        error: Option<String>,
    },
    Skipped {
        reason: SkipReason,
    },
}

#[derive(Debug, Clone)]
pub enum SkipReason {
    RecentlyScanned { elapsed: Duration },
}

#[derive(Debug, Clone)]
pub enum InitRenderEvent {
    Header,
    Step1Detecting,
    Step1Loading,
    Step1Result(Step1Result),
    Step2Header,
    Step2Result { db_path: PathBuf },
    Step3Header,
    Step3Result(Step3Result),
    Step4Header,
    Step4NoSessions { all_projects: bool },
    NextSteps { session_id: String },
}

#[derive(Debug, Clone)]
pub enum IndexEvent {
    IncrementalHint {
        indexed_files: usize,
    },
    LogRootMissing {
        provider_name: String,
        log_root: PathBuf,
    },
    ProviderScanning {
        provider_name: String,
    },
    ProviderSessionCount {
        provider_name: String,
        count: usize,
        project_hash: String,
        all_projects: bool,
    },
    SessionRegistered {
        session_id: String,
    },
    Completed {
        total_sessions: usize,
        scanned_files: usize,
        skipped_files: usize,
        verbose: bool,
    },
}

#[derive(Debug, Clone)]
pub enum WatchStart {
    Provider { name: String, log_root: PathBuf },
    Session { id: String, log_root: PathBuf },
}

#[derive(Debug, Clone)]
pub struct WatchTokenUsage {
    pub total_tokens: u64,
    pub limit: Option<u64>,
    pub input_pct: Option<f64>,
    pub output_pct: Option<f64>,
    pub total_pct: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct WatchSummary {
    pub recent_lines: Vec<String>,
    pub token_usage: Option<WatchTokenUsage>,
    pub turn_count: usize,
}

#[derive(Debug, Clone)]
pub struct ToolCallSampleViewModel {
    pub arguments: String,
    pub result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolStatsEntry {
    pub tool_name: String,
    pub count: usize,
    pub sample: Option<ToolCallSampleViewModel>,
}

#[derive(Debug, Clone)]
pub struct ToolClassificationViewModel {
    pub tool_name: String,
    pub origin: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderStatsViewModel {
    pub provider_name: String,
    pub tools: Vec<ToolStatsEntry>,
    pub classifications: Vec<ToolClassificationViewModel>,
}

#[derive(Debug, Clone)]
pub struct LabStatsViewModel {
    pub total_sessions: usize,
    pub providers: Vec<ProviderStatsViewModel>,
}
