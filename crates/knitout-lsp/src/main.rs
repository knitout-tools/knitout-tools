//! Entry point for the knitout language server.
//!
//! Speaks LSP over stdio, so an editor extension can launch this binary and forward traffic to it.

mod backend;
mod docs;
mod hover;
mod messages;
mod positions;

use backend::Backend;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
