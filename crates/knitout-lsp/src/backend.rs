//! The knitout language server.
//!
//! Maintains, for every open document, its full text and a tree-sitter parse tree. On open and on
//! change it re-parses the document, runs `knitout-weak-ast` over the resulting CST, discards the
//! `Program<Parsed>`, and publishes the collected warnings and errors as LSP diagnostics.

use std::sync::Mutex;

use dashmap::DashMap;
use knitout_weak_ast::{ParseError, ParseWarning, Span, cst_to_program};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tree_sitter::{Parser, Tree};

use crate::messages::{error_message, warning_message};

/// A single open document: its current text and the parse tree for that text.
struct Document {
    text: String,
    tree: Tree,
}

pub struct Backend {
    client: Client,
    /// Open documents keyed by URI.
    documents: DashMap<Url, Document>,
    /// The tree-sitter parser, reused across parses. `Parser` is not `Sync`, so it lives behind a
    /// mutex; parsing is fast and never held across an `await`.
    parser: Mutex<Parser>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_knitout::LANGUAGE.into())
            .expect("failed to load knitout tree-sitter grammar");
        Self {
            client,
            documents: DashMap::new(),
            parser: Mutex::new(parser),
        }
    }

    /// Parse `text` from scratch, returning the fresh parse tree.
    fn parse(&self, text: &str) -> Tree {
        let mut parser = self.parser.lock().expect("parser mutex poisoned");
        parser
            .parse(text, None)
            .expect("tree-sitter parse returned None (language unset or cancelled)")
    }

    /// Run the parser + weak-AST walker over a document and publish the resulting diagnostics.
    async fn analyze_and_publish(&self, uri: Url, version: Option<i32>) {
        let diagnostics = match self.documents.get(&uri) {
            Some(doc) => compute_diagnostics(&doc.text, &doc.tree),
            None => return,
        };
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }
}

/// Walk the CST with `knitout-weak-ast`, discard the `Program<Parsed>`, and convert the collected
/// warnings and errors into LSP diagnostics.
fn compute_diagnostics(text: &str, tree: &Tree) -> Vec<Diagnostic> {
    let (_program, warnings, errors) = cst_to_program(text.as_bytes(), tree);

    let mut diagnostics = Vec::with_capacity(warnings.len() + errors.len());
    for warning in &warnings {
        diagnostics.push(Diagnostic {
            range: span_to_range(text, warning_span(warning)),
            severity: Some(DiagnosticSeverity::WARNING),
            source: Some("knitout".to_string()),
            message: warning_message(warning),
            ..Default::default()
        });
    }
    for error in &errors {
        diagnostics.push(Diagnostic {
            range: span_to_range(text, error_span(error)),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("knitout".to_string()),
            message: error_message(error),
            ..Default::default()
        });
    }
    diagnostics
}

/// The primary span to anchor a warning's diagnostic on. Variants without a span anchor at the
/// start of the file.
fn warning_span(warning: &ParseWarning) -> Option<Span> {
    match warning {
        ParseWarning::MagicNotFirst(span)
        | ParseWarning::HeaderAfterOperations(span)
        | ParseWarning::UnrecognizedExtensionHeader(span)
        | ParseWarning::UnrecognizedExtension(span) => Some(*span),
        ParseWarning::DuplicateHeader { span, .. } => Some(*span),
    }
}

/// The primary span to anchor an error's diagnostic on. Variants without a span anchor at the
/// start of the file.
fn error_span(error: &ParseError) -> Option<Span> {
    match error {
        ParseError::MagicMissing | ParseError::MissingCarriersHeader => None,
        ParseError::BadMagicVersion(span)
        | ParseError::UnrecognizedOpcode(span)
        | ParseError::MissingArgument(span)
        | ParseError::BadDirection(span)
        | ParseError::BadNeedle(span)
        | ParseError::BadNeedleIndex(span)
        | ParseError::BadBed(span)
        | ParseError::BadU32(span)
        | ParseError::BadBigDecimal(span)
        | ParseError::BadPresserMode(span)
        | ParseError::BadXferStyle(span)
        | ParseError::BadPositionValue(span)
        | ParseError::TooManyArguments(span)
        | ParseError::BadUtf8(span)
        | ParseError::BadCarrierName(span) => Some(*span),
        ParseError::ConflictingMagicVersion { span, .. }
        | ParseError::ConflictingHeader { span, .. } => Some(*span),
    }
}

/// Convert a byte span into an LSP range. `None` (a diagnostic with no source location) collapses
/// to the very start of the document.
fn span_to_range(text: &str, span: Option<Span>) -> Range {
    match span {
        Some(span) => Range {
            start: offset_to_position(text, span.start()),
            end: offset_to_position(text, span.end()),
        },
        None => Range {
            start: Position::new(0, 0),
            end: Position::new(0, 0),
        },
    }
}

/// Convert a byte offset into an LSP [`Position`] (zero-based line, UTF-16 character offset).
fn offset_to_position(text: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    let mut idx = 0usize;
    for ch in text.chars() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += ch.len_utf16() as u32;
        }
        idx += ch.len_utf8();
    }
    Position { line, character }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // Full-document sync: each change carries the whole document, which we re-parse.
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "knitout-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "knitout language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let tree = self.parse(&text);
        self.documents.insert(uri.clone(), Document { text, tree });
        self.analyze_and_publish(uri, Some(params.text_document.version))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // With FULL sync there is exactly one change carrying the entire new document text.
        let Some(change) = params.content_changes.into_iter().next_back() else {
            return;
        };
        let text = change.text;
        let tree = self.parse(&text);
        self.documents.insert(uri.clone(), Document { text, tree });
        self.analyze_and_publish(uri, Some(params.text_document.version))
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        // Clear any diagnostics we published for this document.
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_knitout::LANGUAGE.into())
            .unwrap();
        parser.parse(text, None).unwrap()
    }

    #[test]
    fn offset_to_position_handles_lines_and_utf16() {
        let text = "ab\ncd\n";
        assert_eq!(offset_to_position(text, 0), Position::new(0, 0));
        assert_eq!(offset_to_position(text, 1), Position::new(0, 1));
        // Byte 3 is the start of the second line ("cd").
        assert_eq!(offset_to_position(text, 3), Position::new(1, 0));
        assert_eq!(offset_to_position(text, 4), Position::new(1, 1));

        // A non-BMP scalar (👍, 4 UTF-8 bytes, 2 UTF-16 code units) advances the character column
        // by 2, and the following byte offset maps past it.
        let emoji = "👍x";
        assert_eq!(offset_to_position(emoji, 0), Position::new(0, 0));
        assert_eq!(offset_to_position(emoji, 4), Position::new(0, 2));
    }

    #[test]
    fn span_to_range_maps_none_to_origin() {
        let range = span_to_range("anything", None);
        assert_eq!(range.start, Position::new(0, 0));
        assert_eq!(range.end, Position::new(0, 0));
    }

    #[test]
    fn span_to_range_maps_bytes_to_positions() {
        let text = "knit f1\n";
        let range = span_to_range(text, Some(Span::new(5, 7)));
        assert_eq!(range.start, Position::new(0, 5));
        assert_eq!(range.end, Position::new(0, 7));
    }

    #[test]
    fn valid_program_has_no_diagnostics() {
        let text = ";!knitout-2\n;;Carriers: 1 2 3\n";
        let tree = parse(text);
        assert!(compute_diagnostics(text, &tree).is_empty());
    }

    #[test]
    fn missing_headers_produce_error_diagnostics() {
        // No magic line and no Carriers header: two errors, both flagged as ERROR severity.
        let text = "knit + f1 1\n";
        let tree = parse(text);
        let diags = compute_diagnostics(text, &tree);
        assert!(
            diags
                .iter()
                .all(|d| d.severity == Some(DiagnosticSeverity::ERROR)),
            "expected only errors, got {diags:?}"
        );
        assert!(
            diags.iter().any(|d| d.message.contains("version line")),
            "expected a missing-magic diagnostic, got {diags:?}"
        );
        assert!(
            diags.iter().any(|d| d.message.contains("Carriers")),
            "expected a missing-Carriers diagnostic, got {diags:?}"
        );
        assert!(diags.iter().all(|d| d.source.as_deref() == Some("knitout")));
    }

    #[test]
    fn magic_not_first_produces_warning() {
        // A header before the magic line makes the magic line "not first": a warning, not an error.
        let text = ";;Carriers: 1\n;!knitout-2\n";
        let tree = parse(text);
        let diags = compute_diagnostics(text, &tree);
        assert!(
            diags
                .iter()
                .any(|d| d.severity == Some(DiagnosticSeverity::WARNING)),
            "expected a warning diagnostic, got {diags:?}"
        );
    }
}
