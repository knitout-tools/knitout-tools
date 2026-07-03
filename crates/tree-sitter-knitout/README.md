# tree-sitter-knitout

A tree-sitter parser for the
[knitout](https://textiles-lab.github.io/knitout/knitout.html) machine
knitting language. See the
[knitout-tools](https://github.com/knitout-tools/knitout-tools) repo.

There's two things to update when releasing a new version in `tree-sitter-knitout`: the `Cargo.toml` and `tree-sitter.json`.

Run tests with `tree-sitter test`. You'll almost certainly want to always generate before testing: `tree-sitter generate && tree-sitter test`.

