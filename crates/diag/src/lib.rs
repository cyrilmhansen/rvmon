#![forbid(unsafe_code)]

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Severity { Error, Warning, Info }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, severity: Severity::Error, message: message.into() }
    }
}

pub type Result<T> = std::result::Result<T, Diagnostic>;
