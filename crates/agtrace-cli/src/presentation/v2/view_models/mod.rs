pub mod common;
pub mod doctor;
pub mod index;
pub mod init;
pub mod lab;
pub mod pack;
pub mod project;
pub mod provider;
pub mod result;
pub mod session;

use std::fmt::Display;

pub use common::{Guidance, OutputFormat, StatusBadge, StatusLevel, ViewMode};

/// Core trait that bridges Data (ViewModel) and Display (View).
///
/// ViewModels implement this trait to provide different visual representations
/// based on the requested `ViewMode`.
///
/// ## The Golden Rule: Return `Box<dyn Display>`
/// The trait returns a boxed `Display` trait object that encapsulates
/// the rendering logic for the requested mode. This allows:
/// - The ViewModel to remain a pure data container
/// - The View to handle all formatting logic
/// - Late binding of the display implementation
pub trait CreateView {
    /// Creates a display-ready view for the given mode.
    ///
    /// # Example
    /// ```ignore
    /// let view = session_vm.create_view(ViewMode::Compact);
    /// println!("{}", view);
    /// ```
    fn create_view<'a>(&'a self, mode: ViewMode) -> Box<dyn Display + 'a>;
}
pub use doctor::{
    CheckStatus, DiagnoseResultViewModel, DiagnoseResultsViewModel, DoctorCheckResultViewModel,
    FailureExample, InspectLine, InspectResultViewModel,
};
pub use index::{IndexEvent, IndexMode, IndexResultViewModel, VacuumResultViewModel};
pub use init::{ConfigStatus, InitProgress, InitResultViewModel, ProviderInfo, ScanOutcome};
pub use lab::{
    EventPayloadViewModel, EventViewModel, LabExportViewModel, LabGrepViewModel,
    LabStatsViewModel, ProviderStats, ToolCallSample, ToolClassification, ToolStatsEntry,
};
pub use pack::{PackReportViewModel, ReportTemplate, SessionDigest};
pub use project::{ProjectEntryViewModel, ProjectListViewModel};
pub use provider::{
    ProviderDetectedViewModel, ProviderEntry, ProviderListViewModel, ProviderSetViewModel,
};
pub use result::CommandResultViewModel;
pub use session::{
    AgentStepViewModel, ContextUsage, ContextWindowSummary, FilterSummary,
    SessionAnalysisViewModel, SessionHeader, SessionListEntry, SessionListViewModel,
    TurnAnalysisViewModel, TurnMetrics,
};
