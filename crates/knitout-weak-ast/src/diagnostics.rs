use knitout_ast::Span;

/// A parse-time problem detected during CST lowering or weak→strong conversion.
/// Spans are byte offsets into the original source.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseDiagnostic {
    pub span: Span,
    pub message: String,
}

impl ParseDiagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}
