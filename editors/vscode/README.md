# Knitout for VS Code

A minimal VS Code extension that launches the [`knitout-lsp`](../../crates/knitout-lsp)
language server and displays its diagnostics. It registers the `knitout` language and associates
`.k` / `.knitout` files with it. It contributes no other editor features — no syntax highlighting,
semantic tokens, commands, or code actions. Its only purpose is to start the server and surface
parser/AST diagnostics.

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
