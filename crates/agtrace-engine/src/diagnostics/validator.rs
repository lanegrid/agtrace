use std::collections::HashMap;

#[derive(Debug)]
pub struct DiagnoseResult {
    pub provider_name: String,
    pub total_files: usize,
    pub successful: usize,
    pub failures: HashMap<FailureType, Vec<FailureExample>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FailureType {
    MissingField(String),
    TypeMismatch(String),
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

#[derive(Debug, Clone)]
pub struct FailureExample {
    pub path: String,
    pub reason: String,
}

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
