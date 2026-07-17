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
find so an editor can surface all of a file's issues at once. There are only two
levels: **warnings** (nonfatal — the program still runs, even if the result is
undesirable) and **errors** (fatal — the program cannot be compiled).

## Architecture

`knitout-semantics` is kept deliberately small: it tracks loop *counts* per
needle. Several of these diagnostics need loop *identity* and history, so the
[`model`](src/model.rs) module layers a reusable loop-lineage model on top of the
semantics VM, driven purely by the VM's effects (it never re-implements the
machine semantics). Each loop is tagged with its carrier, generation (a proxy for
"row"), transfer and split-through counts, whether it ever pulled through another
loop, and where it was made.

## Warnings

- **No yarn carrier** — a knit/tuck/split written with no carriers (use
  `drop`/`amiss`/`xfer`).
- **Miss with no yarn** — a `miss` with no carriers does nothing.
- **Knitting through nothing** — a knit on a needle that holds no loops.
- **Transferring nothing** — a split/xfer whose source holds no loops.
- **Long float** — a carrier used far from its previous stitch.
- **Max. hold loops** — too many loops stacked on one needle.
- **Transfer with overlapped loops** — a stack of loops transferred at once.
- **Continuous pick-up stitch** — new loops on two adjacent needles (laddering).
- **Drop after pick-up** — a tuck knit off in the opposite direction (a `drop`).
- **Unnecessary loop remain** — loops left on the machine at the end.
- **Loop hold error** — a loop held while its carrier knits many further rows.
- **Continuous transfer without knitting** — one loop transferred many times.
- **Unstable split knit** — one loop split-through more than once.
- **Transfer with pick-up stitch** — transferring a loop that never pulled
  through another loop.
- **Racking wearing out yarn** — two yarn-adjacent loops pulled far apart.
- **Pulldown / take-down overload** — a loop moved a long way laterally (SWG-N).
- **Unstable yarn feeding** — knitting off the hook/grippers onto the far bed
  (SWG-N).
- **Slack of edge stitch** — knitting a racked tube edge.
- plus carrier-lifecycle warnings (unstitched carrier, carrier left in action,
  hook left engaged).

## Errors

- **Use of inactive carrier** — a carrier used before it was brought in.
- **Transfer at wrong racking** — needles that don't line up at the racking.
- **Transfer with wrong beds** — not `f`↔`b`, `f`↔`bs`, or `fs`↔`b`.
- **Machine too wide** — the program spans more needles than the `Width:` header.
- plus hook/carrier misuse errors carried over from the base analysis.

## Thresholds

The "how far / how many / how long" limits live in [`Thresholds`](src/config.rs)
and are easy to adjust in one place. `analyze` uses the defaults; `analyze_with`
takes a custom set. Several items are SWG/SWG-N-specific heuristics — see the
doc comments on the individual checks.
