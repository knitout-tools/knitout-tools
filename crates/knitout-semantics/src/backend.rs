use knitout_ast::{Program, Validated};

use crate::{effect::KnitEffect, state::MachineState};

pub trait Backend {
    /// The type returned once the program is finished
    type Output;
    /// The type of error that can be returned by the backend
    type Error;

    /// Create an instance of a backend that will listen to this program's execution. The backend
    /// will likely use this to view the version number, headers, and carrier table. Has the option
    /// to bail out at this point with an error.
    fn register(program: &Program<Validated>) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Observe an effect, with read access to the machine state, the program, and the index in the
    /// operations where the effect happened. The machine state before the effect happens. If an
    /// error is returned, the observation is cancelled.
    fn observe(
        &mut self,
        effect: KnitEffect,
        state: &MachineState,
        operation_index: usize,
        program: &Program<Validated>,
    ) -> Option<Self::Error>;

    /// After the program is finished, return the output of the backend. Can optionally fail.
    fn finish(self) -> Result<Self::Output, Self::Error>;
}

/// Run a knitout program on the backend, returning either the intended result or an error
pub fn run_knitout<B: Backend>(program: &Program<Validated>) -> Result<B::Output, B::Error> {
    MachineState::execute::<B>(program)
}
