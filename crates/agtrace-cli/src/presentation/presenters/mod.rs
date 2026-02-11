pub mod doctor;
pub mod index;
pub mod init;
pub mod lab;
pub mod pack;
pub mod project;
pub mod provider;
pub mod session;
pub mod watch;
pub mod watch_tui;
pub mod worktree;

pub use doctor::{present_check_result, present_diagnose_results, present_inspect_result};
pub use index::{present_index_info, present_index_result, present_vacuum_result};
pub use init::present_init_result;
pub use lab::{
    present_event, present_events, present_lab_export, present_lab_grep, present_lab_stats,
};
pub use pack::present_pack_report;
pub use project::present_project_list;
pub use provider::{present_provider_detected, present_provider_list, present_provider_set};
pub use session::{present_session_analysis, present_session_list, present_session_state};
pub use watch::{
    present_watch_attached, present_watch_error, present_watch_rotated,
    present_watch_start_provider, present_watch_start_session, present_watch_stream_update,
    present_watch_waiting,
};
pub use watch_tui::build_screen_view_model;
pub use worktree::{present_worktree_list, present_worktree_sessions};
