# knitout-lsp

A [Language Server Protocol][lsp] server for [knitout][knitout] (`.k`), the machine-knitting
language. Its sole job is to surface **parser and AST diagnostics** in your editor as you type.

It maintains, for every open document, the document text and a [tree-sitter][ts] parse tree. On
open and on every change it re-parses the document, runs [`knitout-weak-ast`](../knitout-weak-ast)
over the resulting syntax tree, and publishes the collected warnings (e.g. duplicate headers,
unrecognized extensions) and errors (e.g. missing `;!knitout-N` version line, missing `Carriers:`
header, bad needles/opcodes) as LSP diagnostics. The parsed program itself is discarded — this
server does not validate, compile, or run knitout.

It deliberately implements **only** diagnostics. There is no syntax highlighting, completion,
hover, formatting, code actions, symbols, rename, or references. The LSP methods it handles are:

- `initialize` / `initialized`
- `shutdown`
- `textDocument/didOpen`
- `textDocument/didChange`
- `textDocument/didClose`
- `textDocument/publishDiagnostics`

Diagnostic wording lives in one place — [`src/messages.rs`](src/messages.rs) — which maps each
parse-warning/error variant to a single human-readable string, so messages are easy to edit.

## Building

From the workspace root:

```sh
cargo build --release -p knitout-lsp
```

This produces the server binary at `target/release/knitout-lsp`.

## Running

The server speaks LSP over **stdin/stdout**, so it is normally launched by an editor rather than
run by hand. Any LSP client can start it by executing the `knitout-lsp` binary and forwarding
JSON-RPC traffic over its stdio.

To make it available to editors, either put the binary on your `PATH`:

```sh
cargo install --path crates/knitout-lsp   # installs `knitout-lsp` into ~/.cargo/bin
```

or point your editor/extension at the absolute path of the built binary.

### Neovim example

```lua
vim.filetype.add({ extension = { k = "knitout", knitout = "knitout" } })

vim.api.nvim_create_autocmd("FileType", {
  pattern = "knitout",
  callback = function(args)
    vim.lsp.start({
      name = "knitout-lsp",
      cmd = { "knitout-lsp" }, -- or an absolute path to the binary
      root_dir = vim.fs.dirname(args.file),
    })
  end,
})
```

## VS Code extension

A minimal extension that registers the `knitout` language, associates `.k` / `.knitout` files with
it, launches this server, and displays its diagnostics lives in
[`editors/vscode`](../../editors/vscode). It adds no other editor features.

1. Build the server (see [Building](#building) above) and make sure `knitout-lsp` is on your
   `PATH`, or set the `knitout.server.path` setting to the absolute path of the binary.
2. Install the extension's dependencies and compile it:

   ```sh
   cd editors/vscode
   npm install
   npm run compile
   ```

3. Open the `editors/vscode` folder in VS Code and press <kbd>F5</kbd> to launch an Extension
   Development Host, then open a `.k` file. Parser/AST diagnostics appear inline and in the
   Problems panel.

[lsp]: https://microsoft.github.io/language-server-protocol/
[knitout]: https://textiles-lab.github.io/knitout/knitout.html
[ts]: https://tree-sitter.github.io/tree-sitter/
