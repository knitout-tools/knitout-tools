//! Runtime diagnostics for knitout programs.
//!
//! This crate runs a validated [`knitout_ast::Program`] through the
//! [`knitout_semantics`] virtual machine and collects user-facing runtime warnings and errors with
//! source spans, ready to be handed to an editor/LSP. It is the runtime counterpart to
//! `knitout-weak-ast`'s parse diagnostics.

mod analysis;
mod diagnostics;

pub use analysis::{RuntimeDiagnostics, analyze};
pub use diagnostics::{RuntimeError, RuntimeWarning};
