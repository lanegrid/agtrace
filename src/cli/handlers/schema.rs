use anyhow::Result;
use owo_colors::OwoColorize;

pub fn handle(provider: String, format: String) -> Result<()> {
    match provider.as_str() {
        "claude" => display_claude_schema(&format),
        "codex" => display_codex_schema(&format),
        "gemini" => display_gemini_schema(&format),
        _ => anyhow::bail!("Unknown provider: {}", provider),
    }
}

fn display_codex_schema(format: &str) -> Result<()> {
    match format {
        "rust" => {
            println!("{}", include_str!("../../providers/codex/schema.rs"));
        }
        "json" => {
            // TODO: Generate JSON Schema
            println!("JSON Schema output not yet implemented");
        }
        _ => {
            // Text format
            println!("{}", "Provider: Codex".bright_blue().bold());
            println!("Schema version: v0.53-v0.63");
            println!();
            println!("{}", "Root structure (JSONL - one record per line):".bold());
            println!("  CodexRecord (enum):");
            println!("    - SessionMeta");
            println!("    - ResponseItem");
            println!("    - EventMsg");
            println!("    - TurnContext");
            println!();
            println!("{}", "SessionMeta:".bold());
            println!("  timestamp: String");
            println!("  payload:");
            println!("    id: String (session_id)");
            println!("    cwd: String");
            println!("    originator: String");
            println!("    cli_version: String");
            println!("    source: String | Object");
            println!("    model_provider: String");
            println!("    git: GitInfo (optional)");
            println!();
            println!("{}", "TurnContext:".bold());
            println!("  timestamp: String");
            println!("  payload:");
            println!("    cwd: String");
            println!("    approval_policy: String");
            println!("    sandbox_policy: SandboxPolicy");
            println!("    model: String");
            println!("    effort: String");
            println!("    summary: String");
            println!();
            println!("{}", "SandboxPolicy (untagged enum):".bold());
            println!("  New format (v0.63+):");
            println!("    {{ \"type\": \"read-only\" | \"workspace-write\" }}");
            println!("  Old format (v0.53):");
            println!("    {{ \"mode\": \"...\", \"network_access\": bool, ... }}");
            println!();
            println!("{}", "ResponseItem:".bold());
            println!("  timestamp: String");
            println!("  payload:");
            println!("    Message (role: user/assistant)");
            println!("    FunctionCall");
            println!("    FunctionCallOutput");
            println!("    Reasoning");
            println!("    CustomToolCall");
            println!("    CustomToolCallOutput");
            println!("    GhostSnapshot");
        }
    }
    Ok(())
}

fn display_claude_schema(format: &str) -> Result<()> {
    match format {
        "rust" => {
            println!("{}", include_str!("../../providers/claude/schema.rs"));
        }
        "json" => {
            println!("JSON Schema output not yet implemented");
        }
        _ => {
            println!("{}", "Provider: Claude Code".bright_blue().bold());
            println!("Schema version: v2.0.28+");
            println!();
            println!("{}", "Root structure (JSONL - one record per line):".bold());
            println!("  ClaudeRecord (enum):");
            println!("    - User (user message with content)");
            println!("    - Assistant (assistant response)");
            println!("    - FileHistorySnapshot");
            println!();
            println!("{}", "User record:".bold());
            println!("  uuid: String (event_id)");
            println!("  sessionId: String");
            println!("  timestamp: String");
            println!("  isMeta: bool");
            println!("  parentUuid: String (optional)");
            println!("  message:");
            println!("    role: \"user\"");
            println!("    content: [UserContent]");
            println!();
            println!("{}", "UserContent (enum):".bold());
            println!("  - Text {{ text: String }}");
            println!("  - Image {{ source: Value }}");
            println!("  - ToolResult {{ tool_use_id: String, content: Value }}");
            println!();
            println!("{}", "Assistant record:".bold());
            println!("  uuid: String");
            println!("  sessionId: String");
            println!("  timestamp: String");
            println!("  message:");
            println!("    id: String");
            println!("    role: \"assistant\"");
            println!("    model: String");
            println!("    content: [AssistantContent]");
            println!("    usage: TokenUsage (optional)");
            println!();
            println!("{}", "AssistantContent (enum):".bold());
            println!("  - Text {{ text: String }}");
            println!("  - Thinking {{ thinking: String }}");
            println!("  - ToolUse {{ id: String, name: String, input: Object }}");
        }
    }
    Ok(())
}

fn display_gemini_schema(format: &str) -> Result<()> {
    match format {
        "rust" => {
            println!("{}", include_str!("../../providers/gemini/schema.rs"));
        }
        "json" => {
            println!("JSON Schema output not yet implemented");
        }
        _ => {
            println!("{}", "Provider: Gemini".bright_blue().bold());
            println!("Schema version: unknown");
            println!();
            println!(
                "{}",
                "Root structure (JSON - single session object):".bold()
            );
            println!("  GeminiSession:");
            println!("    sessionId: String");
            println!("    projectHash: String");
            println!("    startTime: String");
            println!("    lastUpdated: String");
            println!("    messages: [GeminiMessage]");
            println!();
            println!("{}", "GeminiMessage (enum):".bold());
            println!("  - User:");
            println!("      id: String");
            println!("      timestamp: String");
            println!("      content: String");
            println!("  - Gemini:");
            println!("      id: String");
            println!("      timestamp: String");
            println!("      content: String");
            println!("      model: String");
            println!("      thoughts: [Thought]");
            println!("      toolCalls: [ToolCall]");
            println!("      tokens: TokenUsage");
            println!("  - Info:");
            println!("      id: String");
            println!("      timestamp: String");
            println!("      content: String");
            println!();
            println!("{}", "Note:".yellow().bold());
            println!("  Some Gemini files may contain an array of messages directly");
            println!("  instead of a session object. This format is not yet supported.");
        }
    }
    Ok(())
}
