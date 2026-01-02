mod analysis;
mod event_details;
mod event_previews;
mod list_sessions;
mod project;
mod session_full;
mod session_summary;
mod session_turns;
mod turn_steps;

// Re-export request types (Args)
pub use analysis::AnalyzeSessionArgs;
pub use event_details::GetEventDetailsArgs;
pub use event_previews::SearchEventPreviewsArgs;
pub use list_sessions::ListSessionsArgs;
pub use session_full::GetSessionFullArgs;
pub use session_summary::GetSessionSummaryArgs;
pub use session_turns::GetSessionTurnsArgs;
pub use turn_steps::GetTurnStepsArgs;

// Re-export response types (ViewModels)
pub use analysis::AnalysisViewModel;
pub use event_details::EventDetailsViewModel;
pub use event_previews::{EventPreviewViewModel, SearchEventPreviewsViewModel};
pub use list_sessions::ListSessionsViewModel;
pub use project::ProjectInfoViewModel;
pub use session_full::SessionFullViewModel;
pub use session_summary::SessionSummaryViewModel;
pub use session_turns::SessionTurnsViewModel;
pub use turn_steps::TurnStepsViewModel;

// Internal types (used within ViewModels, but not exported to higher levels)
#[allow(unused_imports)]
pub(crate) use event_previews::PreviewContent;
#[allow(unused_imports)]
pub(crate) use list_sessions::SessionSummaryDto;
#[allow(unused_imports)]
pub(crate) use session_turns::TurnWithIndex;
