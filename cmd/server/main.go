package main

import (
	"context"
	"flag"
	"os"
	"os/signal"
	"syscall"

	"github.com/vibe-kanban/unified-sql-lsp/internal/config"
	"github.com/vibe-kanban/unified-sql-lsp/internal/lsp"
	"go.uber.org/zap"
)

var (
	version = "dev"
	commit  = "none"
	date    = "unknown"
)

func main() {
	// Parse command-line flags
	configPath := flag.String("config", "", "Path to configuration file")
	logLevel := flag.String("log-level", "info", "Log level (debug, info, warn, error)")
	port := flag.Int("port", 0, "TCP port for LSP server (0 for stdio)")
	flag.Parse()

	// Initialize logger
	logger := zap.L()
	if *logLevel == "debug" {
		logger, _ = zap.NewDevelopment()
	} else {
		logger, _ = zap.NewProduction()
	}

	defer logger.Sync()

	logger.Info("Starting unified-sql-lsp",
		zap.String("version", version),
		zap.String("commit", commit),
		zap.String("date", date),
	)

	// Load configuration
	cfg, err := config.LoadServerConfig(*configPath)
	if err != nil {
		logger.Fatal("Failed to load configuration", zap.Error(err))
	}

	// Create context with cancellation
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Initialize LSP server
	server, err := lsp.NewServer(ctx, cfg, logger)
	if err != nil {
		logger.Fatal("Failed to create server", zap.Error(err))
	}

	// Handle shutdown signals
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		sig := <-sigChan
		logger.Info("Received shutdown signal", zap.String("signal", sig.String()))
		cancel()
	}()

	// Start server (stdio or TCP)
	if *port > 0 {
		if err := server.ServeTCP(ctx, *port); err != nil {
			logger.Fatal("TCP server error", zap.Error(err))
		}
	} else {
		if err := server.ServeStdio(ctx); err != nil {
			logger.Fatal("Stdio server error", zap.Error(err))
		}
	}

	logger.Info("Server shutdown complete")
}
