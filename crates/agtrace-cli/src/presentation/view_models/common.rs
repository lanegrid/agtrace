use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    Minimal,
    #[default]
    Compact,
    Standard,
    Verbose,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl From<crate::args::OutputFormat> for OutputFormat {
    fn from(format: crate::args::OutputFormat) -> Self {
        match format {
            crate::args::OutputFormat::Plain => Self::Text,
            crate::args::OutputFormat::Json => Self::Json,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusBadge {
    pub level: StatusLevel,
    pub label: String,
}

impl StatusBadge {
    pub fn success(label: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Success,
            label: label.into(),
        }
    }

    pub fn info(label: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Info,
            label: label.into(),
        }
    }

    pub fn warning(label: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Warning,
            label: label.into(),
        }
    }

    pub fn error(label: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Error,
            label: label.into(),
        }
    }

    pub fn icon(&self) -> &str {
        match self.level {
            StatusLevel::Success => "✅",
            StatusLevel::Info => "ℹ️",
            StatusLevel::Warning => "⚠️",
            StatusLevel::Error => "❌",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StatusLevel {
    Success,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Guidance {
    pub description: String,
    pub command: Option<String>,
}

impl Guidance {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            command: None,
        }
    }

    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }
}
