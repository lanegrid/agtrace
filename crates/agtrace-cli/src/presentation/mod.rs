//! # Presentation Layer
//!
//! This module implements the **User Interface** logic for the CLI.
//! It is designed using an adaptation of the **MVVM (Model-View-ViewModel)** pattern
//! with handler-owned UI state for the interactive TUI.
//!
//! ## 🏗️ Architecture & Data Flow
//!
//! ### For Console Output (JSON/Text):
//! The data flow is strictly unidirectional.
//!
//! ```text
//! [ Handler ] --> [ Presenter ] --> [ ViewModel ] --> [ Renderer ] ==(JSON)==> [ serde_json ] --> Output
//!    (Controller)      (Converter)       (Data)          (Driver)  ==(Text)==> [ View ] --> Output
//!                                                                                 (Layout)
//! ```
//!
//! ### For the Interactive TUI (`agtrace watch`):
//! The handler owns the UI state (selection, scroll, filters); the presenter is a
//! pure function of the live workspace snapshot and that state.
//!
//! ```text
//! [ WorkspaceSource ] --> [ presenters::watch::build_screen ] --> [ WatchScreenVm ]
//!   (live view + gen)          (pure: view + UiState + now)             |
//!          ^                                                           v
//!   [ handlers::watch ] <-- key actions (views::watch::input) -- [ views::watch::* ]
//!     (UiState, loop)                                             (ratatui widgets)
//! ```
//!
//! ---
//!
//! ## 🌟 Golden Rules
//!
//! ### 1. The JSON Test (Raw Data Strategy) 🧪
//! **ViewModel must contain "Raw Data", not "Formatted Strings".**
//! * ❌ Bad: `struct Vm { duration: "2 minutes" }`
//! * ✅ Good: `struct Vm { duration_sec: u64 }`
//! * **Reason:** JSON output is an API. Clients need numbers, not strings.
//!
//! ### 2. The Density Rule 🔍
//! `ViewMode` defines **Information Density**, not Shape.
//! * **Minimal:** Machine-readable IDs/Paths only. (For pipes/scripts)
//! * **Compact:** One line per item. (For scanning lists)
//! * **Standard:** Structured context/trees. (Default for humans)
//! * **Verbose:** No secrets. All hidden fields and raw values. (For debugging)
//!
//! ### 3. The Schema Stability Rule 📦
//! **JSON Output is always "Full Data".**
//! * `--format json` ignores `ViewMode`. It always dumps the complete ViewModel.
//! * `ViewMode` only affects the Text/Console rendering.
//!
//! ### 4. The TUI Rules (For Interactive UIs) 🎮
//! **Separate Data (ViewModel) from State (UI State) and Input (reducer).**
//!
//! * **ViewModel (from Presenter):** Read-only snapshot. Contains WHAT to display.
//! * **UI State (`view_models::watch::UiState`, owned by the handler):** WHERE the user
//!   is (selection by agent id, collapsed nodes, scroll, filters).
//! * **Input reducer (`views::watch::input`):** maps keys to actions and applies them to
//!   the UI state; domain effects (quit, rescan) go back to the handler.
//! * **Index safety:** views clamp scroll offsets against the data they render.
//!
//! ---
//!
//! ## 📂 Directory Guide: Where does code go?
//!
//! ### 1. `view_models/` (The Data Contract)
//! * **What:** Structs that define *what* information is available.
//! * **Rule:** Pure data containers. Must implement `Serialize`. **No** calculation logic.
//! * **Trait:** Defines `CreateView` trait to bridge Data and View.
//!
//! ### 2. `presenters/` (The Transformation Logic)
//! * **What:** Pure functions that convert Domain Models into ViewModels.
//! * **Rule:** Handles calculation (deltas, totals), grouping, and specific business logic (e.g., "When to show a tip").
//! * **Constraint:** Does **not** use `formatters`. Produces raw data.
//!
//! ### 3. `views/` (The Rendering Logic)
//! * **What:** Structs that implement `fmt::Display` or Ratatui `Widget` trait.
//! * **Rule:** Handles **Layout** (indentation), **Styling** (colors), **Filtering** (hiding items based on Mode), and **Formatting** (using `formatters`).
//! * **Pattern:** `struct SessionView<'a> { data: &'a SessionVM, mode: ViewMode }`
//! * **For TUI:** `views/watch/` holds the ratatui widgets and the input reducer.
//!
//! ### 4. `renderers/` (The Driver)
//! * **What:** The entry point that takes a ViewModel and handles output.
//! * **For Console:** Switches between JSON and Text output.
//!
//! ### 5. `formatters/` (The Utilities)
//! * **What:** Reusable string manipulation functions used by **Views**.
//! * **Examples:** `humanize_bytes(1024) -> "1 KB"`, `truncate(str, 80)`.
//!
//! ---
//!
//! ## ⚖️ Decision Matrix
//!
//! | If you need to... | Go to... |
//! |-------------------|----------|
//! | Add a new field to the JSON output | **`view_models/`** |
//! | Calculate a sum, average, or diff | **`presenters/`** |
//! | Decide *when* to show a "Guidance" | **`presenters/`** |
//! | Change the color of a warning | **`views/`** |
//! | Hide an item in "Compact" mode | **`views/`** (Logic inside `fmt::Display`) |
//! | Format a timestamp as "2m ago" | **`formatters/`** (Called by `views`) |
//! | Handle keyboard input for TUI | **`views/watch/input.rs`** (action + reducer) |
//! | Manage scroll position / selection | **`view_models/watch.rs`** (`UiState`, owned by the handler) |
//! | Change what a TUI pane shows | **`presenters/watch.rs`** (pure `build_screen`) |

pub mod formatters;
pub mod presenters;
pub mod renderers;
pub mod view_models;
pub mod views;

// Re-exports for convenience
pub use renderers::{ConsoleRenderer, Renderer};
pub use view_models::{
    CommandResultViewModel, CreateView, Guidance, StatusBadge, StatusLevel,
    common::{OutputFormat, ViewMode},
};
