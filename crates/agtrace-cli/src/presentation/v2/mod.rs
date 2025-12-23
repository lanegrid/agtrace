//! # Presentation Layer (v2)
//!
//! This module implements the **User Interface** logic for the CLI.
//! It is designed using an adaptation of the **MVVM (Model-View-ViewModel)** pattern
//! to ensure strict separation between the Domain Logic (`agtrace_engine`) and the Output Logic.
//!
//! ## 🏗️ Architecture & Data Flow
//!
//! The data flow is strictly unidirectional.
//! The `Renderer` decides whether to format via `serde` (JSON) or `View` (Text).
//!
//! ```text
//! [ Handler ] --> [ Presenter ] --> [ ViewModel ] --> [ Renderer ] ==(JSON)==> [ serde_json ] --> Output
//!    (Controller)      (Converter)       (Data)          (Driver)  ==(Text)==> [ View ] --> Output
//!                                                                                 (Layout)
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
//! * **What:** Structs that implement `fmt::Display`.
//! * **Rule:** Handles **Layout** (indentation), **Styling** (colors), **Filtering** (hiding items based on Mode), and **Formatting** (using `formatters`).
//! * **Pattern:** `struct SessionView<'a> { data: &'a SessionVM, mode: ViewMode }`
//!
//! ### 4. `renderers/` (The Driver)
//! * **What:** The entry point that takes a ViewModel and handles the switch between JSON and Text output.
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
