use std::collections::HashMap;

/// Result of diagnosing log file parsing health for a provider.
///
/// Contains success/failure statistics and categorized failure examples
/// for identifying systematic parsing issues.
#[derive(Debug)]
pub struct DiagnoseResult {
    /// Provider being diagnosed (claude, codex, gemini).
    pub provider_name: String,
    /// Total number of log files checked.
    pub total_files: usize,
    /// Number of files successfully parsed.
    pub successful: usize,
    /// Failed files grouped by failure type.
    pub failures: HashMap<FailureType, Vec<FailureExample>>,
}

/// Category of log file parsing failure.
///
/// Used to group similar failures together for easier diagnosis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FailureType {
    /// Required field is missing from the log entry.
    MissingField(String),
    /// Field has unexpected type or format.
    TypeMismatch(String),
    /// Generic parsing error not categorized further.
    ParseError,
}

impl std::fmt::Display for FailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureType::MissingField(field) => write!(f, "missing_field ({})", field),
            FailureType::TypeMismatch(field) => write!(f, "type_mismatch ({})", field),
            FailureType::ParseError => write!(f, "parse_error"),
        }
    }
}

/// Example of a specific file parsing failure.
///
/// Provides the file path and error reason for investigation.
#[derive(Debug, Clone)]
pub struct FailureExample {
    /// Absolute path to the file that failed to parse.
    pub path: String,
    /// Human-readable description of the failure.
    pub reason: String,
}

/// Categorize a parse error message into a structured failure type.
///
/// Analyzes error text to extract field names and classify the failure
/// as missing field, type mismatch, or generic parse error.
pub fn categorize_parse_error(error_msg: &str) -> (FailureType, String) {
    if error_msg.contains("missing field") {
        if let Some(field) = extract_field_name(error_msg) {
            (
                FailureType::MissingField(field.clone()),
                format!("Missing required field: {}", field),
            )
        } else {
            (FailureType::ParseError, error_msg.to_string())
        }
    } else if error_msg.contains("expected") || error_msg.contains("invalid type") {
        if let Some(field) = extract_field_name(error_msg) {
            (
                FailureType::TypeMismatch(field.clone()),
                format!("Type mismatch for field: {}", field),
            )
        } else {
            (FailureType::ParseError, error_msg.to_string())
        }
    } else {
        (FailureType::ParseError, error_msg.to_string())
    }
}

fn extract_field_name(error_msg: &str) -> Option<String> {
    if let Some(start) = error_msg.find("field `") {
        let rest = &error_msg[start + 7..];
        if let Some(end) = rest.find('`') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_missing_field() {
        let error = "missing field `source` at line 1 column 2";
        let (failure_type, reason) = categorize_parse_error(error);
        assert_eq!(
            failure_type,
            FailureType::MissingField("source".to_string())
        );
        assert_eq!(reason, "Missing required field: source");
    }

    #[test]
    fn test_categorize_type_mismatch() {
        let error = "invalid type for field `timestamp`: expected string, got number";
        let (failure_type, reason) = categorize_parse_error(error);
        assert_eq!(
            failure_type,
            FailureType::TypeMismatch("timestamp".to_string())
        );
        assert_eq!(reason, "Type mismatch for field: timestamp");
    }

    #[test]
    fn test_categorize_generic_parse_error() {
        let error = "unexpected character at line 1";
        let (failure_type, _reason) = categorize_parse_error(error);
        assert_eq!(failure_type, FailureType::ParseError);
    }

    #[test]
    fn test_extract_field_name() {
        assert_eq!(
            extract_field_name("missing field `source`"),
            Some("source".to_string())
        );
        assert_eq!(
            extract_field_name("field `timestamp` has wrong type"),
            Some("timestamp".to_string())
        );
        assert_eq!(extract_field_name("no field here"), None);
    }
}
