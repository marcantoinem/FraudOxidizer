use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseCsvError {
    #[error("failed to read csv: {0}")]
    Io(#[from] std::io::Error),
    #[error("csv is missing a header row")]
    MissingHeader,
    #[error("line {line} has {found} columns, expected {expected}")]
    InvalidColumnCount {
        expected: usize,
        found: usize,
        line: usize,
    },
    #[error("field {field} must start with {expected_prefix}, got {value}")]
    InvalidPrefix {
        field: &'static str,
        expected_prefix: &'static str,
        value: String,
    },
    #[error("field {field} must contain a valid number, got {value}")]
    InvalidNumber { field: &'static str, value: String },
    #[error("field {field} must contain a valid float, got {value}")]
    InvalidFloat { field: &'static str, value: String },
    #[error("field {field} has unsupported value {value}")]
    InvalidEnumValue { field: &'static str, value: String },
}
