use bigdecimal::BigDecimal;
use knitout_ast::{Carrier, CarrierTable, IsSlider, Span};
use std::str::FromStr;
use tree_sitter::Node;

use crate::diagnostics::ParseError;
use crate::types::*;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Walk a tree-sitter CST for a knitout source file and produce a weak AST
/// plus errors. Never panics; always returns a (possibly incomplete) program.
pub fn lower_cst(source: &[u8], tree: &tree_sitter::Tree) -> (WeakProgram, Vec<ParseError>) {
    let mut ctx = LowerCtx {
        source,
        errors: Vec::new(),
        carrier_table: CarrierTable::new(),
    };
    let root = tree.root_node();
    ctx.lower_source_file(root)
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

struct LowerCtx<'s> {
    source: &'s [u8],
    errors: Vec<ParseError>,
    carrier_table: CarrierTable,
}

impl<'s> LowerCtx<'s> {
    fn span_of(&self, node: Node) -> Span {
        Span::new(node.start_byte() as u32, node.end_byte() as u32)
    }

    fn text_of(&self, node: Node) -> &str {
        node.utf8_text(self.source).unwrap_or("")
    }

    fn push_error(&mut self, e: ParseError) {
        self.errors.push(e);
    }

    /// Get a required named field. Emits an error and returns None when the
    /// field is absent, MISSING, or an ERROR node.
    fn required_field<'t>(&mut self, parent: Node<'t>, field: &'static str) -> Option<Node<'t>> {
        match parent.child_by_field_name(field) {
            None => {
                self.push_error(ParseError::MissingField {
                    span: self.span_of(parent),
                    field,
                });
                None
            }
            Some(n) if n.is_missing() => {
                self.push_error(ParseError::MissingField {
                    span: self.span_of(n),
                    field,
                });
                None
            }
            Some(n) if n.is_error() => {
                self.push_error(ParseError::InvalidField {
                    span: self.span_of(n),
                    field,
                });
                None
            }
            Some(n) => Some(n),
        }
    }

    /// Get an optional named field. Returns None silently when absent.
    /// Emits an error for MISSING or ERROR nodes.
    fn optional_field<'t>(&mut self, parent: Node<'t>, field: &'static str) -> Option<Node<'t>> {
        match parent.child_by_field_name(field) {
            None => None,
            Some(n) if n.is_missing() => {
                self.push_error(ParseError::MissingField {
                    span: self.span_of(n),
                    field,
                });
                None
            }
            Some(n) if n.is_error() => {
                self.push_error(ParseError::InvalidField {
                    span: self.span_of(n),
                    field,
                });
                None
            }
            Some(n) => Some(n),
        }
    }

    // -----------------------------------------------------------------------
    // Leaf parsers
    // -----------------------------------------------------------------------

    fn lower_bed(&mut self, node: Node) -> WeakBed {
        match self.text_of(node) {
            "f" => WeakBed::Front,
            "b" => WeakBed::Back,
            _ => {
                let span = self.span_of(node);
                self.push_error(ParseError::UnrecognizedBed(span));
                WeakBed::Unknown(span)
            }
        }
    }

    fn lower_direction(&mut self, node: Node) -> WeakDirection {
        match self.text_of(node) {
            "+" => WeakDirection::Right,
            "-" => WeakDirection::Left,
            _ => {
                let span = self.span_of(node);
                self.push_error(ParseError::UnrecognizedDirection(span));
                WeakDirection::Unknown(span)
            }
        }
    }

    fn lower_presser_mode(&mut self, node: Node) -> WeakPresserMode {
        match self.text_of(node) {
            "off" => WeakPresserMode::Off,
            "auto" => WeakPresserMode::Auto,
            "on" => WeakPresserMode::On,
            _ => {
                let span = self.span_of(node);
                self.push_error(ParseError::UnrecognizedPresserMode(span));
                WeakPresserMode::Unknown(span)
            }
        }
    }

    fn lower_xfer_style(&mut self, node: Node) -> WeakXferStyle {
        match self.text_of(node) {
            "four-pass" => WeakXferStyle::FourPass,
            "two-pass" => WeakXferStyle::TwoPass,
            _ => {
                let span = self.span_of(node);
                self.push_error(ParseError::UnrecognizedXferStyle(span));
                WeakXferStyle::Unknown(span)
            }
        }
    }

    fn lower_position_value(&mut self, node: Node) -> WeakPositionValue {
        match self.text_of(node) {
            "Left" => WeakPositionValue::Left,
            "Center" => WeakPositionValue::Center,
            "Right" => WeakPositionValue::Right,
            "Keep" => WeakPositionValue::Keep,
            _ => {
                let span = self.span_of(node);
                self.push_error(ParseError::UnrecognizedPositionValue(span));
                WeakPositionValue::Unknown(span)
            }
        }
    }

    fn lower_rational(&mut self, node: Node) -> Option<BigDecimal> {
        let text = self.text_of(node);
        match BigDecimal::from_str(text) {
            Ok(v) => Some(v),
            Err(_) => {
                self.push_error(ParseError::InvalidRational(self.span_of(node)));
                None
            }
        }
    }

    fn lower_nonneg_integer(&mut self, node: Node) -> Option<u32> {
        let text = self.text_of(node).trim_start_matches('+');
        match text.parse::<u32>() {
            Ok(v) => Some(v),
            Err(_) => {
                self.push_error(ParseError::InvalidInteger(self.span_of(node)));
                None
            }
        }
    }

    fn lower_nonneg_half_integer(&mut self, node: Node) -> Option<u32> {
        // Grammar: /[+]?\d*(\.5)/  — values like "0.5", "1.5", "10.5"
        // Encoded as the integer part only (0.5 → 0)
        let text = self.text_of(node).trim_start_matches('+');
        let integer_part = if let Some(dot) = text.find('.') {
            &text[..dot]
        } else {
            text
        };
        let as_u32: u32 = if integer_part.is_empty() {
            0
        } else {
            match integer_part.parse() {
                Ok(v) => v,
                Err(_) => {
                    self.push_error(ParseError::InvalidInteger(self.span_of(node)));
                    return None;
                }
            }
        };
        Some(as_u32)
    }

    fn lower_needle_index(&mut self, node: Node) -> Option<i32> {
        match self.text_of(node).parse::<i32>() {
            Ok(v) => Some(v),
            Err(_) => {
                self.push_error(ParseError::InvalidInteger(self.span_of(node)));
                None
            }
        }
    }

    fn lower_carrier_name(&mut self, node: Node) -> Carrier {
        let name = self.text_of(node).to_owned();
        self.carrier_table.make_carrier(&name)
    }

    // -----------------------------------------------------------------------
    // Needle
    // -----------------------------------------------------------------------

    fn lower_needle(&mut self, node: Node) -> WeakNeedle {
        let span = self.span_of(node);

        let bed = match self.required_field(node, "bed") {
            Some(n) => self.lower_bed(n),
            None => WeakBed::Unknown(span),
        };

        let is_slider = match node.child_by_field_name("slider") {
            Some(n) if !n.is_missing() && !n.is_error() => IsSlider::Slider,
            _ => IsSlider::NotSlider,
        };

        let index = match self.required_field(node, "index") {
            Some(n) => self.lower_needle_index(n),
            None => None,
        };

        WeakNeedle {
            span,
            bed,
            is_slider,
            index,
        }
    }

    fn lower_required_needle(&mut self, parent: Node, field: &'static str) -> Option<WeakNeedle> {
        self.required_field(parent, field)
            .map(|n| self.lower_needle(n))
    }

    // -----------------------------------------------------------------------
    // Carrier sets
    // -----------------------------------------------------------------------

    /// Parse a required carrier_set field. Returns None when absent/errored.
    fn lower_required_carrier_set(
        &mut self,
        parent: Node,
        field: &'static str,
    ) -> Option<Vec<Carrier>> {
        let node = self.required_field(parent, field)?;
        Some(self.lower_carrier_set_node(node))
    }

    /// Parse an optional carrier_set field. Returns empty vec when absent.
    fn lower_optional_carrier_set(&mut self, parent: Node, field: &'static str) -> Vec<Carrier> {
        match self.optional_field(parent, field) {
            Some(n) => self.lower_carrier_set_node(n),
            None => Vec::new(),
        }
    }

    fn lower_carrier_set_node(&mut self, node: Node) -> Vec<Carrier> {
        let mut carriers = Vec::new();
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() == "carrier" && !child.is_missing() && !child.is_error() {
                carriers.push(self.lower_carrier_name(child));
            } else if child.is_error() || child.is_missing() {
                self.push_error(ParseError::InvalidCarrier(self.span_of(child)));
            }
        }
        carriers
    }

    // -----------------------------------------------------------------------
    // Source file
    // -----------------------------------------------------------------------

    fn lower_source_file(mut self, root: Node) -> (WeakProgram, Vec<ParseError>) {
        let span = self.span_of(root);

        if root.is_error() {
            self.push_error(ParseError::UnparsedFile(span));
            return (
                WeakProgram {
                    span,
                    magic: None,
                    headers: Vec::new(),
                    lines: Vec::new(),
                    carrier_table: self.carrier_table,
                },
                self.errors,
            );
        }

        let magic = match root.child_by_field_name("magic") {
            Some(n) if n.is_missing() => {
                self.push_error(ParseError::MissingMagic(span));
                WeakMagic {
                    span,
                    version: None,
                }
            }
            Some(n) if n.is_error() => {
                self.push_error(ParseError::InvalidMagic(self.span_of(n)));
                WeakMagic {
                    span: self.span_of(n),
                    version: None,
                }
            }
            Some(n) => self.lower_magic(n),
            None => {
                self.push_error(ParseError::MissingMagic(span));
                WeakMagic {
                    span,
                    version: None,
                }
            }
        };

        let mut headers = Vec::new();
        let mut lines = Vec::new();

        let mut cursor = root.walk();
        for child in root.named_children(&mut cursor) {
            match child.kind() {
                "magic" => {}
                "header" => headers.push(self.lower_header(child)),
                "line" => lines.push(self.lower_line(child)),
                _ if child.is_error() => {
                    self.push_error(ParseError::InvalidOperation(self.span_of(child)));
                }
                _ => {}
            }
        }

        let carrier_table = std::mem::replace(&mut self.carrier_table, CarrierTable::new());
        WeakProgram {
            span,
            magic: Some(magic),
            headers,
            lines,
            carrier_table,
        }
    }

    // -----------------------------------------------------------------------
    // Magic string
    // -----------------------------------------------------------------------

    fn lower_magic(&mut self, node: Node) -> WeakMagic {
        let span = self.span_of(node);
        let version = match node.child_by_field_name("version") {
            None => {
                self.push_error(ParseError::MissingVersion(span));
                None
            }
            Some(n) if n.is_missing() => {
                self.push_error(ParseError::MissingVersion(self.span_of(n)));
                None
            }
            Some(n) if n.is_error() => {
                self.push_error(ParseError::InvalidVersion(self.span_of(n)));
                None
            }
            Some(n) => match self.text_of(n).parse::<u32>() {
                Ok(v) => Some(v),
                Err(_) => {
                    self.push_error(ParseError::InvalidVersion(self.span_of(n)));
                    None
                }
            },
        };
        WeakMagic { span, version }
    }

    // -----------------------------------------------------------------------
    // Headers
    // -----------------------------------------------------------------------

    fn lower_header(&mut self, node: Node) -> WeakHeader {
        let span = self.span_of(node);

        if node.is_error() {
            self.push_error(ParseError::InvalidHeader(span));
            return WeakHeader {
                span,
                kind: WeakHeaderKind::Unknown,
            };
        }

        let mut cursor = node.walk();
        let child = node.named_children(&mut cursor).next();

        match child {
            None => {
                self.push_error(ParseError::InvalidHeader(span));
                WeakHeader {
                    span,
                    kind: WeakHeaderKind::Unknown,
                }
            }
            Some(c) if c.is_error() => {
                self.push_error(ParseError::UnrecognizedHeader(self.span_of(c)));
                WeakHeader {
                    span,
                    kind: WeakHeaderKind::Unknown,
                }
            }
            Some(c) => {
                let kind = match c.kind() {
                    "carriers_header" => WeakHeaderKind::Carriers(self.lower_carriers_header(c)),
                    "gauge_header" => WeakHeaderKind::Gauge(self.lower_gauge_header(c)),
                    "width_header" => WeakHeaderKind::Width(self.lower_width_header(c)),
                    "position_header" => WeakHeaderKind::Position(self.lower_position_header(c)),
                    "machine_header" => WeakHeaderKind::Machine(self.lower_machine_header(c)),
                    "yarn_header" => WeakHeaderKind::Yarn(self.lower_yarn_header(c)),
                    "extension_header" => WeakHeaderKind::Extension(self.lower_extension_header(c)),
                    _ => {
                        self.push_error(ParseError::UnrecognizedHeader(self.span_of(c)));
                        WeakHeaderKind::Unknown
                    }
                };
                WeakHeader { span, kind }
            }
        }
    }

    fn lower_carriers_header(&mut self, node: Node) -> WeakCarriersHeader {
        let value = match node.child_by_field_name("value") {
            None => None,
            Some(n) if n.is_missing() => {
                self.push_error(ParseError::MissingField {
                    span: self.span_of(n),
                    field: "value",
                });
                None
            }
            Some(n) if n.is_error() => {
                self.push_error(ParseError::InvalidField {
                    span: self.span_of(n),
                    field: "value",
                });
                None
            }
            Some(n) => Some(self.lower_carriers_value(n)),
        };
        WeakCarriersHeader { value }
    }

    fn lower_carriers_value(&mut self, node: Node) -> WeakCarriersValue {
        let span = self.span_of(node);
        let mut items = Vec::new();
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.is_error() {
                self.push_error(ParseError::InvalidCarrierOrRail(self.span_of(child)));
                items.push(WeakCarrierOrRail::Unknown(self.span_of(child)));
            } else {
                match child.kind() {
                    "carrier" => {
                        items.push(WeakCarrierOrRail::Carrier(self.lower_carrier_name(child)));
                    }
                    "rail_definition" => {
                        items.push(WeakCarrierOrRail::Rail(self.lower_rail_definition(child)));
                    }
                    _ => {
                        self.push_error(ParseError::InvalidCarrierOrRail(self.span_of(child)));
                        items.push(WeakCarrierOrRail::Unknown(self.span_of(child)));
                    }
                }
            }
        }
        WeakCarriersValue { span, items }
    }

    fn lower_rail_definition(&mut self, node: Node) -> WeakRail {
        let span = self.span_of(node);
        let left = match self.required_field(node, "left") {
            Some(n) => self.lower_rail_side(n),
            None => WeakRailSide {
                span,
                carriers: Vec::new(),
            },
        };
        let right = match self.required_field(node, "right") {
            Some(n) => self.lower_rail_side(n),
            None => WeakRailSide {
                span,
                carriers: Vec::new(),
            },
        };
        WeakRail { span, left, right }
    }

    fn lower_rail_side(&mut self, node: Node) -> WeakRailSide {
        let span = self.span_of(node);
        let mut carriers = Vec::new();
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() == "carrier" && !child.is_missing() && !child.is_error() {
                carriers.push(self.lower_carrier_name(child));
            } else if child.is_error() || child.is_missing() {
                self.push_error(ParseError::InvalidCarrier(self.span_of(child)));
            }
        }
        WeakRailSide { span, carriers }
    }

    fn lower_gauge_header(&mut self, node: Node) -> WeakGaugeHeader {
        let value = self
            .required_field(node, "value")
            .and_then(|n| self.lower_nonneg_integer(n));
        WeakGaugeHeader { value }
    }

    fn lower_width_header(&mut self, node: Node) -> WeakWidthHeader {
        let value = self
            .required_field(node, "value")
            .and_then(|n| self.lower_nonneg_integer(n));
        WeakWidthHeader { value }
    }

    fn lower_position_header(&mut self, node: Node) -> WeakPositionHeader {
        let value = match self.required_field(node, "value") {
            Some(n) => self.lower_position_value(n),
            None => WeakPositionValue::Unknown(self.span_of(node)),
        };
        WeakPositionHeader { value }
    }

    fn lower_machine_header(&mut self, node: Node) -> WeakMachineHeader {
        let value = self
            .required_field(node, "value")
            .map(|n| self.text_of(n).to_owned());
        WeakMachineHeader { value }
    }

    fn lower_yarn_header(&mut self, node: Node) -> WeakYarnHeader {
        let carrier = self
            .required_field(node, "carrier")
            .map(|n| self.lower_carrier_name(n));
        let value = self
            .required_field(node, "value")
            .map(|n| self.text_of(n).trim().to_owned());
        WeakYarnHeader { carrier, value }
    }

    fn lower_extension_header(&mut self, node: Node) -> WeakExtensionHeader {
        let name = self
            .required_field(node, "name")
            .map(|n| self.text_of(n).to_owned());
        let value = self
            .required_field(node, "value")
            .map(|n| self.text_of(n).trim().to_owned());
        WeakExtensionHeader { name, value }
    }

    // -----------------------------------------------------------------------
    // Lines
    // -----------------------------------------------------------------------

    fn lower_line(&mut self, node: Node) -> WeakLine {
        let span = self.span_of(node);

        let operation = match node.child_by_field_name("operation") {
            None => None,
            Some(n) if n.is_missing() => {
                self.push_error(ParseError::MissingField {
                    span: self.span_of(n),
                    field: "operation",
                });
                None
            }
            Some(n) if n.is_error() => {
                let op_span = self.span_of(n);
                self.push_error(ParseError::InvalidOperation(op_span));
                Some(WeakOperation::Unknown(op_span))
            }
            Some(n) => Some(self.lower_operation(n)),
        };

        let comment = node
            .child_by_field_name("comment")
            .map(|n| self.lower_comment(n));

        let source = node
            .child_by_field_name("source_comment")
            .and_then(|n| self.lower_source_comment(n));

        WeakLine {
            span,
            operation,
            comment,
            source,
        }
    }

    fn lower_comment(&mut self, node: Node) -> WeakComment {
        let span = self.span_of(node);
        let text = node
            .child_by_field_name("text")
            .filter(|n| !n.is_missing() && !n.is_error())
            .map(|n| self.text_of(n).to_owned());
        WeakComment { span, text }
    }

    fn lower_source_comment(&mut self, node: Node) -> Option<String> {
        node.child_by_field_name("source")
            .filter(|n| !n.is_missing() && !n.is_error())
            .map(|n| self.text_of(n).to_owned())
    }

    // -----------------------------------------------------------------------
    // Operations (dispatch)
    // -----------------------------------------------------------------------

    fn lower_operation(&mut self, node: Node) -> WeakOperation {
        let span = self.span_of(node);

        if node.is_error() {
            self.push_error(ParseError::InvalidOperation(span));
            return WeakOperation::Unknown(span);
        }

        match node.kind() {
            "in" | "inhook" | "releasehook" | "out" | "outhook" | "stitch" | "rack" | "knit"
            | "tuck" | "miss" | "split" | "pause" | "drop" | "amiss" | "xfer" => {
                WeakOperation::Base(self.lower_base_operation(node))
            }
            "extension" => {
                let mut cursor = node.walk();
                match node.named_children(&mut cursor).next() {
                    Some(inner) if !inner.is_error() => {
                        WeakOperation::Extension(self.lower_extension_operation(inner))
                    }
                    Some(inner) => {
                        let s = self.span_of(inner);
                        self.push_error(ParseError::InvalidOperation(s));
                        WeakOperation::Unknown(s)
                    }
                    None => {
                        self.push_error(ParseError::InvalidOperation(span));
                        WeakOperation::Unknown(span)
                    }
                }
            }
            _ => {
                self.push_error(ParseError::InvalidOperation(span));
                WeakOperation::Unknown(span)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Base operations
    // -----------------------------------------------------------------------

    fn lower_base_operation(&mut self, node: Node) -> WeakBaseOperation {
        let span = self.span_of(node);
        let kind = match node.kind() {
            "in" => WeakBaseOperationKind::In {
                carriers: self.lower_required_carrier_set(node, "carriers"),
            },
            "inhook" => WeakBaseOperationKind::Inhook {
                carriers: self.lower_required_carrier_set(node, "carriers"),
            },
            "releasehook" => WeakBaseOperationKind::Releasehook {
                carriers: self.lower_required_carrier_set(node, "carriers"),
            },
            "out" => WeakBaseOperationKind::Out {
                carriers: self.lower_required_carrier_set(node, "carriers"),
            },
            "outhook" => WeakBaseOperationKind::Outhook {
                carriers: self.lower_required_carrier_set(node, "carriers"),
            },
            "stitch" => {
                let before_pull = self
                    .required_field(node, "before_pull")
                    .and_then(|n| self.lower_rational(n));
                let after_pull = self
                    .required_field(node, "after_pull")
                    .and_then(|n| self.lower_rational(n));
                WeakBaseOperationKind::Stitch {
                    before_pull,
                    after_pull,
                }
            }
            "rack" => {
                let offset = self
                    .required_field(node, "offset")
                    .and_then(|n| self.lower_rational(n));
                WeakBaseOperationKind::Rack { offset }
            }
            "knit" => {
                let direction = self.lower_direction_field(node);
                let needle = self.lower_required_needle(node, "needle");
                let carriers = self.lower_optional_carrier_set(node, "carriers");
                WeakBaseOperationKind::Knit {
                    direction,
                    needle,
                    carriers,
                }
            }
            "tuck" => {
                let direction = self.lower_direction_field(node);
                let needle = self.lower_required_needle(node, "needle");
                let carriers = self.lower_optional_carrier_set(node, "carriers");
                WeakBaseOperationKind::Tuck {
                    direction,
                    needle,
                    carriers,
                }
            }
            "miss" => {
                let direction = self.lower_direction_field(node);
                let needle = self.lower_required_needle(node, "needle");
                let carriers = self.lower_optional_carrier_set(node, "carriers");
                WeakBaseOperationKind::Miss {
                    direction,
                    needle,
                    carriers,
                }
            }
            "split" => {
                let direction = self.lower_direction_field(node);
                let source = self.lower_required_needle(node, "source");
                let destination = self.lower_required_needle(node, "destination");
                let carriers = self.lower_optional_carrier_set(node, "carriers");
                WeakBaseOperationKind::Split {
                    direction,
                    source,
                    destination,
                    carriers,
                }
            }
            "pause" => WeakBaseOperationKind::Pause,
            "drop" => WeakBaseOperationKind::Drop {
                needle: self.lower_required_needle(node, "needle"),
            },
            "amiss" => WeakBaseOperationKind::Amiss {
                needle: self.lower_required_needle(node, "needle"),
            },
            "xfer" => WeakBaseOperationKind::Xfer {
                source: self.lower_required_needle(node, "source"),
                destination: self.lower_required_needle(node, "destination"),
            },
            other => unreachable!("lower_base_operation called with non-base node `{other}`"),
        };
        WeakBaseOperation { span, kind }
    }

    fn lower_direction_field(&mut self, parent: Node) -> WeakDirection {
        match self.required_field(parent, "direction") {
            Some(n) => self.lower_direction(n),
            None => WeakDirection::Unknown(self.span_of(parent)),
        }
    }

    // -----------------------------------------------------------------------
    // Extension operations
    // -----------------------------------------------------------------------

    fn lower_extension_operation(&mut self, node: Node) -> WeakExtensionOperation {
        let span = self.span_of(node);
        let kind = match node.kind() {
            "x_stitch_number" => WeakExtensionOperationKind::XStitchNumber {
                value: self
                    .required_field(node, "number")
                    .and_then(|n| self.lower_nonneg_integer(n)),
            },
            "x_speed_number" => WeakExtensionOperationKind::XSpeedNumber {
                value: self
                    .required_field(node, "number")
                    .and_then(|n| self.lower_nonneg_integer(n)),
            },
            "x_sub_roller_number" => WeakExtensionOperationKind::XSubRollerNumber {
                value: self
                    .required_field(node, "number")
                    .and_then(|n| self.lower_nonneg_integer(n)),
            },
            "x_presser_mode" => {
                let mode = match self.required_field(node, "mode") {
                    Some(n) => self.lower_presser_mode(n),
                    None => WeakPresserMode::Unknown(span),
                };
                WeakExtensionOperationKind::XPresserMode { mode }
            }
            "x_xfer_stitch_number" => WeakExtensionOperationKind::XXferStitchNumber {
                value: self
                    .required_field(node, "number")
                    .and_then(|n| self.lower_nonneg_integer(n)),
            },
            "x_end_pass" => WeakExtensionOperationKind::XEndPass,
            "x_vis_color" => {
                let color = self
                    .required_field(node, "color")
                    .map(|n| self.text_of(n).to_owned());
                let carriers = self.lower_required_carrier_set(node, "carriers");
                WeakExtensionOperationKind::XVisColor { color, carriers }
            }
            "x_loop_in" => {
                let direction = self.lower_direction_field(node);
                let needle = self.lower_required_needle(node, "needle");
                let carriers = self.lower_required_carrier_set(node, "carriers");
                WeakExtensionOperationKind::XLoopIn {
                    direction,
                    needle,
                    carriers,
                }
            }
            "x_yarn_in" => {
                let direction = self.lower_direction_field(node);
                let needle = self.lower_required_needle(node, "needle");
                let carriers = self.lower_required_carrier_set(node, "carriers");
                WeakExtensionOperationKind::XYarnIn {
                    direction,
                    needle,
                    carriers,
                }
            }
            "x_yarn_out" => {
                let direction = self.lower_direction_field(node);
                let needle = self.lower_required_needle(node, "needle");
                let carriers = self.lower_required_carrier_set(node, "carriers");
                WeakExtensionOperationKind::XYarnOut {
                    direction,
                    needle,
                    carriers,
                }
            }
            "x_end" => WeakExtensionOperationKind::XEnd,
            "x_roller_advance" => WeakExtensionOperationKind::XRollerAdvance {
                value: self
                    .required_field(node, "amount")
                    .and_then(|n| self.lower_rational(n)),
            },
            "x_add_roller_advance" => WeakExtensionOperationKind::XAddRollerAdvance {
                value: self
                    .required_field(node, "amount")
                    .and_then(|n| self.lower_rational(n)),
            },
            "x_carrier_spacing" => WeakExtensionOperationKind::XCarrierSpacing {
                value: self
                    .required_field(node, "amount")
                    .and_then(|n| self.lower_nonneg_integer(n)),
            },
            "x_carrier_stopping_distance" => WeakExtensionOperationKind::XCarrierStoppingDistance {
                value: self
                    .required_field(node, "amount")
                    .and_then(|n| self.lower_nonneg_half_integer(n)),
            },
            "x_xfer_style" => {
                let style = match self.required_field(node, "style") {
                    Some(n) => self.lower_xfer_style(n),
                    None => WeakXferStyle::Unknown(span),
                };
                WeakExtensionOperationKind::XXferStyle { style }
            }
            "unknown_extension" => {
                let name = self
                    .required_field(node, "name")
                    .map(|n| self.text_of(n).to_owned());
                let mut arguments = Vec::new();
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.kind() == "extension_argument"
                        && !child.is_missing()
                        && !child.is_error()
                    {
                        arguments.push(self.text_of(child).to_owned());
                    }
                }
                WeakExtensionOperationKind::UnknownExtension { name, arguments }
            }
            _ => {
                self.push_error(ParseError::InvalidOperation(span));
                WeakExtensionOperationKind::UnknownExtension {
                    name: None,
                    arguments: Vec::new(),
                }
            }
        };
        WeakExtensionOperation { span, kind }
    }
}
