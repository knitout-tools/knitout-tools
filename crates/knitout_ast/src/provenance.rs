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
    span: Option<Span>,
    source: Option<String>,
}

impl Provenance {
    pub fn new(span: Option<Span>, source: Option<String>) -> Self {
        Self { span, source }
    }

    pub fn span(&self) -> Option<Span> {
        self.span
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_new_stores_start_and_end() {
        let s = Span::new(3, 7);
        assert_eq!(s.start(), 3);
        assert_eq!(s.end(), 7);
    }

    #[test]
    fn spans_with_same_bounds_are_equal() {
        assert_eq!(Span::new(0, 5), Span::new(0, 5));
        assert_ne!(Span::new(0, 5), Span::new(0, 6));
    }

    #[test]
    fn provenance_default_has_no_span_or_source() {
        let p = Provenance::default();
        assert_eq!(p.span(), None);
        assert_eq!(p.source(), None);
    }

    #[test]
    fn provenance_new_round_trips_span_and_source() {
        let span = Span::new(10, 20);
        let p = Provenance::new(Some(span), Some("knit.js:53".to_string()));
        assert_eq!(p.span(), Some(span));
        assert_eq!(p.source(), Some("knit.js:53"));
    }

    #[test]
    fn provenance_allows_span_without_source_and_vice_versa() {
        let span_only = Provenance::new(Some(Span::new(0, 1)), None);
        assert_eq!(span_only.span(), Some(Span::new(0, 1)));
        assert_eq!(span_only.source(), None);

        let source_only = Provenance::new(None, Some("cast-on helper".to_string()));
        assert_eq!(source_only.span(), None);
        assert_eq!(source_only.source(), Some("cast-on helper"));
    }
}
