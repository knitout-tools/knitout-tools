# knitout-weak-ast

A crate that specifies possibly invalid abstract syntax trees (ASTs) for the
[knitout](https://textiles-lab.github.io/knitout/knitout.html) machine
knitting language. It is part of the
[knitout-tools](https://github.com/knitout-tools/knitout-tools) repo.

This crate defines two functionalities: walking tree-sitter-knitout's CST to
always yield a potentially invalid, or *weak* (i.e., some malformed nodes are missing,
some unrecognized symbols) AST, and functionality that converts a potentially
invalid AST to, perhaps, a valid (or *strong*) AST.

It does this by introducing the type bundle `Parsed: Phase`, where `Phase` is
from the knitout-ast crate. The weak AST is exactly a `Program<Parsed>`,
whereas knitout-ast works with a `Program<Valid>`. A key difference is that
`Parsed::Other = ParseError`, which means that nodes throughout the AST can be
`ParseError`s instead of their expected data.

This crate exists to enable tools that want to provide service on malformed
knitout files, such as formatters. It should not be used by, say, a compiler,
where completely valid input is paramount.

As the CST is walked to create a weak AST, this crate also emits diagnostics
that point out invalid and missing nodes.

