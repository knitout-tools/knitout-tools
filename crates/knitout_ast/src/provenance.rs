/// Byte offsets into source code
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Span {
    start: u32,
    end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> u32 {
        self.start
    }

    pub fn end(&self) -> u32 {
        self.end
    }
}

/// A record of where a program's operation came from
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Provenance {
    /// A span in the source code of the program, if the program was parsed from text.
    span: Option<Span>,
    /// A ";!source: " comment's body, or optionally some string provided by a high-level DSL that
    /// writes knitout AST directly.
    source: Option<String>,
}

impl Provenance {
    pub fn new(span: Option<Span>, source: Option<String>) -> Self {
        Self { span, source }
    }

    pub fn span(&self) -> Option<Span> {
        self.span
    }

    pub fn source(&self) -> Option<&String> {
        self.source.as_ref()
    }
}
