// V2 normalization layer - converts provider raw data to v2::AgentEvent

mod builder;
mod claude;
mod codex;
mod gemini;

pub use builder::EventBuilder;
pub use claude::normalize_claude_session_v2;
pub use codex::normalize_codex_session_v2;
pub use gemini::normalize_gemini_session_v2;
