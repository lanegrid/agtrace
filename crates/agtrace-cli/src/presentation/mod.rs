//! # Presentation Layer
//!
//! This module implements the **User Interface** logic for the CLI.
//! It is designed using an adaptation of the **MVVM (Model-View-ViewModel)** pattern
//! to ensure strict separation between the Domain Logic (`agtrace_engine`, `agtrace_types`)
//! and the Output Logic (Console, JSON, TUI).
//!
//! ## 🏗️ Architecture & Rationale
//!
//! The presentation layer is strictly unidirectional. Data flows from the Domain
//! to the User through a series of transformations.
//!
//! ```text
//! [ Handler (Controller) ]
//!        │
//!        ▼
//! [ Domain / Engine ] <--- (Internal Logic & Types)
//!        │
//!        │ (Raw Domain Models)
//!        ▼
//! [ Presenters ]      <--- (Transformation Logic)
//!        │
//!        │ (Pure Data / DTOs)
//!        ▼
//! [ ViewModels ]      <--- (Display Contract)
//!        │
//!        ▼
//! [ Renderers / Views ] <--- (Visual Representation)
//!        │
//!        ▼
//! [ User Output ]     (Console stdout / TUI / JSON)
//! ```
//!
//! ---
//!
//! ## 🧩 Component Responsibilities
//!
//! ### 1. ViewModels (`src/presentation/view_models/`)
//! **Definition:** Data Transfer Objects (DTOs) optimized for display.
//! * **Responsibility:** Defines *what* information is available to the user.
//! * **Constraint:** Must be pure structs/enums. **NO** logic. **NO** dependencies on `agtrace_types` (unless generic `Value`).
//! * **Why:** Decouples the internal domain structure from the external API (JSON output). Internal refactors shouldn't break CLI output scripts.
//!
//! ### 2. Presenters (`src/presentation/presenters/`)
//! **Definition:** Pure functions that map Domain Models to ViewModels.
//! * **Responsibility:** Extracts, summarizes, and formats domain data.
//! * **Constraint:** No side effects (no printing, no I/O). Input is Domain Model, Output is ViewModel.
//! * **Why:** Keeps formatting logic out of the core Engine and keeps display logic out of the Handlers.
//!
//! ### 3. Formatters (`src/presentation/formatters/`)
//! **Definition:** Reusable string manipulation utilities.
//! * **Responsibility:** Low-level formatting (e.g., relative time "5m ago", token bars `[==..]`, path shortening).
//! * **Why:** Ensures consistency across different commands (e.g., time is always displayed the same way).
//!
//! ### 4. Views & Renderers (`src/presentation/views/`, `src/presentation/renderers/`)
//! **Definition:** Strategies for rendering ViewModels to a specific medium.
//! * **Responsibility:**
//!     * **Views:** Implement `fmt::Display` for specific ViewModels (Layout, Color, Indentation).
//!     * **Renderers:** The `TraceView` trait abstracts the output target (Console vs. TUI).
//! * **Why:** Allows the application to switch modes (Interactive vs. Scripting) without changing business logic.
//!
//! ---
//!
//! ## ⚖️ Design Rules (The "Do's and Don'ts")
//!
//! 1.  **Handlers are Thin:** Handlers should only orchestrate. They call the Engine, pass the result to a Presenter, and pass the ViewModel to a Renderer.
//! 2.  **ViewModels are Dumb:** ViewModels should not contain methods like `calculate_total()`. All calculations happen in Presenters.
//! 3.  **Domain Isolation:** Never expose `agtrace_types` directly in a ViewModel. If the domain changes, fix the Presenter, not the View.
//! 4.  **TUI Isolation:** TUI state management (`app.rs`) is complex and isolated within `renderers/tui/`. It does not leak into general logic.
//!
//! ## 📂 Directory Structure
//!
//! * `formatters/` - Small utils (Time, Token strings).
//! * `presenters/` - Converters (Domain -> ViewModel).
//! * `renderers/`  - Output drivers (Console impl, TUI impl, Traits).
//! * `shared/`     - Cross-cutting concerns (DisplayOptions).
//! * `view_models/` - The Data Structures (The API contract).
//! * `views/`      - Console layout logic (`fmt::Display` impls).

pub mod formatters;
pub mod presenters;
pub mod renderers;
pub mod shared;
pub mod v2;
pub mod view_models;
pub mod views;
