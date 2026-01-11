use tower_lsp::{LspService, Server};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    tracing::info!("Starting Unified SQL LSP server");

    // Create stdin/stdout streams
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Create the LSP service
    let (service, socket) = LspService::new(unified_sql_lsp_lsp::backend::LspBackend::new);

    // Run the server using Server::new
    Server::new(stdin, stdout, socket).serve(service).await;
}
