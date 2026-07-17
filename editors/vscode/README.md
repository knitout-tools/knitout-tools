# Knitout for VS Code

A minimal VS Code extension for [knitout][knitout] (`.k`). It:

- registers the `knitout` language and associates `.k` / `.knitout` files with it,
- provides **static syntax highlighting** via a TextMate grammar
  ([`syntaxes/knitout.tmLanguage.json`](syntaxes/knitout.tmLanguage.json)), and
- launches the [`knitout-lsp`](../../crates/knitout-lsp) language server, which supplies
  **diagnostics** and **hover documentation**.

It contributes no other editor features — no semantic tokens, completion, commands, code actions,
or snippets.

[knitout]: https://textiles-lab.github.io/knitout/knitout.html

## Requirements

Build the language server first:

```sh
cargo build --release -p knitout-lsp
```

The `knitout-lsp` binary must be on your `PATH`, or point the extension at it via the
`knitout.server.path` setting (absolute path to the executable).

## Developing

```sh
cd editors/vscode
npm install
npm run compile
```

Then press <kbd>F5</kbd> in VS Code to launch an Extension Development Host, and open a `.k` file.
