use knitout_ast::{Operation, Program};

use crate::state::MachineState;

pub trait Backend {
    /// The type returned once the program is finished
    type Output;
    /// The type of error that can be returned by the backend
    type Error;

    /// Register a program with the backend. The backend should use this to view the version number,
    /// headers, and carrier table. Has the option to bail out at this point with an error.
    fn register(&mut self, program: &Program) -> Option<Self::Error>;

    /// Observe an operation, its effect, and the machine state after the operation.
    fn observe(
        &mut self,
        op: &Operation,
        effect: KnitEffect,
        state: &MachineState,
    ) -> Option<Self::Error>;

    /// Observe a diagnostic message.
    fn diagnostic(&mut self, message: SemanticDiagnostic) -> Option<Self::Error>;

    /// After the program is finished, return the output of the backend.
    fn finish(self) -> Result<Self::Output, Self::Error>;
}
