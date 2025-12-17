// Analysis module - Session metrics, selection lenses, and diagnostics
// Pure business logic for analyzing agent sessions

pub mod digest;
pub mod lenses;
pub mod metrics;
pub mod packing;

pub use digest::SessionDigest;
pub use packing::analyze_and_select_sessions;
