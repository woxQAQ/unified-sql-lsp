package lsp

import (
	"context"
	"fmt"

	"github.com/woxQAQ/unified-sql-lsp/internal/addon"
	"github.com/woxQAQ/unified-sql-lsp/internal/config"
	"github.com/woxQAQ/unified-sql-lsp/internal/wasm"
	"go.uber.org/zap"
)

type Server struct {
	cfg         *config.ServerConfig
	logger      *zap.Logger
	wasmRuntime *wasm.Runtime
	addonMgr    *addon.Manager
	hostFuncs   *wasm.HostFunctionsImpl
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

	// Initialize host functions.
	hostFuncs := wasm.NewHostFunctions(logger)

	// Initialize add-on manager.
	addonMgr := addon.NewManager(cfg, wasmRuntime, hostFuncs, logger)

	// Load all add-ons.
	if err := addonMgr.LoadAll(ctx); err != nil {
		logger.Warn("Failed to load add-ons", zap.Error(err))
		// Don't fail server startup - add-ons are optional for MVP
	}

	logger.Info("LSP server initialized",
		zap.Uint32("wasm_memory_pages", cfg.Wasm.MemoryPages),
		zap.String("wasm_cache_dir", cfg.Wasm.CacheDir),
		zap.Int("addons_loaded", addonMgr.Registry().Count()),
	)

	return &Server{
		cfg:         cfg,
		logger:      logger,
		wasmRuntime: wasmRuntime,
		addonMgr:    addonMgr,
		hostFuncs:   hostFuncs,
	}, nil
}

// Close gracefully shuts down the server.
func (s *Server) Close(ctx context.Context) error {
	s.logger.Info("Shutting down LSP server")

	// Shutdown add-on manager.
	if err := s.addonMgr.Shutdown(ctx); err != nil {
		s.logger.Error("Failed to shutdown add-on manager", zap.Error(err))
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
