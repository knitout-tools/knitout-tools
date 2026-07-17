//! Runtime diagnostics for knitout programs.
//!
//! This crate runs a validated [`knitout_ast::Program`] through the
//! [`knitout_semantics`] virtual machine and collects user-facing runtime warnings and errors with
//! source spans, ready to be handed to an editor/LSP. It is the runtime counterpart to
//! `knitout-weak-ast`'s parse diagnostics.
//!
//! The semantics VM tracks only loop *counts*; the [`model`] module layers a reusable loop-lineage
//! model on top of it (driven purely by the VM's effects) so richer, history-aware diagnostics can
//! be computed without duplicating the machine semantics. Thresholds live in [`Thresholds`].

mod analysis;
mod config;
mod diagnostics;
mod model;

pub use analysis::{RuntimeDiagnostics, analyze, analyze_with};
pub use config::Thresholds;
pub use diagnostics::{FloatAxis, RuntimeError, RuntimeWarning};
