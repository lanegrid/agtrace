use crate::types::SchemaFormat;

#[derive(Debug, Clone)]
pub struct ProviderSchemaDisplay {
    pub provider_name: String,
    pub schema_version: String,
    pub root_description: String,
    pub sections: Vec<SchemaSection>,
}

#[derive(Debug, Clone)]
pub struct SchemaSection {
    pub title: String,
    pub items: Vec<SchemaItem>,
}

#[derive(Debug, Clone)]
pub enum SchemaItem {
    Field { name: String, description: String },
    EnumVariant { name: String },
    Note { text: String, level: NoteLevel },
}

#[derive(Debug, Clone)]
pub enum NoteLevel {
    Info,
    Warning,
}

impl ProviderSchemaDisplay {
    pub fn for_codex() -> Self {
        Self {
            provider_name: "Codex".to_string(),
            schema_version: "v0.53-v0.63".to_string(),
            root_description: "JSONL - one record per line".to_string(),
            sections: vec![
                SchemaSection {
                    title: "Root structure".to_string(),
                    items: vec![
                        SchemaItem::Field {
                            name: "CodexRecord (enum)".to_string(),
                            description: String::new(),
                        },
                        SchemaItem::EnumVariant {
                            name: "SessionMeta".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "ResponseItem".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "EventMsg".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "TurnContext".to_string(),
                        },
                    ],
                },
                SchemaSection {
                    title: "SessionMeta".to_string(),
                    items: vec![
                        SchemaItem::Field {
                            name: "timestamp".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "payload".to_string(),
                            description: String::new(),
                        },
                        SchemaItem::Field {
                            name: "  id".to_string(),
                            description: "String (session_id)".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  cwd".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  originator".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  cli_version".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  source".to_string(),
                            description: "String | Object".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  model_provider".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  git".to_string(),
                            description: "GitInfo (optional)".to_string(),
                        },
                    ],
                },
                SchemaSection {
                    title: "TurnContext".to_string(),
                    items: vec![
                        SchemaItem::Field {
                            name: "timestamp".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "payload".to_string(),
                            description: String::new(),
                        },
                        SchemaItem::Field {
                            name: "  cwd".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  approval_policy".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  sandbox_policy".to_string(),
                            description: "SandboxPolicy".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  model".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  effort".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  summary".to_string(),
                            description: "String".to_string(),
                        },
                    ],
                },
                SchemaSection {
                    title: "SandboxPolicy (untagged enum)".to_string(),
                    items: vec![
                        SchemaItem::Note {
                            text: "New format (v0.63+):".to_string(),
                            level: NoteLevel::Info,
                        },
                        SchemaItem::Field {
                            name: "  { \"type\": \"read-only\" | \"workspace-write\" }".to_string(),
                            description: String::new(),
                        },
                        SchemaItem::Note {
                            text: "Old format (v0.53):".to_string(),
                            level: NoteLevel::Info,
                        },
                        SchemaItem::Field {
                            name: "  { \"mode\": \"...\", \"network_access\": bool, ... }"
                                .to_string(),
                            description: String::new(),
                        },
                    ],
                },
                SchemaSection {
                    title: "ResponseItem".to_string(),
                    items: vec![
                        SchemaItem::Field {
                            name: "timestamp".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "payload".to_string(),
                            description: String::new(),
                        },
                        SchemaItem::EnumVariant {
                            name: "Message (role: user/assistant)".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "FunctionCall".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "FunctionCallOutput".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "Reasoning".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "CustomToolCall".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "CustomToolCallOutput".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "GhostSnapshot".to_string(),
                        },
                    ],
                },
            ],
        }
    }

    pub fn for_claude() -> Self {
        Self {
            provider_name: "Claude Code".to_string(),
            schema_version: "v2.0.28+".to_string(),
            root_description: "JSONL - one record per line".to_string(),
            sections: vec![
                SchemaSection {
                    title: "Root structure".to_string(),
                    items: vec![
                        SchemaItem::Field {
                            name: "ClaudeRecord (enum)".to_string(),
                            description: String::new(),
                        },
                        SchemaItem::EnumVariant {
                            name: "User (user message with content)".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "Assistant (assistant response)".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "FileHistorySnapshot".to_string(),
                        },
                    ],
                },
                SchemaSection {
                    title: "User record".to_string(),
                    items: vec![
                        SchemaItem::Field {
                            name: "uuid".to_string(),
                            description: "String (event_id)".to_string(),
                        },
                        SchemaItem::Field {
                            name: "sessionId".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "timestamp".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "isMeta".to_string(),
                            description: "bool".to_string(),
                        },
                        SchemaItem::Field {
                            name: "parentUuid".to_string(),
                            description: "String (optional)".to_string(),
                        },
                        SchemaItem::Field {
                            name: "message".to_string(),
                            description: String::new(),
                        },
                        SchemaItem::Field {
                            name: "  role".to_string(),
                            description: "\"user\"".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  content".to_string(),
                            description: "[UserContent]".to_string(),
                        },
                    ],
                },
                SchemaSection {
                    title: "UserContent (enum)".to_string(),
                    items: vec![
                        SchemaItem::EnumVariant {
                            name: "Text { text: String }".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "Image { source: Value }".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "ToolResult { tool_use_id: String, content: Value }".to_string(),
                        },
                    ],
                },
                SchemaSection {
                    title: "Assistant record".to_string(),
                    items: vec![
                        SchemaItem::Field {
                            name: "uuid".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "sessionId".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "timestamp".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "message".to_string(),
                            description: String::new(),
                        },
                        SchemaItem::Field {
                            name: "  id".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  role".to_string(),
                            description: "\"assistant\"".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  model".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  content".to_string(),
                            description: "[AssistantContent]".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  usage".to_string(),
                            description: "TokenUsage (optional)".to_string(),
                        },
                    ],
                },
                SchemaSection {
                    title: "AssistantContent (enum)".to_string(),
                    items: vec![
                        SchemaItem::EnumVariant {
                            name: "Text { text: String }".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "Thinking { thinking: String }".to_string(),
                        },
                        SchemaItem::EnumVariant {
                            name: "ToolUse { id: String, name: String, input: Object }".to_string(),
                        },
                    ],
                },
            ],
        }
    }

    pub fn for_gemini() -> Self {
        Self {
            provider_name: "Gemini".to_string(),
            schema_version: "unknown".to_string(),
            root_description: "JSON - single session object".to_string(),
            sections: vec![
                SchemaSection {
                    title: "Root structure".to_string(),
                    items: vec![
                        SchemaItem::Field {
                            name: "GeminiSession".to_string(),
                            description: String::new(),
                        },
                        SchemaItem::Field {
                            name: "  sessionId".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  projectHash".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  startTime".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  lastUpdated".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "  messages".to_string(),
                            description: "[GeminiMessage]".to_string(),
                        },
                    ],
                },
                SchemaSection {
                    title: "GeminiMessage (enum)".to_string(),
                    items: vec![
                        SchemaItem::Note {
                            text: "User:".to_string(),
                            level: NoteLevel::Info,
                        },
                        SchemaItem::Field {
                            name: "    id".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "    timestamp".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "    content".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Note {
                            text: "Gemini:".to_string(),
                            level: NoteLevel::Info,
                        },
                        SchemaItem::Field {
                            name: "    id".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "    timestamp".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "    content".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "    model".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "    thoughts".to_string(),
                            description: "[Thought]".to_string(),
                        },
                        SchemaItem::Field {
                            name: "    toolCalls".to_string(),
                            description: "[ToolCall]".to_string(),
                        },
                        SchemaItem::Field {
                            name: "    tokens".to_string(),
                            description: "TokenUsage".to_string(),
                        },
                        SchemaItem::Note {
                            text: "Info:".to_string(),
                            level: NoteLevel::Info,
                        },
                        SchemaItem::Field {
                            name: "    id".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "    timestamp".to_string(),
                            description: "String".to_string(),
                        },
                        SchemaItem::Field {
                            name: "    content".to_string(),
                            description: "String".to_string(),
                        },
                    ],
                },
                SchemaSection {
                    title: "Note".to_string(),
                    items: vec![SchemaItem::Note {
                        text: "Some Gemini files may contain an array of messages directly instead of a session object. This format is not yet supported.".to_string(),
                        level: NoteLevel::Warning,
                    }],
                },
            ],
        }
    }
}

pub enum ProviderSchemaContent {
    Display(ProviderSchemaDisplay),
    RustSource(String),
    JsonSchema(String),
}

impl ProviderSchemaContent {
    pub fn for_provider(provider: &str, format: SchemaFormat) -> anyhow::Result<Self> {
        match format {
            SchemaFormat::Rust => {
                let source = match provider {
                    "claude" => include_str!("../../../agtrace-providers/src/claude/schema.rs"),
                    "codex" => include_str!("../../../agtrace-providers/src/codex/schema.rs"),
                    "gemini" => include_str!("../../../agtrace-providers/src/gemini/schema.rs"),
                    _ => anyhow::bail!("Unknown provider: {}", provider),
                };
                Ok(Self::RustSource(source.to_string()))
            }
            SchemaFormat::Json => Ok(Self::JsonSchema(
                "JSON Schema output not yet implemented".to_string(),
            )),
            SchemaFormat::Text => {
                let display = match provider {
                    "claude" => ProviderSchemaDisplay::for_claude(),
                    "codex" => ProviderSchemaDisplay::for_codex(),
                    "gemini" => ProviderSchemaDisplay::for_gemini(),
                    _ => anyhow::bail!("Unknown provider: {}", provider),
                };
                Ok(Self::Display(display))
            }
        }
    }
}
