// Diagnostics module - File validation and health checking
// Pure business logic for validating log file parsing

pub mod validator;

pub use validator::{DiagnoseResult, FailureExample, FailureType, categorize_parse_error};
