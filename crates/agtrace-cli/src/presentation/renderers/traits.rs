use crate::presentation::view_models::{
    CorpusStats, DiagnoseResultViewModel, DisplayOptions, DoctorCheckResultViewModel,
    EventViewModel, GuidanceContext, IndexEvent, InitRenderEvent, InspectDisplay,
    LabStatsViewModel, ProjectSummary, ProviderConfigSummary, ProviderSetResult, RawFileContent,
    ReactionViewModel, SessionDigestViewModel, SessionListEntryViewModel, SessionViewModel,
    StreamStateViewModel, WatchStart, WatchSummary,
};
use crate::presentation::views::ReportTemplate;
use crate::types::OutputFormat;
use anyhow::Result;
use std::path::Path;

pub trait TraceView:
    SystemView + SessionView + DiagnosticView + WatchView + Send + Sync + 'static
{
}

impl<T> TraceView for T where
    T: SystemView + SessionView + DiagnosticView + WatchView + Send + Sync + 'static
{
}

pub trait SystemView {
    fn render_guidance(&self, context: &GuidanceContext) -> Result<()>;
    fn render_provider_list(&self, providers: &[ProviderConfigSummary]) -> Result<()>;
    fn render_provider_detected(&self, providers: &[ProviderConfigSummary]) -> Result<()>;
    fn render_provider_set(&self, result: &ProviderSetResult) -> Result<()>;
    fn render_warning(&self, message: &str) -> Result<()>;
    fn render_project_list(
        &self,
        current_root: &str,
        current_hash: &str,
        projects: &[ProjectSummary],
    ) -> Result<()>;
    fn render_corpus_overview(&self, stats: &CorpusStats) -> Result<()>;
    fn render_index_event(&self, event: IndexEvent) -> Result<()>;
    fn render_init_event(&self, event: InitRenderEvent) -> Result<()>;
    fn render_lab_export(&self, exported: usize, output_path: &Path) -> Result<()>;
    fn render_lab_stats(&self, stats: &LabStatsViewModel) -> Result<()>;
}

pub trait SessionView {
    fn render_session_list(
        &self,
        sessions: &[SessionListEntryViewModel],
        format: OutputFormat,
    ) -> Result<()>;
    fn render_session_raw_files(&self, files: &[RawFileContent]) -> Result<()>;
    fn render_session_events_json(&self, events: &[EventViewModel]) -> Result<()>;
    fn render_session_compact(
        &self,
        session: &SessionViewModel,
        options: &DisplayOptions,
    ) -> Result<()>;
    fn render_session_timeline(
        &self,
        events: &[EventViewModel],
        truncate: bool,
        enable_color: bool,
    ) -> Result<()>;
    fn render_session_assemble_error(&self) -> Result<()>;
    fn render_pack_report(
        &self,
        digests: &[SessionDigestViewModel],
        template: ReportTemplate,
        pool_size: usize,
        candidate_count: usize,
    ) -> Result<()>;
}

pub trait DiagnosticView {
    fn render_doctor_check(&self, result: &DoctorCheckResultViewModel) -> Result<()>;
    fn render_diagnose_results(
        &self,
        results: &[DiagnoseResultViewModel],
        verbose: bool,
    ) -> Result<()>;
    fn render_inspect(&self, display: &InspectDisplay) -> Result<()>;
}

pub trait WatchView {
    fn render_watch_start(&self, start: &WatchStart) -> Result<()>;
    fn on_watch_attached(&self, display_name: &str) -> Result<()>;
    fn on_watch_initial_summary(&self, summary: &WatchSummary) -> Result<()>;
    fn on_watch_rotated(&self, old_path: &Path, new_path: &Path) -> Result<()>;
    fn on_watch_waiting(&self, message: &str) -> Result<()>;
    fn on_watch_error(&self, message: &str, fatal: bool) -> Result<()>;
    fn on_watch_orphaned(&self, orphaned: usize, total_events: usize) -> Result<()>;
    fn on_watch_token_warning(&self, warning: &str) -> Result<()>;
    fn on_watch_reactor_error(&self, reactor_name: &str, error: &str) -> Result<()>;
    fn on_watch_reaction_error(&self, error: &str) -> Result<()>;
    fn on_watch_reaction(&self, reaction: &ReactionViewModel) -> Result<()>;
    fn render_stream_update(
        &self,
        state: &StreamStateViewModel,
        new_events: &[EventViewModel],
    ) -> Result<()>;
}
