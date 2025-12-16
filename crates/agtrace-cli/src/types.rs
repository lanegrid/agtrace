use clap::ValueEnum;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum OutputFormat {
    Plain,
    Json,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Plain => write!(f, "plain"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum ViewStyle {
    Timeline,
    Compact,
}

impl fmt::Display for ViewStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewStyle::Timeline => write!(f, "timeline"),
            ViewStyle::Compact => write!(f, "compact"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum PackTemplate {
    Compact,
    Diagnose,
    Tools,
}

impl fmt::Display for PackTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackTemplate::Compact => write!(f, "compact"),
            PackTemplate::Diagnose => write!(f, "diagnose"),
            PackTemplate::Tools => write!(f, "tools"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum ExportFormat {
    Jsonl,
    Text,
}

impl fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportFormat::Jsonl => write!(f, "jsonl"),
            ExportFormat::Text => write!(f, "text"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum ExportStrategy {
    Raw,
    Clean,
    Reasoning,
}

impl fmt::Display for ExportStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportStrategy::Raw => write!(f, "raw"),
            ExportStrategy::Clean => write!(f, "clean"),
            ExportStrategy::Reasoning => write!(f, "reasoning"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum ProviderName {
    ClaudeCode,
    Codex,
    Gemini,
}

impl fmt::Display for ProviderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderName::ClaudeCode => write!(f, "claude_code"),
            ProviderName::Codex => write!(f, "codex"),
            ProviderName::Gemini => write!(f, "gemini"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum ProviderFilter {
    ClaudeCode,
    Codex,
    Gemini,
    All,
}

impl fmt::Display for ProviderFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderFilter::ClaudeCode => write!(f, "claude_code"),
            ProviderFilter::Codex => write!(f, "codex"),
            ProviderFilter::Gemini => write!(f, "gemini"),
            ProviderFilter::All => write!(f, "all"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum InspectFormat {
    Raw,
    Json,
}

impl fmt::Display for InspectFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InspectFormat::Raw => write!(f, "raw"),
            InspectFormat::Json => write!(f, "json"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum SchemaFormat {
    Text,
    Json,
    Rust,
}

impl fmt::Display for SchemaFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaFormat::Text => write!(f, "text"),
            SchemaFormat::Json => write!(f, "json"),
            SchemaFormat::Rust => write!(f, "rust"),
        }
    }
}
