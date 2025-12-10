use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GeminiSession {
    pub session_id: String,
    pub project_hash: String,
    pub start_time: String,
    pub last_updated: String,
    pub messages: Vec<GeminiMessage>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum GeminiMessage {
    User(UserMessage),
    Gemini(GeminiAssistantMessage),
    Info(InfoMessage),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserMessage {
    pub id: String,
    pub timestamp: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GeminiAssistantMessage {
    pub id: String,
    pub timestamp: String,
    pub content: String,
    pub model: String,
    #[serde(default)]
    pub thoughts: Vec<Thought>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    pub tokens: TokenUsage,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InfoMessage {
    pub id: String,
    pub timestamp: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Thought {
    pub subject: String,
    pub description: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
    #[serde(default)]
    pub result: Vec<FunctionResponse>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub result_display: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub render_output_as_markdown: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponse {
    pub function_response: FunctionResponseInner,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FunctionResponseInner {
    pub id: String,
    pub name: String,
    pub response: Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenUsage {
    pub input: u32,
    pub output: u32,
    pub cached: u32,
    pub thoughts: u32,
    pub tool: u32,
    pub total: u32,
}
