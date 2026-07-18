use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub source: String,
    pub line: usize,
    pub column: usize,
}

impl Diagnostic {
    pub fn new(
        code: &'static str,
        message: impl Into<String>,
        source: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            source: source.into(),
            line,
            column,
        }
    }

    pub fn plain(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, message, "<artifact>", 1, 1)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}: {}: {}",
            self.source, self.line, self.column, self.code, self.message
        )
    }
}

impl std::error::Error for Diagnostic {}

pub type Result<T> = std::result::Result<T, Diagnostic>;
