// Minimal VS Code extension: it launches the knitout language server and forwards LSP traffic to
// it. It contributes no editor features of its own — diagnostics come entirely from the server.

import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext): void {
  // Resolve the server executable: an explicit configured path, or `knitout-lsp` on PATH.
  const configured = vscode.workspace
    .getConfiguration("knitout")
    .get<string>("server.path");
  const command = configured && configured.length > 0 ? configured : "knitout-lsp";

  const serverOptions: ServerOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "knitout" }],
  };

  client = new LanguageClient(
    "knitout-lsp",
    "Knitout Language Server",
    serverOptions,
    clientOptions,
  );

  // Starting the client also launches the server and begins forwarding LSP traffic.
  client.start();
  context.subscriptions.push(client);
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
