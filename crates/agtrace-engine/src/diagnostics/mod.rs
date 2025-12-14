// Diagnostics module - File validation and health checking
// Pure business logic for validating log file parsing

pub mod validator;

pub use validator::{categorize_parse_error, DiagnoseResult, FailureExample, FailureType};
