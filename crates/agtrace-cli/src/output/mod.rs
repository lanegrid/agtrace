// Re-export from views for backward compatibility
pub use crate::views::doctor::print_results;
pub use crate::views::pack::{
    print_compact as output_compact, print_diagnose as output_diagnose, print_tools as output_tools,
};
pub use crate::views::session::{format_compact, format_token_summary, print_events_timeline};
