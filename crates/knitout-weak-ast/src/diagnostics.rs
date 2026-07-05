use knitout_ast::Span;

/// A non-fatal observation about the knitout source. The program is still representable as
/// `Program<Parsed>` and may convert to `Program<Validated>`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ParseWarning {
    /// A `;!knitout-N` line appeared but was not the first non-blank line in the file.
    MagicNotFirst(Span),
    /// A header was defined more than once (if not conflicting, not an error; raised even if the
    /// values are conflicting too).
    DuplicateHeader {
        /// Span of the header name+colon in the most recent occurrence (e.g., "Gauge: 15").
        span: Span,
        /// Span of the header name+colon in the second-to-last occurrence.
        previous: Span,
    },
    /// A `;;Header:` line appeared after at least one operation line.
    HeaderAfterOperations(Span),
    /// An unrecognized extension header `;;X-foo: ...` that does not match any known header.
    UnrecognizedExtensionHeader(Span),
}

/// An unrecoverable parse error: the `Program<Parsed>` is still produced, but it will contain
/// `Other(Span)` nodes in places that could not be resolved, and `parsed_to_valid` will return
/// `None`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ParseError {
    /// No `;!knitout-N` magic line was found anywhere in the file.
    MagicMissing,
    /// The version number after `knitout-` could not be parsed as a non-negative integer.
    BadMagicVersion(Span),
    /// Conflicting magic versions
    ConflictingMagicVersion {
        /// Span of the magic version in the second (conflicting) occurrence.
        span: Span,
        /// Span of the magic version in the first occurrence.
        previous: Span,
    },
    /// A header was defined twice with different values.
    ConflictingHeader {
        /// Span of the header name+colon in the second (conflicting) occurrence.
        span: Span,
        /// Span of the header name+colon in the first occurrence.
        previous: Span,
    },
    /// The required `Carriers:` header was not found.
    MissingCarriersHeader,
    /// The opcode token is not a recognised base or extension operation.
    UnrecognizedOpcode(Span),
    /// A required positional argument was missing (points to the end of what was present).
    MissingArgument(Span),
    /// A direction token was not `+` or `-`.
    BadDirection(Span),
    /// A token that was expected to be a needle (`[fb]s?-?[0-9]+`) could not be parsed.
    BadNeedle(Span),
    /// The index portion of a needle (the integer part) could not be parsed as `i32`.
    BadNeedleIndex(Span),
    /// The bed character of a needle was not `f` or `b`.
    BadBed(Span),
    /// A token that should be a `u32` could not be parsed.
    BadU32(Span),
    /// A token that should be a decimal number could not be parsed.
    BadBigDecimal(Span),
    /// A presser-mode token was not `off`, `auto`, or `on`.
    BadPresserMode(Span),
    /// A transfer-style token was not `four-pass` or `two-pass`.
    BadXferStyle(Span),
    /// A Position header value was not `Left`, `Center`, `Right`, or `Keep`.
    BadPositionValue(Span),
    /// An argument was present where none was expected.
    TooManyArguments(Span),
    /// Couldn't read the source as UTF-8.
    BadUtf8(Span),
    /// Illegal carrier name (empty, whitespace, semicolon, comma)
    BadCarrierName(Span),
}
