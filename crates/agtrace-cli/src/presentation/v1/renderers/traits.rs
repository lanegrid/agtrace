use crate::presentation::v1::view_models::{ReactionViewModel, WatchStart, WatchSummary};
use crate::presentation::v2::view_models::{EventViewModel, StreamStateViewModel};
use anyhow::Result;
use std::path::Path;

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
        turns: Option<&[crate::presentation::v1::view_models::TurnUsageViewModel]>,
    ) -> Result<()>;
}
