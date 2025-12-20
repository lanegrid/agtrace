use crate::presentation::view_models::{EventViewModel, StreamStateViewModel, WatchStart};
use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum TuiEvent {
    #[allow(dead_code)]
    Input(KeyEvent),
    #[allow(dead_code)]
    Tick,
    WatchStart(WatchStart),
    WatchAttached(String),
    WatchRotated(String, String),
    WatchWaiting(String),
    WatchError(String, bool),
    StreamUpdate(StreamStateViewModel, Vec<EventViewModel>),
}
