use std::collections::HashMap;
use std::convert::Infallible;

use either::Either;
use knitout_ast::{Carrier, Program, Span, Validated};
use knitout_semantics::{Backend, Diagnostic, KnitEffect, MachineState, run_knitout};

use crate::diagnostics::{RuntimeError, RuntimeWarning};

/// The runtime diagnostics produced by analyzing a program: warnings (the program still runs) and
/// errors (it does not).
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct RuntimeDiagnostics {
    /// Non-fatal observations.
    pub warnings: Vec<RuntimeWarning>,
    /// Fatal problems that stop the program from running as written.
    pub errors: Vec<RuntimeError>,
}

impl RuntimeDiagnostics {
    /// Whether there are neither warnings nor errors.
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty() && self.errors.is_empty()
    }
}

/// Run the semantics virtual machine over a validated program and collect every runtime warning and
/// error it produces. Unlike a compiler, this never stops early: it reports everything it can so an
/// editor can surface all of a file's problems at once.
pub fn analyze(program: &Program<Validated>) -> RuntimeDiagnostics {
    match run_knitout::<Collector>(program) {
        Ok(diagnostics) => diagnostics,
        Err(never) => match never {},
    }
}

/// The span of the operation at index `i`, if it has one. `i == operations().len()` (used for the
/// end-of-program diagnostics) yields `None`.
fn op_span(program: &Program<Validated>, i: usize) -> Option<Span> {
    program
        .operations()
        .get(i)
        .and_then(|op| op.annotation.span())
}

/// The machine width declared by the `Width:` header, if present.
fn machine_width(program: &Program<Validated>) -> Option<u32> {
    match program.headers() {
        Either::Left(headers) => headers.width().as_ref().map(|w| match w.value.value() {
            Either::Left(width) => *width,
            Either::Right(never) => match *never {},
        }),
        Either::Right(never) => match *never {},
    }
}

/// A [`Backend`] that translates the semantics VM's effects into runtime diagnostics with spans,
/// and additionally checks that the program fits within the machine's declared width.
struct Collector {
    /// The machine width from the `Width:` header, if any. `None` disables the width check.
    machine_width: Option<u32>,
    warnings: Vec<RuntimeWarning>,
    errors: Vec<RuntimeError>,
    /// The span of the `in`/`inhook` that brought each carrier in, so a "left in action" warning
    /// can point back at it.
    in_spans: HashMap<Carrier<Validated>, Option<Span>>,
    /// The span of the `inhook` that engaged the hook, for a "hook left engaged" warning.
    hook_span: Option<Span>,
    /// Whether a [`RuntimeError::MachineTooWide`] has already been reported (report it only once).
    width_reported: bool,
}

impl Collector {
    /// Translate a semantics [`Diagnostic`] into a runtime warning or error and store it.
    fn record(&mut self, diagnostic: Diagnostic, span: Option<Span>) {
        match diagnostic {
            Diagnostic::InactiveCarrier { .. } => {
                self.errors
                    .push(RuntimeError::UseOfInactiveCarrier { span });
            }
            Diagnostic::CarrierAlreadyIn { .. } => {
                self.errors
                    .push(RuntimeError::CarrierBroughtInTwice { span });
            }
            Diagnostic::HookInUse => {
                self.errors.push(RuntimeError::HookAlreadyInUse { span });
            }
            Diagnostic::ReleaseEmptyHook => {
                self.errors.push(RuntimeError::ReleaseWithoutHook { span });
            }
            Diagnostic::ReleaseWrongHook => {
                self.errors
                    .push(RuntimeError::ReleaseOfWrongCarriers { span });
            }
            Diagnostic::OutWhileHooked { .. } => {
                self.errors.push(RuntimeError::TakeOutWhileHooked { span });
            }
            Diagnostic::MisalignedTransfer { .. } => {
                self.errors.push(RuntimeError::MisalignedTransfer { span });
            }
            Diagnostic::EmptyNeedle { .. } => {
                self.warnings
                    .push(RuntimeWarning::OperationOnEmptyNeedle { span });
            }
            Diagnostic::UnstitchedCarrier { .. } => {
                self.warnings
                    .push(RuntimeWarning::UnstitchedCarrier { span });
            }
            Diagnostic::CarrierNeverOut { carrier } => {
                // Point at the `in`/`inhook` that brought the carrier in, not the (span-less) end.
                let span = self.in_spans.get(&carrier).copied().flatten();
                self.warnings
                    .push(RuntimeWarning::CarrierLeftInAction { span });
            }
            Diagnostic::HookNeverReleased => {
                self.warnings.push(RuntimeWarning::HookLeftEngaged {
                    span: self.hook_span,
                });
            }
        }
    }

    /// If the program now spans more needles than the machine is wide, report it (once).
    fn check_width(&mut self, state: &MachineState, span: Option<Span>) {
        if self.width_reported {
            return;
        }
        let Some(width) = self.machine_width else {
            return;
        };
        let (Some(min), Some(max)) = (state.min_index(), state.max_index()) else {
            return;
        };
        let used = (i64::from(max) - i64::from(min) + 1).max(0) as u32;
        if used > width {
            self.errors.push(RuntimeError::MachineTooWide {
                span,
                needles_used: used,
                machine_width: width,
            });
            self.width_reported = true;
        }
    }
}

impl Backend for Collector {
    type Output = RuntimeDiagnostics;
    type Error = Infallible;

    fn register(program: &Program<Validated>) -> Result<Self, Self::Error> {
        Ok(Collector {
            machine_width: machine_width(program),
            warnings: Vec::new(),
            errors: Vec::new(),
            in_spans: HashMap::new(),
            hook_span: None,
            width_reported: false,
        })
    }

    fn observe(
        &mut self,
        effect: KnitEffect,
        state: &MachineState,
        operation_index: usize,
        program: &Program<Validated>,
    ) -> Option<Self::Error> {
        let span = op_span(program, operation_index);
        match effect {
            KnitEffect::CarrierIn { carrier, hooked } => {
                self.in_spans.insert(carrier, span);
                if hooked {
                    self.hook_span = span;
                }
            }
            KnitEffect::Diagnostic(diagnostic) => self.record(diagnostic, span),
            _ => {}
        }
        self.check_width(state, span);
        None
    }

    fn finish(self) -> Result<Self::Output, Self::Error> {
        Ok(RuntimeDiagnostics {
            warnings: self.warnings,
            errors: self.errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knitout_ast::{
        Annotated, BaseOperation, Bed, CarrierTable, CarriersHeaderValue, Direction, Headers,
        IsSlider, MyI32, MyU32, Needle, Operation, Provenance,
    };

    fn front(index: i32) -> Needle<Validated> {
        Needle::new(
            Bed::Front(()),
            IsSlider::NotSlider(()),
            MyI32::new(Either::Left(index), ()),
            (),
        )
    }

    fn right() -> Direction<Validated> {
        Direction::Right(())
    }

    fn cs(carriers: &[Carrier<Validated>]) -> Vec<Either<Carrier<Validated>, knitout_ast::Never>> {
        carriers.iter().map(|c| Either::Left(*c)).collect()
    }

    fn base(op: BaseOperation<Validated>) -> Operation<Validated> {
        Operation::BaseOperation(op)
    }

    fn tuck(n: i32, c: Carrier<Validated>) -> Operation<Validated> {
        base(BaseOperation::Tuck {
            direction: right(),
            needle: Either::Left(front(n)),
            carriers: cs(&[c]),
        })
    }

    /// Build a program with an optional `Width:` header. Operation `i` gets span `[i, i+1)`.
    fn build(
        table: CarrierTable,
        width: Option<u32>,
        ops: Vec<Operation<Validated>>,
    ) -> Program<Validated> {
        let mut headers = Headers::new(Either::Left(Annotated::unit(CarriersHeaderValue::new(
            Either::Left(vec![]),
            (),
        ))));
        if let Some(w) = width {
            headers.set_width(Annotated::new(MyU32::new(Either::Left(w), ()), ()));
        }
        let mut program = Program::new(
            Either::Left(Annotated::unit(2u32)),
            Either::Left(headers),
            Vec::new(),
            table,
        );
        for (i, op) in ops.into_iter().enumerate() {
            program.push_operation(Annotated::new(
                op,
                Provenance::new(Some(Span::new(i, i + 1)), None),
            ));
        }
        program
    }

    fn sp(i: usize) -> Option<Span> {
        Some(Span::new(i, i + 1))
    }

    // --- warnings ---------------------------------------------------------

    #[test]
    fn knit_on_empty_needle_is_a_warning_with_span() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = build(
            table,
            None,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Knit {
                    direction: right(),
                    needle: Either::Left(front(0)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        assert!(d.errors.is_empty());
        assert_eq!(
            d.warnings,
            vec![RuntimeWarning::OperationOnEmptyNeedle { span: sp(1) }]
        );
    }

    #[test]
    fn unstitched_carrier_out_is_a_warning() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = build(
            table,
            None,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        assert!(d.errors.is_empty());
        assert_eq!(
            d.warnings,
            vec![RuntimeWarning::UnstitchedCarrier { span: sp(1) }]
        );
    }

    #[test]
    fn carrier_left_in_action_points_at_the_in() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        // `in` is operation index 1 (after a pause at 0), so its span is [1,2).
        let p = build(
            table,
            None,
            vec![
                base(BaseOperation::Pause),
                base(BaseOperation::In { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        assert_eq!(
            d.warnings,
            vec![RuntimeWarning::CarrierLeftInAction { span: sp(1) }]
        );
    }

    #[test]
    fn hook_left_engaged_points_at_the_inhook() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = build(
            table,
            None,
            vec![
                base(BaseOperation::Inhook { carriers: cs(&[c]) }),
                tuck(0, c),
            ],
        );
        let d = analyze(&p);
        assert!(d.errors.is_empty());
        // both the hook and the carrier are left dangling; the hook warning points at the inhook
        assert!(
            d.warnings
                .contains(&RuntimeWarning::HookLeftEngaged { span: sp(0) })
        );
        assert!(
            d.warnings
                .contains(&RuntimeWarning::CarrierLeftInAction { span: sp(0) })
        );
    }

    // --- errors -----------------------------------------------------------

    #[test]
    fn use_of_inactive_carrier_is_an_error() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = build(table, None, vec![tuck(0, c)]);
        let d = analyze(&p);
        assert_eq!(
            d.errors,
            vec![RuntimeError::UseOfInactiveCarrier { span: sp(0) }]
        );
    }

    #[test]
    fn carrier_brought_in_twice_is_an_error() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = build(
            table,
            None,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        assert!(
            d.errors
                .contains(&RuntimeError::CarrierBroughtInTwice { span: sp(1) })
        );
    }

    #[test]
    fn hook_already_in_use_is_an_error() {
        let mut table = CarrierTable::new();
        let c1 = table.make_carrier("1", ());
        let c2 = table.make_carrier("2", ());
        let p = build(
            table,
            None,
            vec![
                base(BaseOperation::Inhook {
                    carriers: cs(&[c1]),
                }),
                base(BaseOperation::Inhook {
                    carriers: cs(&[c2]),
                }),
            ],
        );
        let d = analyze(&p);
        assert!(
            d.errors
                .contains(&RuntimeError::HookAlreadyInUse { span: sp(1) })
        );
    }

    #[test]
    fn release_without_hook_is_an_error() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = build(
            table,
            None,
            vec![base(BaseOperation::Releasehook { carriers: cs(&[c]) })],
        );
        let d = analyze(&p);
        assert_eq!(
            d.errors,
            vec![RuntimeError::ReleaseWithoutHook { span: sp(0) }]
        );
    }

    #[test]
    fn release_of_wrong_carriers_is_an_error() {
        let mut table = CarrierTable::new();
        let c1 = table.make_carrier("1", ());
        let c2 = table.make_carrier("2", ());
        let p = build(
            table,
            None,
            vec![
                base(BaseOperation::Inhook {
                    carriers: cs(&[c1]),
                }),
                base(BaseOperation::Releasehook {
                    carriers: cs(&[c2]),
                }),
            ],
        );
        let d = analyze(&p);
        assert!(
            d.errors
                .contains(&RuntimeError::ReleaseOfWrongCarriers { span: sp(1) })
        );
    }

    #[test]
    fn take_out_while_hooked_is_an_error() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = build(
            table,
            None,
            vec![
                base(BaseOperation::Inhook { carriers: cs(&[c]) }),
                tuck(0, c),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        assert!(
            d.errors
                .contains(&RuntimeError::TakeOutWhileHooked { span: sp(2) })
        );
    }

    #[test]
    fn misaligned_transfer_is_an_error() {
        let table = CarrierTable::new();
        // same-bed xfer never aligns
        let p = build(
            table,
            None,
            vec![base(BaseOperation::Xfer {
                source: Either::Left(front(0)),
                destination: Either::Left(front(1)),
            })],
        );
        let d = analyze(&p);
        assert_eq!(
            d.errors,
            vec![RuntimeError::MisalignedTransfer { span: sp(0) }]
        );
    }

    // --- machine width ----------------------------------------------------

    #[test]
    fn program_within_width_has_no_width_error() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        // needles 0,1,2 span exactly 3 with a width of 3
        let p = build(
            table,
            Some(3),
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                tuck(0, c),
                tuck(1, c),
                tuck(2, c),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        assert!(d.is_empty());
    }

    #[test]
    fn program_wider_than_machine_is_an_error() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        // needles 0..=3 span 4, exceeding a width of 3; the tuck on needle 3 is operation index 4
        let p = build(
            table,
            Some(3),
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                tuck(0, c),
                tuck(1, c),
                tuck(2, c),
                tuck(3, c),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        assert_eq!(
            d.errors,
            vec![RuntimeError::MachineTooWide {
                span: sp(4),
                needles_used: 4,
                machine_width: 3,
            }]
        );
    }

    #[test]
    fn width_error_reported_only_once() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = build(
            table,
            Some(3),
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                tuck(0, c),
                tuck(1, c),
                tuck(2, c),
                tuck(3, c),
                tuck(4, c),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        let width_errors = d
            .errors
            .iter()
            .filter(|e| matches!(e, RuntimeError::MachineTooWide { .. }))
            .count();
        assert_eq!(width_errors, 1);
        // reported at the first offending op (needle 3, span [4,5)), with the running span of 4
        assert_eq!(
            d.errors[0],
            RuntimeError::MachineTooWide {
                span: sp(4),
                needles_used: 4,
                machine_width: 3,
            }
        );
    }

    #[test]
    fn no_width_header_disables_the_check() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = build(
            table,
            None,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                tuck(0, c),
                tuck(50, c),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        assert!(
            !d.errors
                .iter()
                .any(|e| matches!(e, RuntimeError::MachineTooWide { .. }))
        );
    }

    #[test]
    fn negative_needle_indices_count_toward_width() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        // needles -2 and 2 span 5 (-2..=2), exceeding width 3
        let p = build(
            table,
            Some(3),
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                tuck(-2, c),
                tuck(2, c),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        assert_eq!(
            d.errors,
            vec![RuntimeError::MachineTooWide {
                span: sp(2),
                needles_used: 5,
                machine_width: 3,
            }]
        );
    }

    // --- overall ----------------------------------------------------------

    #[test]
    fn clean_program_produces_nothing() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = build(
            table,
            Some(10),
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                tuck(0, c),
                base(BaseOperation::Knit {
                    direction: right(),
                    needle: Either::Left(front(0)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let d = analyze(&p);
        assert!(d.is_empty(), "unexpected diagnostics: {:?}", d);
    }

    #[test]
    fn empty_diagnostics_is_empty() {
        let d = RuntimeDiagnostics::default();
        assert!(d.is_empty());
        let d = RuntimeDiagnostics {
            warnings: vec![RuntimeWarning::HookLeftEngaged { span: None }],
            errors: Vec::new(),
        };
        assert!(!d.is_empty());
    }
}
