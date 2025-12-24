//! # Presentation Layer
//!
//! This module implements the **User Interface** logic for the CLI.
//! It is designed using an adaptation of the **MVVM (Model-View-ViewModel)** pattern
//! with **Component-based UI State management** for interactive TUI.
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
//! ### For Interactive TUI:
//! Components encapsulate UI state and logic to prevent Renderer from becoming a "Big Ball of Mud".
//!
//! ```text
//! [ Handler ] --> [ Presenter ] --> [ ViewModel ] -----> [ Renderer (Router) ]
//!                                      (Data)                    |
//!                                                                v
//!                                                      [ Component ] <-- User Input
//!                                                      (State + Logic)
//!                                                            |
//!                                                            v
//!                                                        [ View ]
//!                                                        (Widget)
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
//! ### 4. The TUI Component Rules (For Interactive UIs) 🎮
//! **Separate Data (ViewModel) from State (UI State) and Logic (Component).**
//!
//! #### 4 Iron Rules for Multi-Page TUI:
//!
//! 1. **Data vs State Separation**
//!    * **ViewModel (from Presenter):** Read-only snapshot. Contains WHAT to display.
//!    * **UI State (in Component):** Mutable context. Contains WHERE user is (scroll, selection).
//!    * ❌ Never put scroll position in ViewModel
//!    * ✅ Always keep it in Component's private state
//!
//! 2. **Renderer as Router**
//!    * Renderer delegates, it does not decide.
//!    * ❌ `match key { Up => self.timeline_state.select(...) }`
//!    * ✅ `self.timeline_component.handle_input(key)`
//!
//! 3. **Index Safety**
//!    * Trust the State, but Verify against Data.
//!    * Always clamp cursor position before rendering: `selected = min(state.selected, data.len() - 1)`
//!
//! 4. **Action Boundaries**
//!    * UI Actions stay locally (scroll, tab switch) → handled in Component
//!    * Domain Actions go up (DB write, navigation) → emit Action to Renderer
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
//! * **For TUI:** Also contains `views/tui/components/` (see below).
//!
//! ### 4. `views/tui/components/` (TUI Component Pattern - NEW)
//! * **What:** Stateful components that encapsulate UI State + Input Handling + Rendering.
//! * **Rule:** Each component owns its private UI state (ListState, scroll position, etc.) and exposes:
//!   * `handle_input(&mut self, key: KeyEvent) -> Option<Action>` - Process user input
//!   * `render(&mut self, f: &mut Frame, area: Rect, data: &ViewModel)` - Render with index safety
//! * **Examples:** `TimelineComponent`, `DashboardComponent`
//! * **When to use:** Interactive TUI pages that need state management
//!
//! ### 5. `renderers/` (The Driver / Router)
//! * **What:** The entry point that takes a ViewModel and handles output.
//! * **For Console:** Switches between JSON and Text output.
//! * **For TUI:** Acts as a Router that delegates to Components. **No business logic, only orchestration.**
//!
//! ### 6. `formatters/` (The Utilities)
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
//! | Handle keyboard input for TUI | **`views/tui/components/`** (Component's `handle_input`) |
//! | Manage scroll position / selection | **`views/tui/components/`** (Private state in Component) |
//! | Perform index safety checks | **`views/tui/components/`** (In Component's `render` method) |
//! | Add a new TUI page/tab | **`views/tui/components/`** (New Component) + **`renderers/tui.rs`** (Router) |

pub mod formatters;
pub mod presenters;
pub mod renderers;
pub mod view_models;
pub mod views;

// Re-exports for convenience
pub use renderers::{ConsoleRenderer, Renderer};
pub use view_models::{
    common::{OutputFormat, ViewMode},
    CommandResultViewModel, CreateView, Guidance, StatusBadge, StatusLevel,
};
