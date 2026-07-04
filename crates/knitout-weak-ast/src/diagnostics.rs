use std::fmt;

use knitout_ast::Span;

/// A structured parse error produced during CST lowering or weak→strong conversion.
///
/// Variants carry only byte-offset spans into the original source; callers slice
/// the source bytes if they need the raw text for display or fixup.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParseError {
    // -----------------------------------------------------------------------
    // File-level
    // -----------------------------------------------------------------------
    /// The root node itself was an ERROR node.
    UnparsedFile(Span),
    /// The `;!knitout-VERSION` magic line is absent.
    MissingMagic(Span),
    /// The magic line was syntactically malformed.
    InvalidMagic(Span),
    /// The magic line is present but has no version number.
    MissingVersion(Span),
    /// The magic line's version token could not be parsed as a positive integer.
    InvalidVersion(Span),

    // -----------------------------------------------------------------------
    // Field-level
    // -----------------------------------------------------------------------
    /// A required grammar field was absent or produced a MISSING node.
    MissingField {
        span: Span,
        field: &'static str,
    },
    /// A required grammar field was an ERROR node.
    InvalidField {
        span: Span,
        field: &'static str,
    },

    // -----------------------------------------------------------------------
    // Unrecognized leaf enum values
    // -----------------------------------------------------------------------
    UnrecognizedBed(Span),
    UnrecognizedDirection(Span),
    UnrecognizedPresserMode(Span),
    UnrecognizedPositionValue(Span),
    UnrecognizedXferStyle(Span),

    // -----------------------------------------------------------------------
    // Malformed numeric literals
    // -----------------------------------------------------------------------
    /// A token that should be a non-negative integer could not be parsed.
    InvalidInteger(Span),
    /// A token that should be a decimal number could not be parsed.
    InvalidRational(Span),

    // -----------------------------------------------------------------------
    // Structural errors
    // -----------------------------------------------------------------------
    InvalidHeader(Span),
    UnrecognizedHeader(Span),
    InvalidCarrier(Span),
    InvalidCarrierOrRail(Span),
    InvalidOperation(Span),
}

impl ParseError {
    /// The source span this error points to.
    pub fn span(&self) -> Span {
        match self {
            Self::UnparsedFile(s)
            | Self::MissingMagic(s)
            | Self::InvalidMagic(s)
            | Self::MissingVersion(s)
            | Self::InvalidVersion(s)
            | Self::UnrecognizedBed(s)
            | Self::UnrecognizedDirection(s)
            | Self::UnrecognizedPresserMode(s)
            | Self::UnrecognizedPositionValue(s)
            | Self::UnrecognizedXferStyle(s)
            | Self::InvalidInteger(s)
            | Self::InvalidRational(s)
            | Self::InvalidHeader(s)
            | Self::UnrecognizedHeader(s)
            | Self::InvalidCarrier(s)
            | Self::InvalidCarrierOrRail(s)
            | Self::InvalidOperation(s) => *s,
            Self::MissingField { span, .. } | Self::InvalidField { span, .. } => *span,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnparsedFile(_) => write!(f, "file could not be parsed"),
            Self::MissingMagic(_) => write!(f, "missing `;!knitout-VERSION` magic line"),
            Self::InvalidMagic(_) => write!(f, "malformed magic line"),
            Self::MissingVersion(_) => write!(f, "missing version number in magic line"),
            Self::InvalidVersion(_) => write!(f, "invalid version number in magic line"),
            Self::MissingField { field, .. } => write!(f, "missing required field `{field}`"),
            Self::InvalidField { field, .. } => write!(f, "invalid `{field}`"),
            Self::UnrecognizedBed(_) => write!(f, "unrecognized bed; expected 'f' or 'b'"),
            Self::UnrecognizedDirection(_) => {
                write!(f, "unrecognized direction; expected '+' or '-'")
            }
            Self::UnrecognizedPresserMode(_) => {
                write!(
                    f,
                    "unrecognized presser mode; expected 'off', 'auto', or 'on'"
                )
            }
            Self::UnrecognizedPositionValue(_) => write!(
                f,
                "unrecognized position value; expected 'Left', 'Center', 'Right', or 'Keep'"
            ),
            Self::UnrecognizedXferStyle(_) => {
                write!(
                    f,
                    "unrecognized xfer style; expected 'four-pass' or 'two-pass'"
                )
            }
            Self::InvalidInteger(_) => write!(f, "invalid non-negative integer"),
            Self::InvalidRational(_) => write!(f, "invalid number"),
            Self::InvalidHeader(_) => write!(f, "invalid header"),
            Self::UnrecognizedHeader(_) => write!(f, "unrecognized header"),
            Self::InvalidCarrier(_) => write!(f, "invalid carrier"),
            Self::InvalidCarrierOrRail(_) => write!(f, "invalid carrier or rail"),
            Self::InvalidOperation(_) => write!(f, "invalid operation"),
        }
    }
}
