// NOTE: agtrace Architecture Rationale
//
// Why Pointer-Based (not copy raw logs)?
// - Provider schemas evolve frequently (Codex v0.53→v0.63 had 3 breaking changes)
// - Copying creates sync issues and storage bloat
// - Failed parses can be retried without data loss
// - Trade-off: On-demand parsing is slightly slower, but enables fail-safe evolution
//
// Why Schema-on-Read (not normalize at write-time)?
// - Provider schemas change without notice
// - Can improve normalization logic without re-indexing
// - Parsing errors don't block indexing (file registered, parsing retried on view)
// - Enables "diagnose → fix schema → re-read" workflow without re-scanning
// - Trade-off: No pre-computed statistics, but diagnostics become trivial
//
// Why Fail-Safe Indexing?
// - Schema updates shouldn't break existing indexes
// - Register files with minimal metadata even if parsing fails
// - User runs `doctor` to identify issues, then retry parsing after fixes
// - Never lose track of sessions due to temporary schema incompatibility
//
// Why Exact-Match Project Isolation (not hierarchical)?
// - Gemini uses sha256(project_root), different hash per directory level
// - Path-based hierarchy (/project/subdir as child of /project) would be inconsistent across providers
// - Simpler mental model: one directory = one project
// - Trade-off: Can't view parent + child sessions together (use --all-projects if needed)

mod args;
mod commands;
pub mod config;
pub mod context;
pub mod presentation;
mod handlers;
pub mod token_usage;
pub mod types;

pub use args::{
    Cli, Commands, DoctorCommand, IndexCommand, LabCommand, ProjectCommand, ProviderCommand,
    SessionCommand,
};
pub use commands::run;
