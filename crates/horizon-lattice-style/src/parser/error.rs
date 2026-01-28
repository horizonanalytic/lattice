//! CSS parsing errors.

/// CSS parse error with location information.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// The error message describing what went wrong.
    pub message: String,
    /// Line number where the error occurred (1-indexed).
    pub line: u32,
    /// Column number where the error occurred (1-indexed).
    pub column: u32,
}

impl ParseError {
    /// Create a new parse error with the given message and location.
    pub fn new(message: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            message: message.into(),
            line,
            column,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CSS parse error at {}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}
