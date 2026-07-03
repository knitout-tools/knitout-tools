# knitout-weak-ast

A crate that specifies possibly invalid abstract syntax trees (ASTs) for the
[knitout](https://textiles-lab.github.io/knitout/knitout.html) machine
knitting language. It is part of the
[knitout-tools](https://github.com/knitout-tools/knitout-tools) repo.

This crate defines two functionalities: walking tree-sitter-knitout's CST to
always yield a potentially invalid, or *weak* (i.e., some malformed nodes are missing,
some unrecognized symbols) AST, and functionality that converts a potentially
invalid AST to, perhaps, a valid (or *strong*) AST.

This crate exists to enable tools that want to provide service on malformed
knitout files, such as formatters. It should not be used by, say, a compiler,
where completely valid input is paramount.

As the CST is walked to create a weak AST, this crate also emits diagnostics
that point out invalid and missing nodes.

