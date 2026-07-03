# knitout-semantics

An analyzer for the
[knitout](https://textiles-lab.github.io/knitout/knitout.html) machine
knitting language. See the
[knitout-tools](https://github.com/knitout-tools/knitout-tools) repo.

This crate consumes parsed ASTs from `knitout-ast`, keeping track of the state
of a virtual machine. It defines a trait `Backend` for other tools (LSPs,
compilers) to implement: backends observe the virtual machine as it executes,
receiving read access to the virtual machine state, the executed instruction,
any warnings/errors, and the effects emitted by running code. These backends
then mutate themselves. When the program is complete, the backend's `finish`
method is called, cleaning it up and returning its main result.

When the virtual machine encounters errors that are considered fatal (e.g., a
`xfer` between misaligned needles), it emits a fatal diagnostic, but does not
stop: it is up to the backend to cease communication.

Roughly speaking, the VM does not track "machine-specific" information like
carrier positions; only the information required to do machine-independent
semantic analysis is tracked. Backends hoping for more information can track
that themselves alongside the machine state.

The main purpose of this crate is to avoid code duplication and sync issues
between LSP code analysis and compiler code analysis.

