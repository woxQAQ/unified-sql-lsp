use std::env;

#[tokio::main]
async fn main() {
    eprintln!("!!! LSP SERVER: Starting up");

    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check for --tcp flag
    let tcp_port = args
        .iter()
        .position(|arg| arg == "--tcp")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|port_str| port_str.parse::<u16>().ok());

    // Check for --catalog flag
    let catalog_name = args
        .iter()
        .position(|arg| arg == "--catalog")
        .and_then(|idx| args.get(idx + 1));

    if let Some(port) = tcp_port {
        // Run in TCP mode
        eprintln!("!!! LSP SERVER: Running in TCP mode on port {}", port);

        // Log catalog if specified
        if let Some(catalog) = catalog_name {
            eprintln!("!!! LSP SERVER: Using catalog: {}", catalog);
        }

        // Initialize stderr logging for TCP mode (safe since stdout not used)
        tracing_subscriber::fmt()
            .with_env_filter("unified_sql_lsp=debug,tower_lsp=debug")
            .with_writer(std::io::stderr)
            .init();

        // Load static catalog
        let catalog = std::sync::Arc::new(unified_sql_lsp_catalog::StaticCatalog::new());
        let server = unified_sql_lsp_lsp::tcp::TcpServer::new(port, catalog)
            .await
            .expect("Failed to start TCP server");

        server.serve().await.expect("TCP server error");
    } else {
        // Run in stdio mode (default)
        eprintln!("!!! LSP SERVER: Running in stdio mode");

        // Log catalog if specified
        if let Some(catalog) = catalog_name {
            eprintln!("!!! LSP SERVER: Using catalog: {}", catalog);
        }

        // DO NOT initialize logging - it interferes with JSON-RPC protocol on stdout
        // The LSP protocol requires stdout to be used exclusively for JSON-RPC messages
        // Logs should only go to stderr, but tracing_subscriber doesn't respect this in all cases
        // So we completely disable logging when running as LSP server

        use tower_lsp::{LspService, Server};

        // Create stdin/stdout streams
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        // Create the LSP service
        let (service, socket) = LspService::new(unified_sql_lsp_lsp::backend::LspBackend::new);

        // Run the server using Server::new
        Server::new(stdin, stdout, socket).serve(service).await;
    }
}
