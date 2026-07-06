# knitout-lsp

A [Language Server Protocol][lsp] server for [knitout][knitout] (`.k`), the machine-knitting
language. It surfaces **parser and AST diagnostics** in your editor as you type, and provides
**hover documentation** for knitout constructs.

It maintains, for every open document, the document text and a [tree-sitter][ts] parse tree. On
open and on every change it re-parses the document, runs [`knitout-weak-ast`](../knitout-weak-ast)
over the resulting syntax tree, and publishes the collected warnings (e.g. duplicate headers,
unrecognized extensions) and errors (e.g. missing `;!knitout-N` version line, missing `Carriers:`
header, bad needles/opcodes) as LSP diagnostics. Argument-count errors (too many / missing
arguments) are enriched with the operation's signature and an example.

### Features

- **Diagnostics** — parser and AST warnings/errors, published on open and change.
- **Hover** — hovering shows documentation with a representative example for:
  - operations (both base and `x-` extension operations),
  - needles (`f10`, `b-3`, `fs5`, …),
  - the `+` / `-` direction,
  - `;;`-headers (`Carriers`, `Yarn-…`, `Machine`, `Gauge`, `Width`, `Position`, extensions),
  - the `;!knitout-N` magic version line.

  Hovering a number or carrier inside an operation shows that operation's documentation.

It does **not** implement completion, formatting, code actions, symbols, rename, references, or
semantic tokens. (Static syntax highlighting is provided separately — see below.) The LSP methods
it handles are:

- `initialize` / `initialized`
- `shutdown`
- `textDocument/didOpen`
- `textDocument/didChange`
- `textDocument/didClose`
- `textDocument/hover`
- `textDocument/publishDiagnostics`

All editable text lives in two places, so it is easy to reword:

- [`src/messages.rs`](src/messages.rs) maps each parse-warning/error variant to a single
  human-readable diagnostic message.
- [`src/docs.rs`](src/docs.rs) holds the hover documentation — one `Doc` (summary, signature,
  example, description) per operation, needle, direction, header kind, and the magic line.

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

### Neovim: language server

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

Diagnostics appear inline, and `K` (or `vim.lsp.buf.hover()`) shows the hover documentation.

### Neovim: syntax highlighting (tree-sitter)

The language server does **not** provide highlighting. In Neovim, use the
[`tree-sitter-knitout`](../tree-sitter-knitout) grammar together with its
[`queries/highlights.scm`](../tree-sitter-knitout/queries/highlights.scm) via
[`nvim-treesitter`](https://github.com/nvim-treesitter/nvim-treesitter).

Register the parser and map the filetype, then install it:

```lua
local parsers = require("nvim-treesitter.parsers").get_parser_configs()
parsers.knitout = {
  install_info = {
    -- Point this at your checkout of this repository.
    url = "https://github.com/knitout-tools/knitout-tools",
    location = "crates/tree-sitter-knitout",
    files = { "src/parser.c" },
    branch = "main",
  },
  filetype = "knitout",
}

vim.filetype.add({ extension = { k = "knitout", knitout = "knitout" } })
```

```vim
:TSInstall knitout
```

Then copy the grammar's highlight query where Neovim can find it, e.g.
`~/.config/nvim/queries/knitout/highlights.scm`:

```scheme
; extends
```

followed by the contents of
[`crates/tree-sitter-knitout/queries/highlights.scm`](../tree-sitter-knitout/queries/highlights.scm)
(or simply copy that file to the path above). Highlighting turns on with `:TSBufEnable highlight`
or automatically once the parser and query are installed.

## VS Code extension

A minimal extension that registers the `knitout` language, associates `.k` / `.knitout` files with
it, launches this server (diagnostics + hover), and ships a TextMate grammar for static syntax
highlighting lives in [`editors/vscode`](../../editors/vscode). It adds no other editor features
(no semantic tokens, completion, commands, etc.).

1. Build the server (see [Building](#building) above) and make sure `knitout-lsp` is on your
   `PATH`, or set the `knitout.server.path` setting to the absolute path of the binary.
2. Install the extension's dependencies and compile it:

   ```sh
   cd editors/vscode
   npm install
   npm run compile
   ```

3. Open the `editors/vscode` folder in VS Code and press <kbd>F5</kbd> to launch an Extension
   Development Host, then open a `.k` file. Syntax highlighting applies immediately; parser/AST
   diagnostics appear inline and in the Problems panel; hovering shows documentation.

[lsp]: https://microsoft.github.io/language-server-protocol/
[knitout]: https://textiles-lab.github.io/knitout/knitout.html
[ts]: https://tree-sitter.github.io/tree-sitter/
