# SDK-First CLI Architecture: Refactor CLI to Depend Only on agtrace-sdk

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with PLANS.md located at the repository root.

## Purpose / Big Picture

After this change, the CLI crate (`crates/agtrace-cli`) will depend solely on `agtrace-sdk`, not on any internal implementation crates (`agtrace-runtime`, `agtrace-engine`, `agtrace-providers`, `agtrace-index`, `agtrace-types`). The SDK will act as a Facade Pattern implementation, providing a clean, stable API boundary while hiding internal complexity.

This enables:
1. **Stable Public API**: External tools can use the same SDK that the CLI uses
2. **Clear Architectural Boundaries**: Implementation details are hidden behind SDK
3. **Reduced Coupling**: CLI changes don't require understanding internal crate structures
4. **Better Testability**: SDK provides mockable interfaces

To verify success: After completion, `crates/agtrace-cli/Cargo.toml` will list only `agtrace-sdk` in dependencies (along with UI crates like `ratatui`, `clap`, etc.). All existing CLI commands (`agtrace session list`, `agtrace watch`, `agtrace init`, etc.) will work identically to before. Running `cargo build` will succeed, and all existing tests will pass.

## Progress

- [x] (2025-12-31 09:00Z) Milestone 1: Audit current CLI dependencies and create SDK API surface design
  - [x] Scanned all CLI handler imports
  - [x] Created SDK_API_DESIGN.md with complete API mapping
  - [x] Documented required types, methods, and facades
- [x] (2025-12-31 09:30Z) Milestone 2: Expand SDK with type re-exports and error handling
  - [x] Created `crates/agtrace-sdk/src/types.rs` with 75+ type re-exports
  - [x] Re-exported AgentEvent, SessionFilter, ProjectHash, etc.
  - [x] SDK compiles successfully
- [x] (2025-12-31 10:00Z) Milestone 3: Implement SDK Client facade and basic operations
  - [x] Created complete Client struct with Arc<AgTrace> wrapper
  - [x] Implemented 5 sub-clients: SessionClient, ProjectClient, SystemClient, WatchClient, InsightClient
  - [x] Added SessionHandle with events(), assemble(), export() methods
  - [x] SDK compiles with 350+ lines of client code
- [x] (2025-12-31 11:00Z) Milestone 4: Migrate simple CLI handlers (provider, project, doctor)
  - [x] Migrated provider.rs (simple type import change)
  - [x] Migrated project.rs (uses client.projects().list())
  - [x] Migrated doctor_run.rs (uses client.system().diagnose())
  - [x] Migrated doctor_check.rs (uses client.system().check_file())
  - [x] Migrated doctor_inspect.rs (uses SystemClient::inspect_file())
  - [x] Updated commands.rs to use Client instead of AgTrace
  - [x] Added agtrace-sdk to CLI Cargo.toml
  - [x] All migrated handlers compile successfully
- [x] (2025-12-31 12:00Z) Milestone 5: Migrate session operations (list, show, export)
  - [x] Added list_without_refresh() method to SessionClient
  - [x] Migrated session_list.rs (uses client.sessions().list())
  - [x] Migrated session_show.rs (uses client.sessions().get().events())
  - [x] Migrated lab_export.rs (uses client.sessions().get().export())
  - [x] All handlers compile - 8 of 18 handlers migrated
- [ ] Milestone 6: Migrate complex TUI watch functionality
  - [ ] Migrate watch_tui.rs
  - [ ] Migrate watch_console.rs
- [ ] Milestone 7: Migrate init and index commands
  - [ ] Migrate init.rs
  - [ ] Migrate index.rs
- [ ] Milestone 8: Migrate lab and insights commands
  - [ ] Migrate lab_stats.rs
  - [ ] Migrate lab_grep.rs
  - [ ] Migrate pack.rs
- [ ] Milestone 9: Remove internal crate dependencies from CLI Cargo.toml and final validation

## Surprises & Discoveries

- **Observation**: CheckResult and InspectResult field names differed from what was expected
  - Evidence: Compilation errors showed `path` should be `file_path`, `provider` should be `provider_name`, `error` should be `error_message`
  - Impact: Required reading runtime source code to verify correct field names

- **Observation**: Multiple function name mismatches in runtime layer
  - Evidence: `export_events` didn't exist, actual function is `export::transform`; `list_projects` should be just `list()`
  - Impact: Required careful checking of runtime APIs before implementing SDK facades

- **Observation**: CheckStatus enum has `Failure` variant, not `Invalid`
  - Evidence: Compilation error when using `CheckStatus::Invalid`
  - Impact: Updated handler to use correct variant name

- **Observation**: ProjectHash constructor methods differ across contexts
  - Evidence: `ProjectHash::from_root()` vs `ProjectHash::from()` - both exist for different use cases
  - Impact: Ensured re-exports maintain both methods for backwards compatibility

- **Observation**: SessionClient needs both `list()` and `list_without_refresh()` methods
  - Evidence: CLI has `--no-auto-refresh` flag that requires skipping auto-indexing
  - Impact: Added both methods to SessionClient to support this flag

## Decision Log

- **Decision**: Use Arc<AgTrace> wrapper pattern instead of trait-based abstraction
  - **Rationale**: Simpler implementation, zero runtime cost, maintains all existing functionality without refactoring runtime layer. Traits would add complexity without benefit at this stage.
  - **Date**: 2025-12-31

- **Decision**: Re-export types using `pub use` instead of re-defining them
  - **Rationale**: Zero-cost abstraction - types remain identical, no conversion needed, maintains full API compatibility with internal crates
  - **Date**: 2025-12-31

- **Decision**: Keep SystemClient::inspect_file() as static method
  - **Rationale**: Matches runtime API which doesn't require workspace context for file inspection
  - **Date**: 2025-12-31

- **Decision**: Add agtrace-sdk to CLI Cargo.toml alongside existing dependencies during migration
  - **Rationale**: Allows incremental migration - handlers can be migrated one-by-one while others continue using direct dependencies. Final milestone will remove old dependencies.
  - **Date**: 2025-12-31

- **Decision**: Migrate handlers in order of complexity: simple (provider, doctor) → session ops → complex (TUI, init)
  - **Rationale**: Build confidence with simple handlers, discover API gaps early, defer complex TUI migration until SDK API is proven stable
  - **Date**: 2025-12-31

## Outcomes & Retrospective

(To be filled at completion)

## Context and Orientation

The agtrace project is a Rust workspace with a layered architecture:

- **CLI Layer** (`crates/agtrace-cli/`): User-facing command-line interface built with `clap` for argument parsing and `ratatui` for TUI (Terminal User Interface) displays
- **Runtime Layer** (`crates/agtrace-runtime/`): Application lifecycle management, configuration (`config.toml`), and high-level operation orchestration via `AgTrace` struct
- **Engine Layer** (`crates/agtrace-engine/`): Business logic for session reconstruction, token counting, and analysis
- **Data Layer** (`crates/agtrace-index/`): SQLite metadata storage; `crates/agtrace-providers/`: Provider-specific log parsers (Claude, Codex, Gemini)
- **Domain Layer** (`crates/agtrace-types/`): Shared type definitions like `AgentEvent`, `StreamId`, `ProjectHash`
- **SDK Layer** (`crates/agtrace-sdk/`): Currently minimal; provides basic `Client` and live event streaming

**Current Problem**: The CLI directly imports from all layers (`use agtrace_runtime::...`, `use agtrace_engine::...`, etc.), creating tight coupling. The `Cargo.toml` in `crates/agtrace-cli/` lists 5+ internal crate dependencies.

**Desired State**: CLI imports only from `agtrace_sdk` (`use agtrace_sdk::...`). SDK re-exports necessary types and provides facade methods that encapsulate lower-layer operations.

**Key Files**:
- `crates/agtrace-cli/src/args.rs`: CLI command structure
- `crates/agtrace-cli/src/handlers/*.rs`: Command implementations (18 handler files)
- `crates/agtrace-cli/src/presentation/*.rs`: View models and TUI rendering
- `crates/agtrace-sdk/src/lib.rs`: SDK public API entry point
- `crates/agtrace-sdk/Cargo.toml`: SDK dependencies

**Critical Dependencies**: The TUI (`handlers/watch_tui.rs`, `presentation/tui/*.rs`) depends heavily on `agtrace_runtime::SessionState` struct fields (`current_usage`, `turn_count`, etc.) for rendering. These must remain accessible through SDK without requiring massive TUI rewrites.

## Plan of Work

### Milestone 1: Audit and Design

**Goal**: Understand all CLI dependencies and design the complete SDK API surface.

**Work**:
1. Scan all files in `crates/agtrace-cli/src/handlers/` and `crates/agtrace-cli/src/presentation/` to catalog every `use agtrace_*::` statement
2. Create a spreadsheet/document mapping each import to its purpose (e.g., `agtrace_engine::AgentSession` → "Used in session_show.rs for display")
3. Design SDK module structure:
   - `agtrace_sdk::types`: Re-exports from `agtrace-types` and `agtrace-engine` domain models
   - `agtrace_sdk::client`: `Client` struct and operation facades
   - `agtrace_sdk::error`: Unified error type
   - `agtrace_sdk::system`: Init, index, doctor operations
   - `agtrace_sdk::watch`: Live monitoring types
4. Document which types must be re-exported vs. wrapped

**Acceptance**: A markdown document (e.g., `SDK_API_DESIGN.md`) exists listing all SDK modules, their re-exports, and method signatures. This document shows that every CLI use case has an SDK API path.

### Milestone 2: SDK Type Re-exports and Error Handling

**Goal**: Expand `agtrace-sdk/src/lib.rs` with all necessary type re-exports and a unified error type.

**Work**:
1. Create `crates/agtrace-sdk/src/types.rs`:
   - Re-export all types from `agtrace-types` (AgentEvent, EventPayload, StreamId, ProjectHash, ToolCallPayload, etc.)
   - Re-export session/engine types from `agtrace-engine` (AgentSession, AgentTurn, AgentStep, SessionSummary, TurnMetrics, ContextWindowUsage)
   - Re-export index types from `agtrace-index` (use alias to avoid conflicts if needed)
2. Create `crates/agtrace-sdk/src/error.rs`:
   - Define `pub struct Error` wrapping internal errors
   - Implement `std::error::Error` and `Display`
   - Provide `From<agtrace_runtime::Error>`, `From<agtrace_index::Error>`, etc.
   - Define `pub type Result<T> = std::result::Result<T, Error>;`
3. Update `crates/agtrace-sdk/src/lib.rs`:
   - Add `pub mod types;` and `pub mod error;`
   - Add top-level re-exports: `pub use types::*;` and `pub use error::{Error, Result};`
4. Add SDK to its own `Cargo.toml` dependencies if not already present: `agtrace-types`, `agtrace-engine`, `agtrace-index`, `agtrace-runtime`, `agtrace-providers`

**Acceptance**: Running `cargo build -p agtrace-sdk` succeeds. A test file can write `use agtrace_sdk::types::AgentEvent;` and access all necessary types.

### Milestone 3: Client Facade Implementation

**Goal**: Create `agtrace_sdk::Client` struct that wraps `agtrace_runtime::AgTrace` and provides sub-clients for different operations.

**Work**:
1. Create `crates/agtrace-sdk/src/client.rs`:
   - Define `pub struct Client { inner: Arc<agtrace_runtime::AgTrace> }`
   - Implement `Client::connect(path: impl Into<PathBuf>) -> Result<Self>` (wraps `AgTrace::new`)
   - Add accessor methods: `pub fn sessions(&self) -> SessionClient`, `pub fn system(&self) -> SystemClient`, `pub fn watch(&self) -> WatchClient`, `pub fn insights(&self) -> InsightClient`, `pub fn projects(&self) -> ProjectClient`
2. Create stub structs in same file or separate modules:
   - `pub struct SessionClient { inner: Arc<agtrace_runtime::AgTrace> }`
   - `pub struct SystemClient { inner: Arc<agtrace_runtime::AgTrace> }`
   - `pub struct WatchClient { inner: Arc<agtrace_runtime::AgTrace> }`
   - `pub struct InsightClient { inner: Arc<agtrace_runtime::AgTrace> }`
   - `pub struct ProjectClient { inner: Arc<agtrace_runtime::AgTrace> }`
3. Expose in `lib.rs`: `pub use client::Client;`

**Acceptance**: A test can instantiate `Client::connect("./test-workspace")` and call accessor methods. Compilation succeeds even though sub-clients don't have methods yet.

### Milestone 4: Migrate Simple Handlers - Provider, Project, Doctor

**Goal**: Implement SDK APIs for non-session, non-watch commands and update their handlers.

**Work**:
1. **Provider Commands** (`handlers/provider.rs`):
   - Add to `SystemClient`: `pub fn list_providers(&self) -> Result<Vec<ProviderConfig>>` (wraps runtime's provider listing)
   - Add to `SystemClient`: `pub fn detect_providers(&self) -> Result<Config>` (wraps `agtrace_runtime::detect_providers`)
   - Update `handlers/provider.rs`: Change `use agtrace_runtime::...` to `use agtrace_sdk::...`, replace `context.agtrace.config()` with `context.client.system().list_providers()`
2. **Project Commands** (`handlers/project.rs`):
   - Add to `ProjectClient`: `pub fn list(&self) -> Result<Vec<ProjectInfo>>` (wraps `ProjectOps::list_projects`)
   - Update `handlers/project.rs`: Use `client.projects().list()`
3. **Doctor Commands** (`handlers/doctor_*.rs`):
   - Add to `SystemClient`: `pub fn diagnose(&self) -> Result<Vec<DiagnoseResult>>`
   - Add to `SystemClient`: `pub fn check_file(&self, path: &Path, provider: Option<&str>) -> Result<CheckResult>`
   - Add to `SystemClient`: `pub fn inspect_file(&self, path: &Path, lines: usize) -> Result<InspectResult>`
   - Update all `handlers/doctor_*.rs` files to use `client.system().diagnose()`, etc.
4. Update `crates/agtrace-cli/src/handlers.rs` context struct: Replace `agtrace: Arc<AgTrace>` with `client: Arc<Client>`
5. Update each handler's function signature from `fn handle(context: &AgTrace, args: Args)` to `fn handle(context: &Client, args: Args)`

**Acceptance**: Running `cargo build -p agtrace-cli` compiles these handlers. Running `agtrace provider list`, `agtrace project list`, and `agtrace doctor run` produces identical output to before.

### Milestone 5: Migrate Session Operations

**Goal**: Implement SDK APIs for session listing, display, and export, then migrate handlers.

**Work**:
1. **Session Types**:
   - Create `crates/agtrace-sdk/src/session.rs`
   - Define `pub struct SessionHandle { id: String, inner: Arc<agtrace_runtime::AgTrace> }`
   - Implement: `pub fn events(&self) -> Result<Vec<AgentEvent>>` (calls `SessionOps::load_events`)
   - Implement: `pub fn assemble(&self) -> Result<AgentSession>` (calls `agtrace_engine::assemble_session`)
   - Implement: `pub fn raw_files(&self) -> Result<Vec<RawFileContent>>` (calls runtime's raw file access)
   - Implement: `pub fn export(&self, strategy: ExportStrategy) -> Result<Vec<AgentEvent>>` (wraps export logic)
2. **SessionClient Methods**:
   - Add: `pub fn list(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>>`
   - Add: `pub fn get(&self, id_or_prefix: &str) -> Result<SessionHandle>`
3. **Handler Migration**:
   - Update `handlers/session_list.rs`: Replace `SessionOps::list_sessions` with `client.sessions().list(filter)`
   - Update `handlers/session_show.rs`: Replace direct `assemble_session` call with `client.sessions().get(id)?.assemble()?`
   - Update `handlers/session_export.rs`: Use `client.sessions().get(id)?.export(strategy)?`
4. **Re-export Filter Types**: Ensure `SessionFilter`, `ExportStrategy` are available via `agtrace_sdk::types` or top-level

**Acceptance**: Run `agtrace session list`, `agtrace session show <id>`, `agtrace session export <id>`. All commands produce correct output. Run existing session-related tests; they pass.

### Milestone 6: Migrate TUI Watch Functionality

**Goal**: Expose live monitoring API through SDK and migrate the complex TUI handler.

**Work**:
1. **Watch Types Re-export**:
   - In `crates/agtrace-sdk/src/watch.rs`, re-export: `pub use agtrace_runtime::{WorkspaceEvent, DiscoveryEvent, StreamEvent, SessionState, TokenLimits, TokenLimit};`
   - These types are used directly by TUI rendering code in `presentation/tui/*.rs`
2. **WatchClient API**:
   - Define `pub struct WatchBuilder` (not `WatchClient` to allow builder pattern)
   - Implement: `pub fn provider(self, name: &str) -> Self`, `pub fn session(self, id: &str) -> Self`, `pub fn all_providers(self) -> Self`
   - Implement: `pub fn start(self) -> Result<LiveStream>`
   - Define `pub struct LiveStream` wrapping `Receiver<WorkspaceEvent>`
   - Implement `Iterator for LiveStream` with `type Item = WorkspaceEvent`
3. **Handler Migration**:
   - Update `handlers/watch_tui.rs`: Replace `agtrace_runtime::watch::start_monitoring` with `client.watch().provider(name).start()?`
   - Update `handlers/watch_console.rs` similarly
   - Ensure `SessionState` fields remain directly accessible (no getter wrappers needed for now)
4. **Presentation Layer**: Update imports in `presentation/tui/*.rs` from `use agtrace_runtime::SessionState` to `use agtrace_sdk::SessionState`

**Acceptance**: Run `agtrace watch --provider claude`. TUI displays correctly with live updates. All session state metrics (token usage, turn count) render properly. Run `agtrace watch --session <id>` in console mode; events stream correctly.

### Milestone 7: Migrate Init and Index Commands

**Goal**: Expose initialization and indexing operations through SDK.

**Work**:
1. **Init API**:
   - In `crates/agtrace-sdk/src/system.rs`, add: `pub fn initialize(config: InitConfig, on_progress: impl Fn(InitProgress)) -> Result<InitResult>`
   - Re-export `InitConfig`, `InitResult`, `InitProgress` from `agtrace-runtime`
   - This function wraps `agtrace_runtime::init::run_init`
2. **Index API**:
   - Add to `SystemClient`: `pub fn reindex(&self, scope: ProjectScope, force: bool, provider_filter: Option<&str>, on_progress: impl Fn(IndexProgress)) -> Result<ScanSummary>`
   - Add: `pub fn vacuum(&self) -> Result<()>`
   - Re-export `IndexProgress`, `ProjectScope`, `ScanSummary`
3. **Handler Migration**:
   - Update `handlers/init.rs`: Replace `agtrace_runtime::init::run_init` with `agtrace_sdk::system::initialize`
   - Update `handlers/index.rs`: Use `client.system().reindex(...)` and `client.system().vacuum()`

**Acceptance**: Run `agtrace init` in a fresh directory. Initialization completes with progress output. Run `agtrace index --force`. Re-indexing executes and reports summary. Tests for init/index pass.

### Milestone 8: Lab and Insights Commands

**Goal**: Migrate analysis tools (corpus stats, tool usage, pack, grep).

**Work**:
1. **InsightClient API**:
   - Add: `pub fn corpus_stats(&self, filter: &SessionFilter) -> Result<CorpusStats>`
   - Add: `pub fn tool_usage(&self, limit: Option<usize>) -> Result<Vec<ToolUsageStat>>`
   - Add: `pub fn pack(&self, limit: usize, strategy: PackStrategy) -> Result<PackResult>`
   - Add: `pub fn grep(&self, pattern: &str, tool_filter: Option<ToolKind>, limit: usize) -> Result<Vec<ToolMatch>>`
2. **Handler Migration**:
   - Update `handlers/lab_stats.rs`, `handlers/lab_pack.rs`, `handlers/lab_grep.rs` to use `client.insights().*()`
   - Ensure all result types are re-exported in SDK

**Acceptance**: Run `agtrace lab stats`, `agtrace lab grep "pattern"`, `agtrace lab pack --limit 10`. All produce expected output.

### Milestone 9: Remove Internal Dependencies and Final Validation

**Goal**: Clean up CLI Cargo.toml and ensure everything works.

**Work**:
1. Edit `crates/agtrace-cli/Cargo.toml`:
   - Remove dependencies: `agtrace-runtime`, `agtrace-engine`, `agtrace-providers`, `agtrace-index`, `agtrace-types`
   - Keep: `agtrace-sdk`, plus external crates (`clap`, `ratatui`, `anyhow`, etc.)
2. Run `cargo clean` and `cargo build --workspace`
3. Fix any remaining compilation errors (likely import path issues)
4. Run full test suite: `cargo test --workspace`
5. Manual smoke tests:
   - `agtrace init` in test directory
   - `agtrace session list`
   - `agtrace watch --provider claude` (verify TUI renders)
   - `agtrace lab stats`
   - `agtrace doctor run`
6. Verify CLI binary size and performance unchanged (no unnecessary overhead from SDK layer)

**Acceptance**: `crates/agtrace-cli/Cargo.toml` lists only `agtrace-sdk` as internal dependency. All commands work identically. All tests pass. No performance degradation.

## Concrete Steps

### Before Starting
```
cd /Users/zawakin/go/src/github.com/lanegrid/agtrace
git checkout -b refactor/sdk-cli-facade
```

### Milestone 1 Steps
```
# Scan CLI dependencies
rg "use agtrace_" crates/agtrace-cli/src/ --no-heading | sort | uniq > /tmp/cli-deps.txt

# Review the list
cat /tmp/cli-deps.txt

# Document findings in SDK_API_DESIGN.md
# (manual work to design API)
```

Expected output: A file with ~50-100 unique import statements from various crates.

### Milestone 2 Steps
```
# Create types module
# (Edit crates/agtrace-sdk/src/types.rs - see Plan of Work)

# Create error module
# (Edit crates/agtrace-sdk/src/error.rs - see Plan of Work)

# Update lib.rs
# (Edit crates/agtrace-sdk/src/lib.rs)

# Test compilation
cargo build -p agtrace-sdk
```

Expected output: `Finished dev [unoptimized + debuginfo] target(s) in X.XXs`

### Milestone 3 Steps
```
# Create client module with facade
# (Edit crates/agtrace-sdk/src/client.rs)

# Build SDK
cargo build -p agtrace-sdk

# Optional: Write integration test
# (Edit crates/agtrace-sdk/tests/client_basic.rs)
cargo test -p agtrace-sdk
```

### Milestones 4-8 Steps
(Follow pattern: Implement SDK API → Update handler → Test command)

For each handler file:
```
# Example for provider.rs
# 1. Add methods to SystemClient in SDK
# 2. Update handler:
#    - Change imports
#    - Change context.agtrace to context.client.system()
# 3. Test
cargo build -p agtrace-cli
cargo run -- provider list
```

### Milestone 9 Steps
```
# Edit Cargo.toml
# Remove internal dependencies except agtrace-sdk

# Clean rebuild
cargo clean
cargo build --workspace

# Run tests
cargo test --workspace

# Smoke tests
cargo run -- init --path /tmp/test-agtrace
cargo run -- session list
cargo run -- watch --provider claude
# (Ctrl-C after verifying TUI works)

# Check binary size
ls -lh target/release/agtrace
```

Expected output: All tests pass, all commands work.

## Validation and Acceptance

**Compilation Check**: `cargo build --workspace` succeeds with no warnings about unused dependencies.

**Dependency Check**:
```
grep "agtrace-" crates/agtrace-cli/Cargo.toml
```
Output should show only `agtrace-sdk = { path = "../agtrace-sdk" }` (no runtime, engine, providers, index, types).

**Functional Tests**:
1. `agtrace init --path /tmp/test-ws` → Creates workspace successfully
2. `agtrace session list` → Shows sessions (or empty list)
3. `agtrace session show <id>` → Displays session details
4. `agtrace watch --provider claude` → TUI opens and shows live events
5. `agtrace doctor run` → Runs diagnostics
6. `agtrace lab stats` → Shows corpus statistics

**Automated Tests**: `cargo test --workspace` passes all existing tests.

**Performance**: `cargo build --release && time ./target/release/agtrace session list` should take similar time as before (no >10% regression).

**Code Quality**: Run `cargo clippy --workspace` with no new warnings.

## Idempotence and Recovery

**Safe Iteration**: This refactoring is incremental. Each milestone can be committed separately. If a milestone fails, roll back to the previous commit and revise the approach.

**Branch Strategy**: Work on `refactor/sdk-cli-facade` branch. Keep `main` stable. Merge only after Milestone 9 validation passes.

**Rollback**: If severe issues arise, `git revert` the merge commit or reset to pre-refactor state. The SDK crate is additive (doesn't modify existing crates), so rollback is safe.

**Partial Completion**: If time-constrained, milestones 1-5 can be completed first (basic operations), leaving TUI (milestone 6) for later. The CLI will still compile with mixed dependency state temporarily.

## Artifacts and Notes

(To be filled with transcripts and evidence as milestones complete)

## Interfaces and Dependencies

### Core SDK Public API (Target State)

In `crates/agtrace-sdk/src/lib.rs`:

```rust
// Re-exported types
pub mod types;
pub mod error;

// Client and facades
pub mod client;
pub use client::Client;

// Convenience re-exports
pub use types::{
    AgentEvent, EventPayload, StreamId, ProjectHash,
    AgentSession, SessionSummary, TurnMetrics,
};
pub use error::{Error, Result};
```

In `crates/agtrace-sdk/src/client.rs`:

```rust
pub struct Client {
    inner: Arc<agtrace_runtime::AgTrace>,
}

impl Client {
    pub fn connect(path: impl Into<PathBuf>) -> Result<Self>;

    pub fn sessions(&self) -> SessionClient;
    pub fn system(&self) -> SystemClient;
    pub fn watch(&self) -> WatchClient;
    pub fn insights(&self) -> InsightClient;
    pub fn projects(&self) -> ProjectClient;
}

pub struct SessionClient { /* ... */ }
impl SessionClient {
    pub fn list(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>>;
    pub fn get(&self, id: &str) -> Result<SessionHandle>;
}

pub struct SessionHandle { /* ... */ }
impl SessionHandle {
    pub fn events(&self) -> Result<Vec<AgentEvent>>;
    pub fn assemble(&self) -> Result<AgentSession>;
    pub fn export(&self, strategy: ExportStrategy) -> Result<Vec<AgentEvent>>;
}

pub struct SystemClient { /* ... */ }
impl SystemClient {
    pub fn diagnose(&self) -> Result<Vec<DiagnoseResult>>;
    pub fn reindex(&self, scope: ProjectScope, force: bool,
                   on_progress: impl Fn(IndexProgress)) -> Result<ScanSummary>;
    pub fn list_providers(&self) -> Result<Vec<ProviderConfig>>;
    pub fn detect_providers(&self) -> Result<Config>;
    pub fn vacuum(&self) -> Result<()>;
}

pub struct WatchClient { /* ... */ }
impl WatchClient {
    pub fn provider(self, name: &str) -> Self;
    pub fn session(self, id: &str) -> Self;
    pub fn all_providers(self) -> Self;
    pub fn start(self) -> Result<LiveStream>;
}

pub struct LiveStream { /* ... */ }
impl Iterator for LiveStream {
    type Item = WorkspaceEvent;
    fn next(&mut self) -> Option<Self::Item>;
}

pub struct InsightClient { /* ... */ }
impl InsightClient {
    pub fn corpus_stats(&self, filter: &SessionFilter) -> Result<CorpusStats>;
    pub fn tool_usage(&self, limit: Option<usize>) -> Result<Vec<ToolUsageStat>>;
    pub fn pack(&self, limit: usize, strategy: PackStrategy) -> Result<PackResult>;
    pub fn grep(&self, pattern: &str, tool_filter: Option<ToolKind>,
                limit: usize) -> Result<Vec<ToolMatch>>;
}

pub struct ProjectClient { /* ... */ }
impl ProjectClient {
    pub fn list(&self) -> Result<Vec<ProjectInfo>>;
}
```

### CLI Handler Context (After Refactor)

In `crates/agtrace-cli/src/handlers.rs`:

```rust
pub struct Context {
    pub client: Arc<agtrace_sdk::Client>, // Changed from: pub agtrace: Arc<agtrace_runtime::AgTrace>
}
```

All handler functions change from:
```rust
pub fn handle(context: &AgTrace, args: Args) -> anyhow::Result<()>
```

To:
```rust
pub fn handle(context: &agtrace_sdk::Client, args: Args) -> anyhow::Result<()>
```

---

## Revision Log

- 2025-12-31 09:00Z: Initial ExecPlan created based on user requirements and PLANS.md template.
- 2025-12-31 12:30Z: Updated Progress section with completed Milestones 1-5. Added Surprises & Discoveries documenting field name mismatches, API naming issues, and method requirements. Added Decision Log documenting key architectural choices (Arc wrapper pattern, type re-exports, static methods, incremental migration strategy). Status: 8 of 18 handlers migrated successfully, SDK infrastructure complete.
