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
    ExtensionOperation, Headers, IsSlider, Needle, Operation, PositionHeaderValue,
    PresserMode, Program, Rail, RailSide, Span, XferStyle,
};
use knitout_ast::{CarrierTable, MyBigDecimal, MyString, MyU32};
use tree_sitter::{Node, Tree};

use crate::{CommentInfo, Parsed};
use crate::diagnostics::{ParseError, ParseWarning};

// ─── span helpers ────────────────────────────────────────────────────────────

fn node_span(node: Node<'_>) -> Span {
    Span::new(node.start_byte() as u32, node.end_byte() as u32)
}

fn text_of<'s>(node: Node<'_>, source: &'s [u8]) -> &'s [u8] {
    &source[node.start_byte()..node.end_byte()]
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

    /// Tracked previously-seen header spans for duplicate detection.
    /// Key = canonical header name, value = span of name+colon.
    seen_headers: std::collections::HashMap<String, Span>,

    /// Accumulated operations.
    operations: Vec<Annotated<Operation<Parsed>, (Span, CommentInfo)>>,

    /// Partially-built headers struct (we can't finalise it until we know if `Carriers:` arrived).
    carriers: Option<(Annotated<CarriersHeaderValue<Parsed>, Span>, Span)>,
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
            // Placeholder headers — will be replaced or patched at the end.
            carriers: None,
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
        let carriers_either: Either<Annotated<CarriersHeaderValue<Parsed>, Span>, Span> =
            match self.carriers {
                Some((ann, _name_span)) => Either::Left(ann),
                None => {
                    self.errors.push(ParseError::MissingCarriersHeader);
                    Either::Right(Span::new(0, 0))
                }
            };

        let headers = self.headers_partial;
        // Patch the carriers field — we built headers_partial with a dummy Right, now replace.
        // We accomplish this by rebuilding: consume the partial headers, reconstruct with carriers.
        let (_, yarn, machine, gauge, width, position, unknown) = headers.into_components();
        let mut final_headers: Headers<Parsed> = Headers::new(carriers_either);
        for (carrier, color_ann) in yarn {
            final_headers.add_yarn(carrier, color_ann.value, color_ann.annotation);
        }
        if let Some(m) = machine {
            final_headers.set_machine(m);
        }
        if let Some(g) = gauge {
            final_headers.set_gauge(g);
        }
        if let Some(w) = width {
            final_headers.set_width(w);
        }
        if let Some(p) = position {
            final_headers.set_position(p);
        }
        for (key, val) in unknown {
            final_headers.add_unknown(key, val.value, val.annotation);
        }

        let mut prog = Program::new(version_either, final_headers, self.carrier_table);
        for op in self.operations {
            prog.push_operation(op);
        }

        (prog, self.warnings, self.errors)
    }

    // ── header helpers ────────────────────────────────────────────────────────

    /// Check for duplicate headers. Returns `true` if this is the first occurrence (proceed),
    /// `false` if it is a duplicate (do not overwrite).
    fn check_header_duplicate(&mut self, name: &str, name_span: Span, _value_repr: &str) -> bool {
        match self.seen_headers.get(name) {
            None => {
                self.seen_headers.insert(name.to_owned(), name_span);
                true
            }
            Some(&prev_span) => {
                // We treat any second definition as a conflict (different value → error,
                // same value → warning).  We record the error here; the caller may refine.
                self.errors.push(ParseError::ConflictingHeader {
                    span: name_span,
                    previous: prev_span,
                });
                false
            }
        }
    }

    fn warn_header_after_ops(&mut self, span: Span) {
        if self.operations_started {
            self.warnings.push(ParseWarning::HeaderAfterOperations(span));
        }
    }

    // ── line dispatchers ──────────────────────────────────────────────────────

    fn lower_line(&mut self, node: Node<'_>, line_index: &mut usize) {
        // The grammar's `line` rule is a transparent choice; the named child is the real node.
        let inner = if node.kind() == "line" {
            match node.named_child(0) {
                Some(n) => n,
                None => return,
            }
        } else {
            node
        };

        match inner.kind() {
            "tag_comment" => self.lower_top_tag_comment(inner, *line_index),
            "header" => self.lower_header(inner),
            "comment" => {} // plain comment, nothing to do
            "op_line" => {
                self.lower_op_line(inner);
            }
            _ => {}
        }
        *line_index += 1;
    }

    // ── magic / tag line ──────────────────────────────────────────────────────

    fn lower_top_tag_comment(&mut self, node: Node<'_>, line_index: usize) {
        let tag_span = node_span(node);
        let value_node = node.child_by_field_name("value");
        let value_text = value_node.map(|n| text_of(n, self.source));

        // Is this the magic line?
        let is_magic = value_text.map_or(false, |t| t.starts_with(b"knitout-"));
        if is_magic {
            let value_node = value_node.unwrap();
            let vstart = value_node.start_byte() as u32;
            let vend = value_node.end_byte() as u32;
            let value_text = value_text.unwrap();

            // Span for ";!knitout-" = from tag start to vstart + len("knitout-")
            let prefix_span = Span::new(tag_span.start(), vstart + 8);
            // Span for the version digits
            let ver_text = &value_text[8..]; // after "knitout-"
            let ver_start = vstart + 8;
            let ver_end = vend;
            let ver_span = Span::new(ver_start, ver_end);

            if self.magic.is_some() {
                // Second magic line → warning (treat as MagicNotFirst of this occurrence)
                self.warnings.push(ParseWarning::MagicNotFirst(tag_span));
                return;
            }

            if line_index > 0 {
                self.warnings.push(ParseWarning::MagicNotFirst(tag_span));
            }

            let ver_str = std::str::from_utf8(ver_text).unwrap_or("");
            match ver_str.parse::<u32>() {
                Ok(ver) => {
                    self.magic = Some((ver, (prefix_span, ver_span)));
                }
                Err(_) => {
                    self.errors.push(ParseError::BadMagicVersion(ver_span));
                    // Still record a "version = 0" placeholder so the tree is navigable.
                    self.magic = Some((0, (prefix_span, ver_span)));
                }
            }
            return;
        }

        // Source tag at the top level — unusual but we just ignore it (not an error).
        // Unknown tags are also ignored here since we can't attach them to an operation.
    }

    // ── header parsing ────────────────────────────────────────────────────────

    fn lower_header(&mut self, node: Node<'_>) {
        let _header_span = node_span(node);

        let content_node = match node.child_by_field_name("content") {
            Some(n) => n,
            None => return, // just ";;" with no content
        };

        let content_text = text_of(content_node, self.source);
        let content_start = content_node.start_byte() as u32;

        // Split at the first ':' to get name and value.
        let colon_pos = content_text.iter().position(|&b| b == b':');
        let (name_bytes, value_bytes, value_start) = match colon_pos {
            Some(p) => {
                let vstart = content_start + p as u32 + 1;
                (&content_text[..p], &content_text[p + 1..], vstart)
            }
            None => {
                // No colon — treat whole content as name with empty value.
                (content_text, &b""[..], content_start + content_text.len() as u32)
            }
        };

        let name_str = std::str::from_utf8(name_bytes).unwrap_or("").trim();
        // name+colon span (for P::Header annotation)
        let name_colon_end = content_start + colon_pos.unwrap_or(name_bytes.len()) as u32 + colon_pos.map_or(0, |_| 1);
        let name_colon_span = Span::new(content_start, name_colon_end);

        // Trim leading whitespace from value
        let value_trimmed_offset = value_bytes
            .iter()
            .position(|&b| b != b' ' && b != b'\t')
            .unwrap_or(value_bytes.len());
        let value_trim_start = value_start + value_trimmed_offset as u32;
        let value_end = content_start + content_text.len() as u32;
        let trimmed_value = &value_bytes[value_trimmed_offset..];
        let value_span = Span::new(value_trim_start, value_end);

        self.warn_header_after_ops(name_colon_span);

        match name_str {
            "Carriers" => self.lower_carriers_header(name_colon_span, trimmed_value, value_trim_start, value_end),
            "Machine" => self.lower_string_header("Machine", name_colon_span, trimmed_value, value_trim_start, value_end),
            "Gauge" => self.lower_u32_header("Gauge", name_colon_span, trimmed_value, value_span),
            "Width" => self.lower_u32_header("Width", name_colon_span, trimmed_value, value_span),
            "Position" => self.lower_position_header(name_colon_span, trimmed_value, value_span),
            s if s.starts_with("Yarn-") => {
                self.lower_yarn_header(name_colon_span, s, trimmed_value, value_trim_start, value_end)
            }
            s if s.len() >= 2 && s[..2].eq_ignore_ascii_case("X-") => {
                self.warnings.push(ParseWarning::UnrecognizedExtensionHeader(name_colon_span));
                // Store as an unknown header.
                let val_str = std::str::from_utf8(trimmed_value).unwrap_or("").trim().to_owned();
                self.headers_partial.add_unknown(s.to_owned(), val_str, name_colon_span);
            }
            _ => {
                // Completely unknown header — silently store.
                let val_str = std::str::from_utf8(trimmed_value).unwrap_or("").trim().to_owned();
                self.headers_partial.add_unknown(name_str.to_owned(), val_str, name_colon_span);
            }
        }
    }

    fn lower_carriers_header(
        &mut self,
        name_span: Span,
        value: &[u8],
        value_start: u32,
        value_end: u32,
    ) {
        if !self.check_header_duplicate("Carriers", name_span, "") {
            return;
        }
        let _carriers_span = Span::new(value_start, value_end);
        let chv = self.parse_carriers_header_value(value, value_start, value_end);
        let ann = Annotated::new(chv, name_span);
        self.carriers = Some((ann, name_span));
    }

    fn lower_string_header(
        &mut self,
        key: &str,
        name_span: Span,
        value: &[u8],
        value_start: u32,
        value_end: u32,
    ) {
        let val_str = std::str::from_utf8(value).unwrap_or("").trim().to_owned();
        if !self.check_header_duplicate(key, name_span, &val_str) {
            return;
        }
        let value_span = Span::new(value_start, value_end);
        let my_string = MyString::new(Either::Left(Annotated::new(val_str, value_span)));
        self.headers_partial.set_machine(Annotated::new(my_string, name_span));
    }

    fn lower_u32_header(&mut self, key: &str, name_span: Span, value: &[u8], value_span: Span) {
        let val_str = std::str::from_utf8(value).unwrap_or("").trim();
        if !self.check_header_duplicate(key, name_span, val_str) {
            return;
        }
        let my_u32 = match val_str.parse::<u32>() {
            Ok(n) => MyU32::new(Either::Left(Annotated::new(n, value_span))),
            Err(_) => {
                self.errors.push(ParseError::BadU32(value_span));
                MyU32::new(Either::Right(value_span))
            }
        };
        if key == "Gauge" {
            self.headers_partial.set_gauge(Annotated::new(my_u32, name_span));
        } else {
            self.headers_partial.set_width(Annotated::new(my_u32, name_span));
        }
    }

    fn lower_position_header(&mut self, name_span: Span, value: &[u8], value_span: Span) {
        let val_str = std::str::from_utf8(value).unwrap_or("").trim();
        if !self.check_header_duplicate("Position", name_span, val_str) {
            return;
        }
        let pos = match val_str {
            "Left" => PositionHeaderValue::Left(value_span),
            "Center" => PositionHeaderValue::Center(value_span),
            "Right" => PositionHeaderValue::Right(value_span),
            "Keep" => PositionHeaderValue::Keep(value_span),
            _ => {
                self.errors.push(ParseError::BadPositionValue(value_span));
                PositionHeaderValue::Other(value_span)
            }
        };
        self.headers_partial.set_position(Annotated::new(pos, name_span));
    }

    fn lower_yarn_header(
        &mut self,
        name_span: Span,
        name_str: &str,
        value: &[u8],
        value_start: u32,
        value_end: u32,
    ) {
        // "Yarn-A" → carrier name is "A"
        let carrier_name = &name_str["Yarn-".len()..];
        let carrier = self.carrier_table.make_carrier(carrier_name, name_span);
        let val_str = std::str::from_utf8(value).unwrap_or("").trim().to_owned();
        let value_span = Span::new(value_start, value_end);
        let my_string = MyString::new(Either::Left(Annotated::new(val_str, value_span)));
        self.headers_partial.add_yarn(carrier, my_string, name_span);
    }

    // ── carriers header value parsing ─────────────────────────────────────────

    fn parse_carriers_header_value(
        &mut self,
        text: &[u8],
        text_start: u32,
        text_end: u32,
    ) -> CarriersHeaderValue<Parsed> {
        let annotation_span = Span::new(text_start, text_end);
        let mut items: Vec<CarrierOrRail<Parsed>> = Vec::new();

        let mut i = 0usize;
        while i < text.len() {
            // Skip spaces
            while i < text.len() && (text[i] == b' ' || text[i] == b'\t') {
                i += 1;
            }
            if i >= text.len() {
                break;
            }
            // Collect token (no whitespace)
            let token_start = i;
            while i < text.len() && text[i] != b' ' && text[i] != b'\t' {
                i += 1;
            }
            let token = &text[token_start..i];
            let abs_start = text_start + token_start as u32;
            let abs_end = text_start + i as u32;
            let token_span = Span::new(abs_start, abs_end);

            if let Some(semi_pos) = token.iter().position(|&b| b == b';') {
                // Rail: "left;right"
                let left_bytes = &token[..semi_pos];
                let right_bytes = &token[semi_pos + 1..];
                let left_start = abs_start;
                let left_end = abs_start + semi_pos as u32;
                let right_start = abs_start + semi_pos as u32 + 1;
                let right_end = abs_end;

                let left_side = self.parse_rail_side(left_bytes, left_start, left_end);
                let right_side = self.parse_rail_side(right_bytes, right_start, right_end);

                items.push(CarrierOrRail::Rail(Rail::new(
                    Either::Left(left_side),
                    Either::Left(right_side),
                    token_span,
                )));
            } else {
                // Single carrier
                let name = std::str::from_utf8(token).unwrap_or("").trim();
                if !name.is_empty() {
                    let carrier = self.carrier_table.make_carrier(name, token_span);
                    items.push(CarrierOrRail::Carrier(carrier));
                }
            }
        }

        CarriersHeaderValue::new(Either::Left(items), annotation_span)
    }

    fn parse_rail_side(
        &mut self,
        text: &[u8],
        text_start: u32,
        text_end: u32,
    ) -> RailSide<Parsed> {
        let side_span = Span::new(text_start, text_end);
        let mut carriers = Vec::new();
        let mut pos = 0u32;

        for part in text.split(|&b| b == b',') {
            let part_start = text_start + pos;
            let part_len = part.len() as u32;
            let part_span = Span::new(part_start, part_start + part_len);
            pos += part_len + 1; // +1 for ','

            let name = std::str::from_utf8(part).unwrap_or("").trim();
            if !name.is_empty() {
                let carrier = self.carrier_table.make_carrier(name, part_span);
                carriers.push(carrier);
            }
        }

        RailSide::new(carriers, side_span)
    }

    // ── operation line ────────────────────────────────────────────────────────

    fn lower_op_line(&mut self, node: Node<'_>) {
        let op_node = match node.child_by_field_name("operation") {
            Some(n) => n,
            None => return,
        };

        // Trailing comment
        let comment_info = if let Some(tag) = node.child_by_field_name("tag_comment") {
            let tag_span = node_span(tag);
            let value_text = tag
                .child_by_field_name("value")
                .map(|v| text_of(v, self.source));
            if value_text.map_or(false, |t| t.starts_with(b"source: ")) {
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

        let opcode_node = match op_node.child_by_field_name("opcode") {
            Some(n) => n,
            None => return,
        };
        let opcode_text = text_of(opcode_node, self.source);
        let opcode_span = node_span(opcode_node);

        // Collect argument nodes
        let mut args: Vec<(Span, &[u8])> = Vec::new();
        let mut cursor = op_node.walk();
        for arg in op_node.children_by_field_name("argument", &mut cursor) {
            args.push((node_span(arg), text_of(arg, self.source)));
        }

        let op = self.parse_operation(opcode_text, opcode_span, &args);
        self.operations.push(Annotated::new(op, (op_span, comment_info)));

        if !self.operations_started {
            self.operations_started = true;
            self.first_op_span = Some(op_span);
        }
    }

    // ── argument helpers ──────────────────────────────────────────────────────

    fn parse_direction(&mut self, text: &[u8], span: Span) -> Direction<Parsed> {
        match text {
            b"+" => Direction::Right(span),
            b"-" => Direction::Left(span),
            _ => {
                self.errors.push(ParseError::BadDirection(span));
                Direction::Other(span)
            }
        }
    }

    fn parse_needle(&mut self, text: &[u8], span: Span) -> Either<Needle<Parsed>, Span> {
        if text.is_empty() {
            self.errors.push(ParseError::BadNeedle(span));
            return Either::Right(span);
        }

        let bed_byte = text[0];
        let bed_span = Span::new(span.start(), span.start() + 1);
        let bed = match bed_byte {
            b'f' => Bed::Front(bed_span),
            b'b' => Bed::Back(bed_span),
            _ => {
                self.errors.push(ParseError::BadBed(bed_span));
                Bed::Other(bed_span)
            }
        };

        let mut pos = 1usize;
        let is_slider = if pos < text.len() && text[pos] == b's' {
            let s_span = Span::new(span.start() + 1, span.start() + 2);
            pos += 1;
            IsSlider::Slider(s_span)
        } else {
            let empty = Span::new(span.start() + 1, span.start() + 1);
            IsSlider::NotSlider(empty)
        };

        let index_bytes = &text[pos..];
        let index_start = span.start() + pos as u32;
        let index_end = span.end();
        let index_span = Span::new(index_start, index_end);

        let index_str = std::str::from_utf8(index_bytes).unwrap_or("");
        match index_str.parse::<i32>() {
            Ok(idx) => {
                let needle = Needle::new(
                    bed,
                    is_slider,
                    Either::Left(Annotated::new(idx, index_span)),
                    span,
                );
                Either::Left(needle)
            }
            Err(_) => {
                self.errors.push(ParseError::BadNeedleIndex(index_span));
                // Return a needle with Other index, keeping bed/slider for partial info.
                let needle = Needle::new(bed, is_slider, Either::Right(index_span), span);
                Either::Left(needle)
            }
        }
    }

    fn parse_big_decimal(&mut self, text: &[u8], span: Span) -> MyBigDecimal<Parsed> {
        let s = std::str::from_utf8(text).unwrap_or("");
        match BigDecimal::from_str(s) {
            Ok(bd) => MyBigDecimal::new(Either::Left(Annotated::new(bd, span))),
            Err(_) => {
                self.errors.push(ParseError::BadBigDecimal(span));
                MyBigDecimal::new(Either::Right(span))
            }
        }
    }

    fn parse_u32(&mut self, text: &[u8], span: Span) -> MyU32<Parsed> {
        let s = std::str::from_utf8(text).unwrap_or("");
        match s.parse::<u32>() {
            Ok(n) => MyU32::new(Either::Left(Annotated::new(n, span))),
            Err(_) => {
                self.errors.push(ParseError::BadU32(span));
                MyU32::new(Either::Right(span))
            }
        }
    }

    fn parse_my_string(&self, text: &[u8], span: Span) -> MyString<Parsed> {
        let s = std::str::from_utf8(text).unwrap_or("").to_owned();
        MyString::new(Either::Left(Annotated::new(s, span)))
    }

    fn parse_carrier(&mut self, text: &[u8], span: Span) -> knitout_ast::Carrier<Parsed> {
        let name = std::str::from_utf8(text).unwrap_or("").trim();
        self.carrier_table.make_carrier(name, span)
    }

    /// Consume `args[at]` as a direction.  If absent, emits `MissingArgument` and returns Other.
    fn take_direction(
        &mut self,
        args: &[(Span, &[u8])],
        at: usize,
        opcode_span: Span,
    ) -> Direction<Parsed> {
        match args.get(at) {
            Some(&(span, text)) => self.parse_direction(text, span),
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                Direction::Other(opcode_span)
            }
        }
    }

    /// Consume `args[at]` as a needle.  If absent, emits `MissingArgument` and returns Right.
    fn take_needle(
        &mut self,
        args: &[(Span, &[u8])],
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
        args: &[(Span, &[u8])],
        at: usize,
        opcode_span: Span,
    ) -> MyBigDecimal<Parsed> {
        match args.get(at) {
            Some(&(span, text)) => self.parse_big_decimal(text, span),
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                MyBigDecimal::new(Either::Right(opcode_span))
            }
        }
    }

    /// Consume `args[at]` as a u32.  If absent, emits `MissingArgument`.
    fn take_u32(
        &mut self,
        args: &[(Span, &[u8])],
        at: usize,
        opcode_span: Span,
    ) -> MyU32<Parsed> {
        match args.get(at) {
            Some(&(span, text)) => self.parse_u32(text, span),
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                MyU32::new(Either::Right(opcode_span))
            }
        }
    }

    /// Consume `args[at]` as a string.  If absent, emits `MissingArgument`.
    fn take_string(
        &mut self,
        args: &[(Span, &[u8])],
        at: usize,
        opcode_span: Span,
    ) -> MyString<Parsed> {
        match args.get(at) {
            Some(&(span, text)) => self.parse_my_string(text, span),
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                MyString::new(Either::Right(opcode_span))
            }
        }
    }

    /// All args from `from` onwards as carriers.
    fn take_carriers(&mut self, args: &[(Span, &[u8])], from: usize) -> Vec<knitout_ast::Carrier<Parsed>> {
        args[from.min(args.len())..]
            .iter()
            .map(|&(span, text)| self.parse_carrier(text, span))
            .collect()
    }

    fn warn_extra_args(&mut self, args: &[(Span, &[u8])], from: usize) {
        for &(span, _) in &args[from.min(args.len())..] {
            self.errors.push(ParseError::TooManyArguments(span));
        }
    }

    // ── operation parsers ─────────────────────────────────────────────────────

    fn parse_operation(
        &mut self,
        opcode: &[u8],
        opcode_span: Span,
        args: &[(Span, &[u8])],
    ) -> Operation<Parsed> {
        let opcode_str = std::str::from_utf8(opcode).unwrap_or("");
        match opcode_str {
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
                Operation::BaseOperation(BaseOperation::Stitch { before_pull, after_pull })
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
                Operation::BaseOperation(BaseOperation::Knit { direction, needle, carriers })
            }
            "tuck" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let needle = self.take_needle(args, 1, opcode_span);
                let carriers = self.take_carriers(args, 2);
                Operation::BaseOperation(BaseOperation::Tuck { direction, needle, carriers })
            }
            "miss" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let needle = self.take_needle(args, 1, opcode_span);
                let carriers = self.take_carriers(args, 2);
                Operation::BaseOperation(BaseOperation::Miss { direction, needle, carriers })
            }
            "split" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let source = self.take_needle(args, 1, opcode_span);
                let destination = self.take_needle(args, 2, opcode_span);
                let carriers = self.take_carriers(args, 3);
                Operation::BaseOperation(BaseOperation::Split { direction, source, destination, carriers })
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
                Operation::BaseOperation(BaseOperation::Xfer { source, destination })
            }

            // ── extension operations ──────────────────────────────────────────
            s if s.starts_with("x-") => self.parse_extension_op(s, opcode_span, args),

            // ── unknown ───────────────────────────────────────────────────────
            _ => {
                self.errors.push(ParseError::UnrecognizedOpcode(opcode_span));
                Operation::Other(opcode_span)
            }
        }
    }

    fn parse_extension_op(
        &mut self,
        opcode_str: &str,
        opcode_span: Span,
        args: &[(Span, &[u8])],
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
                ExtensionOperation::XLoopIn { direction, needle, carriers }
            }
            "x-yarn-in" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let needle = self.take_needle(args, 1, opcode_span);
                let carriers = self.take_carriers(args, 2);
                ExtensionOperation::XYarnIn { direction, needle, carriers }
            }
            "x-yarn-out" => {
                let direction = self.take_direction(args, 0, opcode_span);
                let needle = self.take_needle(args, 1, opcode_span);
                let carriers = self.take_carriers(args, 2);
                ExtensionOperation::XYarnOut { direction, needle, carriers }
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
                let name = self.parse_my_string(opcode_str.as_bytes(), opcode_span);
                let arguments = args
                    .iter()
                    .map(|&(span, text)| self.parse_my_string(text, span))
                    .collect();
                ExtensionOperation::UnknownExtension { name, arguments }
            }
        };
        Operation::ExtensionOperation(ext)
    }

    fn parse_presser_mode_arg(&mut self, args: &[(Span, &[u8])], opcode_span: Span) -> PresserMode<Parsed> {
        match args.get(0) {
            Some(&(span, text)) => match text {
                b"off" => PresserMode::Off(span),
                b"auto" => PresserMode::Auto(span),
                b"on" => PresserMode::On(span),
                _ => {
                    self.errors.push(ParseError::BadPresserMode(span));
                    PresserMode::Other(span)
                }
            },
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                PresserMode::Other(opcode_span)
            }
        }
    }

    fn parse_xfer_style_arg(&mut self, args: &[(Span, &[u8])], opcode_span: Span) -> XferStyle<Parsed> {
        match args.get(0) {
            Some(&(span, text)) => match text {
                b"four-pass" => XferStyle::FourPass(span),
                b"two-pass" => XferStyle::TwoPass(span),
                _ => {
                    self.errors.push(ParseError::BadXferStyle(span));
                    XferStyle::Other(span)
                }
            },
            None => {
                self.errors.push(ParseError::MissingArgument(opcode_span));
                XferStyle::Other(opcode_span)
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
