//! Source-located semantic diagnostics, independent of the CLI and C backend.
use crate::syntax::token::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn new(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        Self {
            code,
            span,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error[{}] at line {}, col {}: {}",
            self.code, self.span.line, self.span.col, self.message
        )
    }
}
impl std::error::Error for Diagnostic {}
