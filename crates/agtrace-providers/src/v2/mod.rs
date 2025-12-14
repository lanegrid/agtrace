// V2 normalization layer - converts provider raw data to v2::AgentEvent

mod builder;
mod gemini;

pub use builder::EventBuilder;
pub use gemini::normalize_gemini_session_v2;
