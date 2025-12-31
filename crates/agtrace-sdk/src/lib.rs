#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
//
// The README.md above is included as documentation and tested by `cargo test --doc`.
// Below is additional API documentation that only appears in rustdoc.

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
//! ## Client-based API (Recommended)
//!
//! ```no_run
//! use agtrace_sdk::{Client, Lens};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Connect to the workspace
//! let client = Client::connect("~/.agtrace").await?;
//!
//! // 2. Watch for live events (Real-time monitoring)
//! let stream = client.watch().all_providers().start()?;
//! for event in stream.take(10) {
//!     println!("New event: {:?}", event);
//! }
//!
//! // 3. Analyze a specific session (Diagnosis)
//! let handle = client.sessions().get("session_id_123")?;
//! let report = handle.analyze()?
//!     .through(Lens::Failures)
//!     .through(Lens::Loops)
//!     .report()?;
//!
//! println!("Health score: {}", report.score);
//! for insight in &report.insights {
//!     println!("Turn {}: {:?} - {}",
//!         insight.turn_index + 1,
//!         insight.severity,
//!         insight.message);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Standalone API (for testing/simulations)
//!
//! ```no_run
//! use agtrace_sdk::{SessionHandle, types::AgentEvent};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // When you have raw events without Client (e.g., testing, simulations)
//! let events: Vec<AgentEvent> = vec![/* ... */];
//! let handle = SessionHandle::from_events(events);
//!
//! let session = handle.assemble()?;
//! println!("Session has {} turns", session.turns.len());
//! # Ok(())
//! # }
//! ```

pub mod analysis;
pub mod client;
pub mod error;
pub mod types;
pub mod watch;

// Re-export core domain types for convenience
pub use agtrace_engine::AgentSession;

// Public facade
pub use analysis::{AnalysisReport, Insight, Lens, Severity};
pub use client::{
    Client, InsightClient, ProjectClient, SessionClient, SessionHandle, SystemClient, WatchClient,
};
pub use error::{Error, Result};
pub use types::{
    AgentEvent, EventPayload, ExportStrategy, SessionFilter, SessionSummary, StreamId, ToolKind,
};
pub use watch::{LiveStream, WatchBuilder};

// ============================================================================
// Low-level Utilities (Power User API)
// ============================================================================

/// Utility functions for building custom observability tools.
///
/// These are stateless, pure functions that power the CLI and can be used
/// by external tool developers to replicate or extend agtrace functionality.
///
/// # When to use this module
///
/// - Building custom TUIs or dashboards that need event stream processing
/// - Writing tests that need to compute project hashes
/// - Implementing custom project detection logic
///
/// # Examples
///
/// ## Event Processing
///
/// ```no_run
/// use agtrace_sdk::{Client, utils};
/// use agtrace_sdk::watch::{StreamEvent, WorkspaceEvent};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::connect("~/.agtrace").await?;
/// let stream = client.watch().all_providers().start()?;
///
/// for workspace_event in stream.take(10) {
///     if let WorkspaceEvent::Stream(StreamEvent::Events { events, .. }) = workspace_event {
///         for event in events {
///             let updates = utils::extract_state_updates(&event);
///             if updates.is_new_turn {
///                 println!("New turn started!");
///             }
///             if let Some(usage) = updates.usage {
///                 println!("Token usage: {:?}", usage);
///             }
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// ## Project Hash Computation
///
/// ```no_run
/// use agtrace_sdk::utils;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let project_root = utils::discover_project_root(None)?;
/// let hash = utils::project_hash_from_root(&project_root.to_string_lossy());
/// println!("Project hash: {}", hash);
/// # Ok(())
/// # }
/// ```
pub mod utils {
    // Event processing utilities
    pub use agtrace_engine::extract_state_updates;

    // Project management utilities
    pub use agtrace_types::{
        discover_project_root, project_hash_from_root, resolve_effective_project_hash,
    };
}
