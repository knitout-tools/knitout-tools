# tree-sitter-knitout

A tree-sitter parser for the
[knitout](https://textiles-lab.github.io/knitout/knitout.html) machine
knitting language. See the
[knitout-tools](https://github.com/knitout-tools/knitout-tools) repo.

The grammar has been designed to be extremely permissive; downstream tools
should walk the CST and validate it (for example, there is no check that a
magic header is present; however, it does sort ";!" tag-style comments from
regular ones"). The nice thing about this is that there are few (perhaps no?)
ways that the CST can have ERROR or MISSING nodes; there is much opportunity
for helpful error messages.

There's two things to update when releasing a new version in
`tree-sitter-knitout`: the `Cargo.toml` and `tree-sitter.json`.

Run tests with `tree-sitter test`. You'll almost certainly want to always
generate before testing: `tree-sitter generate && tree-sitter test`.

Highlights are included. They are position-based more than they are
parse-based: the argument for `rack` will be colored as a number, regardless of
whether it is indeed a number or not. This is a side effect of how permissive
the tree-sitter grammar is.

