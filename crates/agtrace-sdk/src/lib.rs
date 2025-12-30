//! agtrace-sdk: The Observability Platform for AI Agents.
//!
//! # Overview
//!
//! `agtrace-sdk` provides a high-level, stable API for building observability
//! tools on top of agtrace. It abstracts away the internal complexity of
//! providers, indexing, and runtime orchestration, exposing only the essential
//! primitives for monitoring and analyzing AI agent behavior.
//!
//! # Architecture
//!
//! This SDK acts as a facade over:
//! - `agtrace-types`: Core domain models (AgentEvent, etc.)
//! - `agtrace-providers`: Multi-provider log normalization
//! - `agtrace-engine`: Session analysis and diagnostics
//! - `agtrace-index`: Metadata storage and querying
//! - `agtrace-runtime`: Internal orchestration layer
//!
//! # Usage
//!
//! ```no_run
//! use agtrace_sdk::{Client, Lens};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Connect to the workspace
//! let client = Client::connect("~/.agtrace")?;
//!
//! // 2. Watch for live events (Real-time monitoring)
//! let stream = client.watch().all_providers().start()?;
//! if let Some(event) = stream.next_blocking() {
//!     println!("New event: {:?}", event);
//! }
//!
//! // 3. Analyze a specific session (Diagnosis)
//! let events = client.session("session_id_123").events()?;
//! if let Some(session) = agtrace_sdk::assemble_session(&events) {
//!     let report = agtrace_sdk::analyze_session(session)
//!         .through(Lens::Failures)
//!         .through(Lens::Loops)
//!         .report()?;
//!
//!     println!("Health score: {}", report.score);
//!     for insight in &report.insights {
//!         println!("  - {}", insight);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod analysis;
pub mod client;
pub mod error;
pub mod watch;

// Re-export core domain types for convenience
pub use agtrace_engine::session::summarize;
pub use agtrace_engine::{AgentSession, SessionSummary, assemble_session};
pub use agtrace_types::event::AgentEvent;
pub use agtrace_types::tool::ToolKind;

// Public facade
pub use analysis::{AnalysisReport, Lens};
pub use client::{Client, SessionHandle};
pub use error::{Error, Result};
pub use watch::{LiveStream, WatchBuilder};

// Helper function for analysis
pub fn analyze_session(session: AgentSession) -> analysis::SessionAnalyzer {
    analysis::SessionAnalyzer::new(session)
}
