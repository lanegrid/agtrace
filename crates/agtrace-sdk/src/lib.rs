//! agtrace-sdk: SDK for AI agent observability.
//!
//! **Note**: README.md is auto-generated from this rustdoc using `cargo-rdme`.
//! To update: `cargo rdme --workspace-project agtrace-sdk`
//!
//! # Overview
//!
//! `agtrace-sdk` provides a high-level, stable API for building tools on top of agtrace.
//! It powers agtrace's MCP server (letting agents query their execution history) and CLI tools,
//! and can be embedded in your own applications. The SDK normalizes logs from multiple providers
//! (Claude Code, Codex) into a unified data model, enabling cross-provider analysis.
//!
//! # Quickstart
//!
//! ```no_run
//! use agtrace_sdk::{Client, types::SessionFilter};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to the local workspace
//! let client = Client::connect_default().await?;
//!
//! // List sessions and browse the most recent one
//! let sessions = client.sessions().list(SessionFilter::all())?;
//! if let Some(summary) = sessions.first() {
//!     let handle = client.sessions().get(&summary.id)?;
//!     let session = handle.assemble()?;
//!
//!     println!("Session: {} turns, {} tokens",
//!         session.turns.len(),
//!         session.stats.total_tokens);
//!
//!     // Browse tool calls
//!     for turn in &session.turns {
//!         for step in &turn.steps {
//!             for tool in &step.tools {
//!                 println!("  {} ({})",
//!                     tool.call.content.name(),
//!                     if tool.is_error { "failed" } else { "ok" });
//!             }
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! For complete examples, see the [`examples/`](https://github.com/lanegrid/agtrace/tree/main/crates/agtrace-sdk/examples) directory.
//!
//! # Architecture
//!
//! This SDK acts as a facade over:
//! - `agtrace-types`: Core domain models (AgentEvent, etc.)
//! - `agtrace-providers`: Multi-provider log normalization
//! - `agtrace-engine`: Session assembly and analysis
//! - `agtrace-index`: Metadata storage and querying
//! - `agtrace-runtime`: Internal orchestration layer
//!
//! # Usage Patterns
//!
//! ## Session Browsing
//!
//! Access structured session data (Turn → Step → Tool hierarchy):
//!
//! ```no_run
//! use agtrace_sdk::{Client, types::SessionFilter};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::connect_default().await?;
//! let sessions = client.sessions().list(SessionFilter::all())?;
//!
//! for summary in sessions.iter().take(5) {
//!     let handle = client.sessions().get(&summary.id)?;
//!     let session = handle.assemble()?;
//!     println!("{}: {} turns, {} tokens",
//!         summary.id,
//!         session.turns.len(),
//!         session.stats.total_tokens);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Real-time Monitoring
//!
//! Watch for events as they happen:
//!
//! ```no_run
//! use agtrace_sdk::Client;
//! use futures::stream::StreamExt;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::connect_default().await?;
//! let mut stream = client.watch().all_providers().start()?;
//! while let Some(event) = stream.next().await {
//!     println!("Event: {:?}", event);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Diagnostics
//!
//! Run diagnostic checks on sessions:
//!
//! ```no_run
//! use agtrace_sdk::{Client, Diagnostic, types::SessionFilter};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::connect_default().await?;
//! let sessions = client.sessions().list(SessionFilter::all())?;
//! if let Some(summary) = sessions.first() {
//!     let handle = client.sessions().get(&summary.id)?;
//!     let report = handle.analyze()?
//!         .check(Diagnostic::Failures)
//!         .check(Diagnostic::Loops)
//!         .report()?;
//!
//!     println!("Health: {}/100", report.score);
//!     for insight in &report.insights {
//!         println!("  Turn {}: {}", insight.turn_index + 1, insight.message);
//!     }
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
pub mod mcp;
pub mod providers;
pub mod query;
pub mod types;
pub mod watch;

// Re-export core domain types for convenience
pub use agtrace_engine::AgentSession;

/// Live multi-agent workspace model (agent graph, per-agent state, feed), folded
/// from watcher events. Consumed by the `watch` TUI.
pub use agtrace_engine::workspace;

// Public facade
pub use analysis::{AnalysisReport, Diagnostic, Insight, Severity};
pub use client::{
    ChildSessionInfo, Client, ClientBuilder, InsightClient, ProjectClient, SessionClient,
    SessionHandle, SystemClient, WatchClient,
};
pub use error::{Error, Result};
pub use providers::{Providers, ProvidersBuilder};
pub use types::{
    AgentEvent, EventPayload, ExportStrategy, SessionFilter, SessionSummary, ToolKind,
};
pub use watch::{LiveStream, WatchBuilder};

// Query types for MCP and programmatic usage
pub use query::{EventType, Provider};

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
/// use futures::stream::StreamExt;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::connect_default().await?;
/// let mut stream = client.watch().all_providers().start()?;
///
/// let mut count = 0;
/// while let Some(workspace_event) = stream.next().await {
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
///     count += 1;
///     if count >= 10 { break; }
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
    use crate::types::{AgentProvider, ContextEvidence, ContextWindow, ModelCatalog};

    // Event processing utilities
    pub use agtrace_engine::extract_state_updates;

    // Project management utilities
    pub use agtrace_core::{
        discover_project_root, project_hash_from_root, repository_hash_from_path,
        resolve_effective_project_hash, resolve_workspace_path,
    };

    /// Resolve an agent's context window from its folded evidence.
    ///
    /// This is the only context-window computation in agtrace (design §3): fold every
    /// event of one agent with [`ContextEvidence::apply`], then resolve against a catalog
    /// (usually [`crate::Client::model_catalog`]).
    ///
    /// # Example
    ///
    /// ```
    /// use agtrace_sdk::types::{AgentProvider, ContextEvidence};
    /// use agtrace_sdk::utils;
    ///
    /// let evidence = ContextEvidence::with_model("claude-opus-5");
    /// let catalog = utils::builtin_model_catalog();
    /// let window = utils::resolve_context_window(AgentProvider::ClaudeCode, &evidence, &catalog)
    ///     .unwrap();
    /// assert_eq!(window.tokens, 1_000_000);
    /// ```
    pub fn resolve_context_window(
        provider: AgentProvider,
        evidence: &ContextEvidence,
        catalog: &dyn ModelCatalog,
    ) -> Option<ContextWindow> {
        agtrace_engine::resolve_context_window(provider, evidence, catalog)
    }

    /// Catalog with the built-in model tables only (no user config, no provider caches).
    ///
    /// Deterministic; for tools without a workspace. Prefer
    /// [`crate::Client::model_catalog`] when a client is available.
    pub fn builtin_model_catalog() -> agtrace_providers::BuiltinModelCatalog {
        agtrace_providers::BuiltinModelCatalog::tables_only()
    }

    // Event filtering utilities

    /// Filter events suitable for display (excludes sidechain/subagent events).
    ///
    /// This is the recommended way to filter events for user-facing displays
    /// like TUI or console output. It removes internal agent communication
    /// (sidechains) and shows only main stream events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use agtrace_sdk::{Client, utils};
    /// use agtrace_sdk::watch::{StreamEvent, WorkspaceEvent};
    /// use futures::stream::StreamExt;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::connect_default().await?;
    /// let mut stream = client.watch().all_providers().start()?;
    ///
    /// let mut count = 0;
    /// while let Some(workspace_event) = stream.next().await {
    ///     if let WorkspaceEvent::Stream(StreamEvent::Events { events, .. }) = workspace_event {
    ///         let display_events = utils::filter_display_events(&events);
    ///         for event in display_events {
    ///             println!("Event: {:?}", event.payload);
    ///         }
    ///     }
    ///     count += 1;
    ///     if count >= 10 { break; }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn filter_display_events(
        events: &[crate::types::AgentEvent],
    ) -> Vec<crate::types::AgentEvent> {
        events
            .iter()
            .filter(|e| is_display_event(e))
            .cloned()
            .collect()
    }

    /// Check if an event should be displayed (non-sidechain).
    ///
    /// Returns `true` for main agent events, `false` for Claude subagent events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use agtrace_sdk::{Client, utils};
    /// use agtrace_sdk::watch::{StreamEvent, WorkspaceEvent};
    /// use futures::stream::StreamExt;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::connect_default().await?;
    /// let mut stream = client.watch().all_providers().start()?;
    ///
    /// let mut count = 0;
    /// while let Some(workspace_event) = stream.next().await {
    ///     if let WorkspaceEvent::Stream(StreamEvent::Events { events, .. }) = workspace_event {
    ///         for event in &events {
    ///             if utils::is_display_event(event) {
    ///                 println!("Display event: {:?}", event.payload);
    ///             }
    ///         }
    ///     }
    ///     count += 1;
    ///     if count >= 10 { break; }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_display_event(event: &crate::types::AgentEvent) -> bool {
        !event.agent.is_claude_subagent()
    }
}
