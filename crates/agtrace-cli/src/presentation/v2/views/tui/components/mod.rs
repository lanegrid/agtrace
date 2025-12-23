//! TUI Components
//!
//! Components encapsulate UI State + Input Logic + Render Logic.
//! This prevents the Renderer from becoming a "Big Ball of Mud" by:
//! 1. Hiding state manipulation details inside components
//! 2. Delegating input handling to specialized handlers
//! 3. Ensuring index safety within component boundaries
//!
//! ## Pattern:
//! ```rust,ignore
//! pub struct FooComponent {
//!     state: SomeState, // Private UI state
//! }
//!
//! impl FooComponent {
//!     pub fn handle_input(&mut self, key: KeyEvent) -> Option<Action> {
//!         // Handle input, return action if parent needs to respond
//!     }
//!
//!     pub fn render(&mut self, f: &mut Frame, area: Rect, data: &FooViewModel) {
//!         // Index safety checks here
//!         // Render using Views
//!     }
//! }
//! ```

pub mod dashboard;
pub mod timeline;

pub use dashboard::DashboardComponent;
pub use timeline::TimelineComponent;
