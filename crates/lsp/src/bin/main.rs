use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    eprintln!("!!! LSP SERVER: Starting up");

    // DO NOT initialize logging - it interferes with JSON-RPC protocol on stdout
    // The LSP protocol requires stdout to be used exclusively for JSON-RPC messages
    // Logs should only go to stderr, but tracing_subscriber doesn't respect this in all cases
    // So we completely disable logging when running as LSP server

    // Create stdin/stdout streams
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Create the LSP service
    let (service, socket) = LspService::new(unified_sql_lsp_lsp::backend::LspBackend::new);

    // Run the server using Server::new
    Server::new(stdin, stdout, socket).serve(service).await;
}
