//! # Presentation Layer
//!
//! This module implements the **User Interface** logic for the CLI.
//! It is designed using an adaptation of the **MVVM (Model-View-ViewModel)** pattern
//! to ensure strict separation between the Domain Logic (`agtrace_engine`) and the Output Logic.
//!
//! ## 🏗️ Architecture & Data Flow
//!
//! The data flow is strictly unidirectional:
//!
//! ```text
//! [ Handler ] --> [ Presenter ] --> [ ViewModel ] --> [ Renderer ] --> [ Output ]
//!    (Controller)      (Converter)       (Contract)       (View)        (Console/JSON)
//! ```
//!
//! ---
//!
//! ## 📂 Directory Guide: Where does code go?
//!
//! ### 1. `view_models/` (The Data Contract)
//! * **What:** Structs and Enums that define *what* information is available to the user.
//! * **Rule:** Pure data containers. Must implement `Serialize`. **No** calculation logic.
//! * **Constraint:** Do not expose internal domain types (e.g., `agtrace_types`) directly.
//! * **The JSON Test:** "If I output this struct as JSON, is it clean and machine-readable?"
//!
//! ### 2. `presenters/` (The Transformation Logic)
//! * **What:** Pure functions that convert Domain Models into ViewModels.
//! * **Rule:** Handles filtering, summarization, calculation (e.g., diffs, totals), and mapping.
//! * **Why:** Isolates the display logic from the core engine. Changes in the domain model only require updating the presenter.
//!
//! ### 3. `renderers/` (The Output Strategy)
//! * **What:** The driver that takes a `CommandResultViewModel` and paints it to the screen.
//! * **Rule:** Handles **Layout** (indentation, tree structure), **Styling** (colors, bold), and **Format** (JSON vs Text).
//! * **Components:**
//!     * `console.rs`: Standard stdout rendering.
//!     * `traits.rs`: Defines the `Renderer` interface used by Handlers.
//!
//! ### 4. `formatters/` (The Utilities)
//! * **What:** Reusable, small utility functions for string manipulation.
//! * **Examples:** `format_duration`, `shorten_path`, `humanize_bytes`.
//! * **Why:** Ensures consistency across different commands (e.g., time is always displayed the same way).
//!
//! ---
//!
//! ## ⚖️ Decision Matrix
//!
//! | If you need to... | Go to... |
//! |-------------------|----------|
//! | Add a new field to the JSON output | **`view_models/`** |
//! | Calculate a sum, average, or diff | **`presenters/`** |
//! | Change the color of a warning | **`renderers/`** (or `fmt::Display`) |
//! | Change the indentation of a list | **`renderers/`** (or `fmt::Display`) |
//! | Format a timestamp as "2m ago" | **`formatters/`** |

pub mod formatters;
pub mod presenters;
pub mod renderers;
pub mod view_models;

pub use renderers::{ConsoleRenderer, Renderer};
pub use view_models::{CommandResultViewModel, Guidance, StatusBadge, StatusLevel};
