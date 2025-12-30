package lsp

import (
	"context"

	"github.com/vibe-kanban/unified-sql-lsp/internal/config"
	"go.uber.org/zap"
)

type Server struct {
	cfg    *config.ServerConfig
	logger *zap.Logger
}

func NewServer(ctx context.Context, cfg *config.ServerConfig, logger *zap.Logger) (*Server, error) {
	return &Server{
		cfg:    cfg,
		logger: logger,
	}, nil
}

func (s *Server) ServeStdio(ctx context.Context) error {
	// TODO: Implement stdio LSP server
	return nil
}

func (s *Server) ServeTCP(ctx context.Context, port int) error {
	// TODO: Implement TCP LSP server
	return nil
}
