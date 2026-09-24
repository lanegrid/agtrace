//! Legacy single-session streaming used by the old `watch` TUI / console.
//! The workspace watcher (`crate::workspace`) replaces it.

pub mod events;
pub mod streamer;

pub use events::{DiscoveryEvent, StreamEvent, WatchEvent};
pub use streamer::SessionStreamer;
