//! CST → `Program<Parsed>` lowering.
//!
//! This module walks the tree-sitter CST for a knitout source file and produces a
//! `Program<Parsed>` where every AST node is annotated with a `Span` pointing to the
//! relevant bytes in the source buffer.  Parse errors and warnings are collected into
//! side-channel `Vec`s and returned alongside the program; they do not abort parsing.

use std::str::FromStr;

use bigdecimal::BigDecimal;
use either::Either;
use knitout_ast::{
    Annotated, BaseOperation, Bed, CarrierOrRail, CarriersHeaderValue, Direction,
    ExtensionOperation, Headers, IsSlider, Needle, Operation, PositionHeaderValue, PresserMode,
    Program, Rail, RailSide, Span, XferStyle,
};
use knitout_ast::{CarrierTable, MyBigDecimal, MyI32, MyString, MyU32};
use tree_sitter::{Node, Tree};

use crate::diagnostics::{ParseError, ParseWarning};
use crate::{CommentInfo, Parsed};

// ─── span helpers ────────────────────────────────────────────────────────────

fn node_span(node: Node<'_>) -> Span {
    Span::new(node.start_byte(), node.end_byte())
}

// ─── lowering context ────────────────────────────────────────────────────────

struct LowerCtx<'src> {
    source: &'src [u8],
    carrier_table: CarrierTable,
    warnings: Vec<ParseWarning>,
    errors: Vec<ParseError>,

    /// Span of the `MagicVersion` annotation if we have already seen the magic line.
    magic: Option<(u32, (Span, Span))>,
    /// Whether any operation line has been emitted yet.
    operations_started: bool,
    /// Span of the first operation, used to anchor missing-header diagnostics.
    first_op_span: Option<Span>,

    /// Tracked previously-seen header spans for duplicate detection. Key = canonical header name,
    /// value = span of last whole header declaration (name+colon+value).
    seen_headers: std::collections::HashMap<String, Span>,

    /// Accumulated operations.
    operations: Vec<Annotated<Operation<Parsed>, (Span, CommentInfo)>>,

    /// Partially-built headers struct (we can't finalise it until we know if `Carriers:` arrived).
    headers_partial: Headers<Parsed>,
}

impl<'src> LowerCtx<'src> {
    fn new(source: &'src [u8]) -> Self {
        Self {
            source,
            carrier_table: CarrierTable::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            magic: None,
            operations_started: false,
            first_op_span: None,
            seen_headers: std::collections::HashMap::new(),
            operations: Vec::new(),
            // Span::new(0, 0) is a dummy value
            headers_partial: Headers::new(Either::Right(Span::new(0, 0))),
        }
    }

    // ── finalise ─────────────────────────────────────────────────────────────

    fn finish(mut self) -> (Program<Parsed>, Vec<ParseWarning>, Vec<ParseError>) {
        // Resolve the version
        let version_either: Either<Annotated<u32, (Span, Span)>, Span> = match self.magic {
            Some((ver, ann)) => Either::Left(Annotated::new(ver, ann)),
            None => {
                self.errors.push(ParseError::MagicMissing);
                Either::Right(Span::new(0, 0))
            }
        };

        // Resolve the carriers header
        if self.headers_partial.carriers().is_right() {
            self.errors.push(ParseError::MissingCarriersHeader);
        }

        let prog = Program::new(
            version_either,
            self.headers_partial,
            self.operations,
            self.carrier_table,
        );

        (prog, self.warnings, self.errors)
    }

    /// Turns the span into utf8 text. If the span is not valid utf8, pushes a `BadUtf8` error and
    /// returns `None`.
    fn text_of(&mut self, span: Span) -> Option<&'src str> {
        match std::str::from_utf8(&self.source[span.start()..span.end()]) {
            Ok(s) => Some(s),
            Err(_) => {
                self.errors.push(ParseError::BadUtf8(span));
                None
            }
        }
    }

    fn make_carrier(&mut self, name: &str, span: Span) -> knitout_ast::Carrier<Parsed> {
        if name.is_empty() {
            self.errors.push(ParseError::BadCarrierName(span));
        }
        if name
            .chars()
            .any(|c| c.is_whitespace() || c == ';' || c == ',')
        {
            self.errors.push(ParseError::BadCarrierName(span));
        }
        self.carrier_table.make_carrier(name, span)
    }

    // ── header helpers ────────────────────────────────────────────────────────

    /// Check for duplicate headers. Returns `false` if this is the first occurrence, `true` if it
    /// is a duplicate (while pushing a warning). Span should be the whole header.
    fn header_already_defined(&mut self, name: &str, span: Span) -> bool {
        match self.seen_headers.insert(name.to_owned(), span) {
            None => false,
            Some(prev_span) => {
                // We treat any second definition as a conflict (different value → error,
                // same value → warning).  We record the error here; the caller may refine.
                self.warnings.push(ParseWarning::DuplicateHeader {
                    span,
                    previous: prev_span,
                });
                true
            }
        }
    }

    /// Pushes a warning if operations have been seen. Span should be the whole header.
    fn check_header_after_ops(&mut self, span: Span) {
        if self.operations_started {
            self.warnings
                .push(ParseWarning::HeaderAfterOperations(span));
        }
    }

    // ── line dispatchers ──────────────────────────────────────────────────────

    /// Keeps track of line_index for magic header. Increments line index after. Does not increment
    /// line index for empty lines. Increments on tag, header, comment, and op.
    fn lower_line(&mut self, node: Node<'_>, line_index: &mut usize) -> Option<()> {
        // The grammar's `line` rule is a transparent choice; the named child is the real node.
        let inner = if node.kind() == "line" {
            node.named_child(0)?
        } else {
            node
        };

        match inner.kind() {
            "tag_comment" => {
                self.lower_top_tag_comment(inner, *line_index);
            }
            "header" => {
                self.lower_header(inner);
            }
            "comment" => {} // plain unassociated comment, nothing to do
            "op_line" => {
                self.lower_op_line(inner);
            }
            _ => {}
        }
        *line_index += 1;
        Some(())
    }

    // ── magic / tag line ──────────────────────────────────────────────────────

    /// line_index is the number of preceding nonempty lines.
    fn lower_top_tag_comment(&mut self, node: Node<'_>, line_index: usize) -> Option<()> {
        let tag_span = node_span(node);
        let value_span = node_span(node.child_by_field_name("value")?);
        let value_text = self.text_of(value_span)?;

        // Is this the magic line?
        const MAGIC_PREFIX: &str = "knitout-";
        let is_magic = value_text.starts_with(MAGIC_PREFIX);
        if is_magic {
            // Span for ";!knitout-"
            let (prefix_span, ver_span) = tag_span.split(value_span.start() + MAGIC_PREFIX.len());

            if line_index > 0 {
                self.warnings.push(ParseWarning::MagicNotFirst(tag_span));
            }

            match self.text_of(ver_span)?.parse::<u32>() {
                Ok(ver) => {
                    match self.magic {
                        // Conflicting magic versions -> error
                        Some((prev_ver, (prev_prefix, prev_ver_span))) if prev_ver != ver => {
                            self.errors.push(ParseError::ConflictingMagicVersion {
                                span: prefix_span.union(&ver_span),
                                previous: prev_prefix.union(&prev_ver_span),
                            });
                        }
                        // Non-conflicting -> no action (already warned MagicNotFirst)
                        Some(_) => {}
                        // First magic: record it
                        None => {
                            self.magic = Some((ver, (prefix_span, ver_span)));
                        }
                    }
                }
                Err(_) => {
                    self.errors.push(ParseError::BadMagicVersion(ver_span));
                }
            };
        }
        // if not magic, just ignore it as a regular comment. At some point, LSP directives would be
        // handled here: ";!lsp-*" to disable certain warnings

        Some(())
    }

    // ── header parsing ────────────────────────────────────────────────────────

    fn lower_header(&mut self, node: Node<'_>) -> Option<()> {
        let header_span = node_span(node);

        // o.w. just ";;" with no content
        let content_node = node.child_by_field_name("content")?;

        let content_span = node_span(content_node);
        let content_text = self.text_of(content_span)?;
        let content_start = content_span.start();

        // Split at the first ':' to get name and value. If no colon, treat it as a comment.
        let colon_pos = content_text.find(':')?;

        let name_str = content_text[..colon_pos].trim();
        let value_str = &content_text[colon_pos + 1..];

        // Trim leading whitespace from value
        let value_trimmed = value_str.trim_start();
        let value_trimmed_offset = value_str.len() - value_trimmed.len();
        let value_trim_start = content_start + colon_pos + 1 + value_trimmed_offset;
        let value_trimmed = value_trimmed.trim_end();
        let value_span = Span::new(value_trim_start, value_trim_start + value_trimmed.len());

        self.check_header_after_ops(header_span);

        match name_str {
            "Carriers" => self.lower_carriers_header(header_span, value_trimmed, value_span),
            "Machine" => {
                self.lower_string_header("Machine", header_span, value_trimmed, value_span)
            }
            "Gauge" => self.lower_u32_header("Gauge", header_span, value_trimmed, value_span),
            "Width" => self.lower_u32_header("Width", header_span, value_trimmed, value_span),
            "Position" => self.lower_position_header(header_span, value_trimmed, value_span),
            s if s.starts_with("Yarn-") => {
                self.lower_yarn_header(header_span, s, value_trimmed, value_span)
            }
            s if s.len() >= 2 && s[..2].eq_ignore_ascii_case("X-") => {
                self.warnings
                    .push(ParseWarning::UnrecognizedExtensionHeader(header_span));
                self.headers_partial.add_unknown(
                    s.to_owned(),
                    value_trimmed.trim().to_owned(),
                    header_span,
                );
            }
            _ => {
                self.headers_partial.add_unknown(
                    name_str.to_owned(),
                    value_trimmed.trim().to_owned(),
                    header_span,
                );
            }
        };

        Some(())
    }

    fn lower_carriers_header(&mut self, header_span: Span, value: &str, value_span: Span) {
        if self.header_already_defined("Carriers", header_span) {
            return;
        }
        let chv = self.parse_carriers_header_value(value, value_span);
        self.headers_partial
            .set_carriers(Either::Left(Annotated::new(chv, header_span)));
    }

    fn lower_string_header(&mut self, key: &str, header_span: Span, value: &str, value_span: Span) {
        if self.header_already_defined(key, header_span) {
            return;
        }
        let my_string = MyString::new(Either::Left(value.trim().to_owned()), value_span);
        self.headers_partial
            .set_machine(Annotated::new(my_string, header_span));
    }

    fn lower_u32_header(&mut self, key: &str, header_span: Span, value: &str, value_span: Span) {
        if self.header_already_defined(key, header_span) {
            return;
        }
        let val_str = value.trim();
        let my_u32 = match val_str.parse::<u32>() {
            Ok(n) => MyU32::new(Either::Left(n), value_span),
            Err(_) => {
                self.errors.push(ParseError::BadU32(value_span));
                MyU32::new(Either::Right(value_span), value_span)
            }
        };
        if key == "Gauge" {
            self.headers_partial
                .set_gauge(Annotated::new(my_u32, header_span));
        } else {
            self.headers_partial
                .set_width(Annotated::new(my_u32, header_span));
        }
    }

    fn lower_position_header(&mut self, header_span: Span, value: &str, value_span: Span) {
        if self.header_already_defined("Position", header_span) {
            return;
        }
        let val_str = value.trim();
        let pos = match val_str {
            "Left" => PositionHeaderValue::Left(value_span),
            "Center" => PositionHeaderValue::Center(value_span),
            "Right" => PositionHeaderValue::Right(value_span),
            "Keep" => PositionHeaderValue::Keep(value_span),
            _ => {
                self.errors.push(ParseError::BadPositionValue(value_span));
                PositionHeaderValue::Other(value_span, value_span)
            }
        };
        self.headers_partial
            .set_position(Annotated::new(pos, header_span));
    }

    fn lower_yarn_header(
        &mut self,
        header_span: Span,
        name_str: &str,
        value: &str,
        value_span: Span,
    ) {
        // "Yarn-A" → carrier name is "A"
        let carrier_name = &name_str["Yarn-".len()..];
        let carrier = self.make_carrier(carrier_name, header_span);
        let my_string = MyString::new(Either::Left(value.trim().to_owned()), value_span);
        self.headers_partial
            .add_yarn(carrier, my_string, header_span);
    }

    // ── carriers header value parsing ─────────────────────────────────────────

    fn parse_carriers_header_value(
        &mut self,
        text: &str,
        span: Span,
    ) -> CarriersHeaderValue<Parsed> {
        let abs_start = span.start();
        let mut items: Vec<CarrierOrRail<Parsed>> = Vec::new();

        let mut token_start = None;

        // Push a space on the back to trigger final tokenization
        for (i, c) in text
            .char_indices()
            .chain(std::iter::once((text.len(), ' ')))
        {
            if c.is_whitespace() {
                match token_start {
                    None => continue, // still skipping whitespace
                    Some(start) => {
                        token_start = None; // reset
                        let token_span = Span::new(abs_start + start, abs_start + i);
                        let token = match self.text_of(token_span) {
                            Some(t) => t,
                            None => continue, // skip invalid utf8 token
                        };

                        if let Some(semi_pos) = token.find(';') {
                            // Rail: "left;right"
                            let (left_span, right_span) =
                                token_span.split_remove(token_span.start() + semi_pos);
                            let left_str = &token[..semi_pos];
                            let right_str = &token[semi_pos + 1..];

                            let left_side = self.parse_rail_side(left_str, left_span);
                            let right_side = self.parse_rail_side(right_str, right_span);

                            items.push(CarrierOrRail::Rail(Rail::new(
                                Either::Left(left_side),
                                Either::Left(right_side),
                                token_span,
                            )));
                        } else {
                            // Single carrier. We're sure the token is nonempty
                            let carrier = self.make_carrier(token, token_span);
                            items.push(CarrierOrRail::Carrier(carrier));
                        }
                    }
                }
            } else {
                if token_start.is_none() {
                    token_start = Some(i)
                }
            }
        }

        CarriersHeaderValue::new(Either::Left(items), span)
    }

    fn parse_rail_side(&mut self, text: &str, span: Span) -> RailSide<Parsed> {
        let mut carriers = Vec::new();
        let mut pos = 0usize;

        for part in text.split(",") {
            let part_start = span.start() + pos;
            let part_len = part.len();
            let part_span = Span::new(part_start, part_start + part_len);
            pos += part_len + 1; // +1 for ','

            if !part.is_empty() {
                let carrier = self.make_carrier(part, part_span);
                carriers.push(carrier);
            }
        }

        RailSide::new(carriers, span)
    }

    // ── operation line ────────────────────────────────────────────────────────

    fn lower_op_line(&mut self, node: Node<'_>) -> Option<()> {
        if !self.operations_started {
            self.operations_started = true;
            self.first_op_span = Some(node_span(node));
        }

        let op_node = node.child_by_field_name("operation")?;

        // Trailing comment
        let comment_info = if let Some(tag) = node.child_by_field_name("tag_comment") {
            let tag_span = node_span(tag);
            let is_source_tag = if let Some(v) = tag.child_by_field_name("value") {
                self.text_of(node_span(v))
                    .is_some_and(|t| t.starts_with("source: "))
            } else {
                false
            };
            if is_source_tag {
                CommentInfo::SourceTag(tag_span)
            } else {
                CommentInfo::UnknownTag(tag_span)
            }
        } else if let Some(comment) = node.child_by_field_name("comment") {
            CommentInfo::Comment(node_span(comment))
        } else {
            CommentInfo::NoComment
        };

        let op_span = node_span(op_node);

        let opcode_node = op_node.child_by_field_name("opcode")?;
        let opcode_span = node_span(opcode_node);
        let opcode_text = self.text_of(opcode_span)?;

        // Collect argument nodes
        let mut args: Vec<(Span, &'src str)> = Vec::new();
        let mut cursor = op_node.walk();
        for arg in op_node.children_by_field_name("argument", &mut cursor) {
            let arg_span = node_span(arg);
            if let Some(text) = self.text_of(arg_span) {
                args.push((arg_span, text));
            }
        }

        let op = self.parse_operation(opcode_text, opcode_span, &args);
        self.operations
            .push(Annotated::new(op, (op_span, comment_info)));
        Some(())
    }

    // ── argument helpers ──────────────────────────────────────────────────────

    fn parse_direction(&mut self, text: &str, span: Span) -> Direction<Parsed> {
        match text {
            "+" => Direction::Right(span),
            "-" => Direction::Left(span),
            _ => {
                self.errors.push(ParseError::BadDirection(span));
                Direction::Other(span, span)
            }
        }
    }

    fn parse_needle(&mut self, text: &str, span: Span) -> Either<Needle<Parsed>, Span> {
        if text.is_empty() {
            self.errors.push(ParseError::BadNeedle(span));
            return Either::Right(span);
        }

        let bed_span = Span::new(span.start(), span.start() + 1);
        let bed = match text.as_bytes()[0] {
            b'f' => Bed::Front(bed_span),
            b'b' => Bed::Back(bed_span),
            _ => {
                self.errors.push(ParseError::BadBed(bed_span));
                Bed::Other(bed_span, bed_span)
            }
        };

        let mut pos = 1usize;
        let is_slider = if text[pos..].starts_with('s') {
            let s_span = Span::new(span.start() + 1, span.start() + 2);
            pos += 1;
            IsSlider::Slider(s_span)
        } else {
            let empty = Span::new(span.start() + 1, span.start() + 1);
            IsSlider::NotSlider(empty)
        };

        let index_str = &text[pos..];
        let index_span = Span::new(span.start() + pos, span.end());

        match index_str.parse::<i32>() {
            Ok(idx) => {
                let needle = Needle::new(
                    bed,
                    is_slider,
                    MyI32::new(Either::Left(idx), index_span),
                    span,
                );
                Either::Left(needle)
            }
            Err(_) => {
                self.errors.push(ParseError::BadNeedleIndex(index_span));
                // Return a needle with Other index, keeping bed/slider for partial info.
                let needle = Needle::new(
                    bed,
                    is_slider,
                    MyI32::new(Either::Right(index_span), index_span),
                    span,
                );
                Either::Left(needle)
            }
        }
    }

    fn parse_big_decimal(&mut self, text: &str, span: Span) -> MyBigDecimal<Parsed> {
        match BigDecimal::from_str(text) {
            Ok(bd) => MyBigDecimal::new(Either::Left(bd), span),
            Err(_) => {
                self.errors.push(ParseError::BadBigDecimal(span));
                MyBigDecimal::new(Either::Right(span), span)
            }
        }
    }

    fn parse_u32(&mut self, text: &str, span: Span) -> MyU32<Parsed> {
        match text.parse::<u32>() {
            Ok(n) => MyU32::new(Either::Left(n), span),
            Err(_) => {
                self.errors.push(ParseError::BadU32(span));
                MyU32::new(Either::Right(span), span)
            }
        }
    }

    fn parse_my_string(&self, text: &str, span: Span) -> MyString<Parsed> {
        MyString::new(Either::Left(text.to_owned()), span)
    }

    fn parse_carrier(&mut self, text: &str, span: Span) -> knitout_ast::Carrier<Parsed> {
        self.make_carrier(text.trim(), span)
    }

    /// Consume `args[at]` as a direction.  If absent, emits `MissingArgument` and returns Other.
    fn take_direction(
        &mut self,
        args: &[(Span, &str)],
        at: usize,
        opcode_span: Span,
    ) -> Direction<Parsed> {
        match args.get(at) {
            Some(&(span, text)) => self.parse_direction(text, span),
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                Direction::Other(opcode_span, opcode_span)
            }
        }
    }

    /// Consume `args[at]` as a needle.  If absent, emits `MissingArgument` and returns Right.
    fn take_needle(
        &mut self,
        args: &[(Span, &str)],
        at: usize,
        opcode_span: Span,
    ) -> Either<Needle<Parsed>, Span> {
        match args.get(at) {
            Some(&(span, text)) => self.parse_needle(text, span),
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                Either::Right(opcode_span)
            }
        }
    }

    /// Consume `args[at]` as a BigDecimal.  If absent, emits `MissingArgument`.
    fn take_big_decimal(
        &mut self,
        args: &[(Span, &str)],
        at: usize,
        opcode_span: Span,
    ) -> MyBigDecimal<Parsed> {
        match args.get(at) {
            Some(&(span, text)) => self.parse_big_decimal(text, span),
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                MyBigDecimal::new(Either::Right(opcode_span), opcode_span)
            }
        }
    }

    /// Consume `args[at]` as a u32.  If absent, emits `MissingArgument`.
    fn take_u32(&mut self, args: &[(Span, &str)], at: usize, opcode_span: Span) -> MyU32<Parsed> {
        match args.get(at) {
            Some(&(span, text)) => self.parse_u32(text, span),
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                MyU32::new(Either::Right(opcode_span), opcode_span)
            }
        }
    }

    /// Consume `args[at]` as a string.  If absent, emits `MissingArgument`.
    fn take_string(
        &mut self,
        args: &[(Span, &str)],
        at: usize,
        opcode_span: Span,
    ) -> MyString<Parsed> {
        match args.get(at) {
            Some(&(span, text)) => self.parse_my_string(text, span),
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                MyString::new(Either::Right(opcode_span), opcode_span)
            }
        }
    }

    /// All args from `from` onwards as carriers.
    fn take_carriers(
        &mut self,
        args: &[(Span, &str)],
        from: usize,
    ) -> Vec<knitout_ast::Carrier<Parsed>> {
        args[from.min(args.len())..]
            .iter()
            .map(|&(span, text)| self.parse_carrier(text, span))
            .collect()
    }

    fn warn_extra_args(&mut self, args: &[(Span, &str)], from: usize) {
        for &(span, _) in &args[from.min(args.len())..] {
            self.errors.push(ParseError::TooManyArguments(span));
        }
    }

    // ── operation parsers ─────────────────────────────────────────────────────

    fn parse_operation(
        &mut self,
        opcode: &str,
        opcode_span: Span,
        args: &[(Span, &str)],
    ) -> Operation<Parsed> {
        match opcode {
            // ── carrier-list operations ───────────────────────────────────────
            "in" => Operation::BaseOperation(BaseOperation::In {
                carriers: self.take_carriers(args, 0),
            }),
            "inhook" => Operation::BaseOperation(BaseOperation::Inhook {
                carriers: self.take_carriers(args, 0),
            }),
            "releasehook" => Operation::BaseOperation(BaseOperation::Releasehook {
                carriers: self.take_carriers(args, 0),
            }),
            "out" => Operation::BaseOperation(BaseOperation::Out {
                carriers: self.take_carriers(args, 0),
            }),
            "outhook" => Operation::BaseOperation(BaseOperation::Outhook {
                carriers: self.take_carriers(args, 0),
            }),

            // ── stitch / rack ─────────────────────────────────────────────────
            "stitch" => {
                let before_pull = self.take_big_decimal(args, 0, opcode_span);
                let after_pull = self.take_big_decimal(args, 1, opcode_span);
                self.warn_extra_args(args, 2);
                Operation::BaseOperation(BaseOperation::Stitch {
                    before_pull,
                    after_pull,
                })
            }
            "rack" => {
                let offset = self.take_big_decimal(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                Operation::BaseOperation(BaseOperation::Rack { offset })
            }

            // ── direction + needle + carriers ─────────────────────────────────
            "knit" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let needle = self.take_needle(args, 1, opcode_span);
                let carriers = self.take_carriers(args, 2);
                Operation::BaseOperation(BaseOperation::Knit {
                    direction,
                    needle,
                    carriers,
                })
            }
            "tuck" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let needle = self.take_needle(args, 1, opcode_span);
                let carriers = self.take_carriers(args, 2);
                Operation::BaseOperation(BaseOperation::Tuck {
                    direction,
                    needle,
                    carriers,
                })
            }
            "miss" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let needle = self.take_needle(args, 1, opcode_span);
                let carriers = self.take_carriers(args, 2);
                Operation::BaseOperation(BaseOperation::Miss {
                    direction,
                    needle,
                    carriers,
                })
            }
            "split" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let source = self.take_needle(args, 1, opcode_span);
                let destination = self.take_needle(args, 2, opcode_span);
                let carriers = self.take_carriers(args, 3);
                Operation::BaseOperation(BaseOperation::Split {
                    direction,
                    source,
                    destination,
                    carriers,
                })
            }

            // ── no-arg / single-needle ────────────────────────────────────────
            "pause" => {
                self.warn_extra_args(args, 0);
                Operation::BaseOperation(BaseOperation::Pause)
            }
            "drop" => {
                let needle = self.take_needle(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                Operation::BaseOperation(BaseOperation::Drop { needle })
            }
            "amiss" => {
                let needle = self.take_needle(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                Operation::BaseOperation(BaseOperation::Amiss { needle })
            }
            "xfer" => {
                let source = self.take_needle(args, 0, opcode_span);
                let destination = self.take_needle(args, 1, opcode_span);
                self.warn_extra_args(args, 2);
                Operation::BaseOperation(BaseOperation::Xfer {
                    source,
                    destination,
                })
            }

            // ── extension operations ──────────────────────────────────────────
            s if s.starts_with("x-") => self.parse_extension_op(s, opcode_span, args),

            // ── unknown ───────────────────────────────────────────────────────
            _ => {
                self.errors
                    .push(ParseError::UnrecognizedOpcode(opcode_span));
                Operation::Other(opcode_span)
            }
        }
    }

    fn parse_extension_op(
        &mut self,
        opcode_str: &str,
        opcode_span: Span,
        args: &[(Span, &str)],
    ) -> Operation<Parsed> {
        let ext = match opcode_str {
            "x-stitch-number" => {
                let value = self.take_u32(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                ExtensionOperation::XStitchNumber { value }
            }
            "x-speed-number" => {
                let value = self.take_u32(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                ExtensionOperation::XSpeedNumber { value }
            }
            "x-sub-roller-number" => {
                let value = self.take_u32(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                ExtensionOperation::XSubRollerNumber { value }
            }
            "x-presser-mode" => {
                let mode = self.parse_presser_mode_arg(args, opcode_span);
                self.warn_extra_args(args, 1);
                ExtensionOperation::XPresserMode { mode }
            }
            "x-xfer-stitch-number" => {
                let value = self.take_u32(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                ExtensionOperation::XXferStitchNumber { value }
            }
            "x-end-pass" => {
                self.warn_extra_args(args, 0);
                ExtensionOperation::XEndPass
            }
            "x-vis-color" => {
                let color = self.take_string(args, 0, opcode_span);
                let carriers = self.take_carriers(args, 1);
                ExtensionOperation::XVisColor { color, carriers }
            }
            "x-loop-in" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let needle = self.take_needle(args, 1, opcode_span);
                let carriers = self.take_carriers(args, 2);
                ExtensionOperation::XLoopIn {
                    direction,
                    needle,
                    carriers,
                }
            }
            "x-yarn-in" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let needle = self.take_needle(args, 1, opcode_span);
                let carriers = self.take_carriers(args, 2);
                ExtensionOperation::XYarnIn {
                    direction,
                    needle,
                    carriers,
                }
            }
            "x-yarn-out" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let needle = self.take_needle(args, 1, opcode_span);
                let carriers = self.take_carriers(args, 2);
                ExtensionOperation::XYarnOut {
                    direction,
                    needle,
                    carriers,
                }
            }
            "x-end" => {
                self.warn_extra_args(args, 0);
                ExtensionOperation::XEnd
            }
            "x-roller-advance" => {
                let value = self.take_big_decimal(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                ExtensionOperation::XRollerAdvance { value }
            }
            "x-add-roller-advance" => {
                let value = self.take_big_decimal(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                ExtensionOperation::XAddRollerAdvance { value }
            }
            "x-carrier-spacing" => {
                let value = self.take_u32(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                ExtensionOperation::XCarrierSpacing { value }
            }
            "x-carrier-stopping-distance" => {
                let value = self.take_u32(args, 0, opcode_span);
                self.warn_extra_args(args, 1);
                ExtensionOperation::XCarrierStoppingDistance { value }
            }
            "x-xfer-style" => {
                let style = self.parse_xfer_style_arg(args, opcode_span);
                self.warn_extra_args(args, 1);
                ExtensionOperation::XXferStyle { style }
            }
            _ => {
                // Unknown x- extension — store name + all args as strings.
                let name = self.parse_my_string(opcode_str, opcode_span);
                let arguments = args
                    .iter()
                    .map(|&(span, text)| self.parse_my_string(text, span))
                    .collect();
                ExtensionOperation::UnknownExtension { name, arguments }
            }
        };
        Operation::ExtensionOperation(ext)
    }

    fn parse_presser_mode_arg(
        &mut self,
        args: &[(Span, &str)],
        opcode_span: Span,
    ) -> PresserMode<Parsed> {
        match args.first() {
            Some(&(span, text)) => match text {
                "off" => PresserMode::Off(span),
                "auto" => PresserMode::Auto(span),
                "on" => PresserMode::On(span),
                _ => {
                    self.errors.push(ParseError::BadPresserMode(span));
                    PresserMode::Other(span, span)
                }
            },
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                PresserMode::Other(opcode_span, opcode_span)
            }
        }
    }

    fn parse_xfer_style_arg(
        &mut self,
        args: &[(Span, &str)],
        opcode_span: Span,
    ) -> XferStyle<Parsed> {
        match args.first() {
            Some(&(span, text)) => match text {
                "four-pass" => XferStyle::FourPass(span),
                "two-pass" => XferStyle::TwoPass(span),
                _ => {
                    self.errors.push(ParseError::BadXferStyle(span));
                    XferStyle::Other(span, span)
                }
            },
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                XferStyle::Other(opcode_span, opcode_span)
            }
        }
    }
}

// ─── public entry point ───────────────────────────────────────────────────────

/// Lower a tree-sitter CST for a knitout source file into a `Program<Parsed>`.
///
/// Returns the program together with any parse warnings and errors encountered.  Errors are
/// recoverable: the program is always produced, but may contain `Other(Span)` nodes where
/// parsing failed.  Call `parsed_to_valid` to check whether the program is error-free and
/// produce a `Program<Validated>`.
pub fn cst_to_program(
    source: &[u8],
    tree: &Tree,
) -> (Program<Parsed>, Vec<ParseWarning>, Vec<ParseError>) {
    let mut ctx = LowerCtx::new(source);
    let root = tree.root_node();

    let mut line_index = 0usize;
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        ctx.lower_line(child, &mut line_index);
    }

    ctx.finish()
}
