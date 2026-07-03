#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Span {
    start: u32,
    end: u32,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Provenance {
    /// A span in the source code of the program, if the program was parsed from text.
    span: Option<Span>,
    /// A ";!source: " comment's body, or optionally some string provided by a high-level DSL that
    /// writes knitout AST directly.
    source: Option<String>,
}
