# knitout-runtime

Runtime diagnostics for the
[knitout](https://textiles-lab.github.io/knitout/knitout.html) machine
knitting language. See the
[knitout-tools](https://github.com/knitout-tools/knitout-tools) repo.

This crate runs a validated program through the `knitout-semantics` virtual
machine and collects user-facing **runtime** warnings and errors, each carrying
the source span of the operation that produced it. It is the runtime counterpart
to `knitout-weak-ast`'s parse diagnostics, and is intended to feed an LSP.

Unlike a compiler, `analyze` never stops early: it reports every problem it can
find so an editor can surface all of a file's issues at once.

- **Warnings** (the program still runs): operating on an empty needle (e.g.
  knitting where a `tuck` was meant), taking a carrier out before it ever
  stitched, leaving a carrier in action, or leaving the yarn inserting hook
  engaged at the end of the program.
- **Errors** (the program will not run as written): using a carrier that was
  never brought in, bringing a carrier in twice, misusing the hook, a misaligned
  transfer, or a program that is wider than the machine's declared `Width:`.

The semantic bookkeeping lives in `knitout-semantics`; this crate only maps the
machine's observations to spanned, severity-classified diagnostics and adds the
machine-width check.
