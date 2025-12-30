package lsp

import (
	"context"
	"fmt"

	"github.com/woxQAQ/unified-sql-lsp/internal/config"
	"github.com/woxQAQ/unified-sql-lsp/internal/wasm"
	"go.uber.org/zap"
)

type Server struct {
	cfg         *config.ServerConfig
	logger      *zap.Logger
	wasmRuntime *wasm.Runtime
}

func NewServer(ctx context.Context, cfg *config.ServerConfig, logger *zap.Logger) (*Server, error) {
	// Initialize Wasm runtime.
	wasmConfig := &wasm.RuntimeConfig{
		MemoryPages:  cfg.Wasm.MemoryPages,
		DebugEnabled: cfg.Wasm.Debug,
		CacheDir:     cfg.Wasm.CacheDir,
		MaxInstances: cfg.Wasm.MaxInstances,
	}

	wasmRuntime, err := wasm.NewRuntime(ctx, logger, wasmConfig)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize Wasm runtime: %w", err)
	}

	logger.Info("LSP server initialized",
		zap.Uint32("wasm_memory_pages", cfg.Wasm.MemoryPages),
		zap.String("wasm_cache_dir", cfg.Wasm.CacheDir),
	)

	return &Server{
		cfg:         cfg,
		logger:      logger,
		wasmRuntime: wasmRuntime,
	}, nil
}

// Close gracefully shuts down the server.
func (s *Server) Close(ctx context.Context) error {
	s.logger.Info("Shutting down LSP server")

	// Shutdown Wasm runtime.
	if err := s.wasmRuntime.Close(ctx); err != nil {
		s.logger.Error("Failed to shutdown Wasm runtime", zap.Error(err))
		return err
	}

	s.logger.Info("LSP server shutdown complete")
	return nil
}

func (s *Server) ServeStdio(ctx context.Context) error {
	// TODO: Implement stdio LSP server
	return nil
}

func (s *Server) ServeTCP(ctx context.Context, port int) error {
	// TODO: Implement TCP LSP server
	return nil
}
