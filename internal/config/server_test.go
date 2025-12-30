package config

import (
	"os"
	"testing"
)

// Basic smoke tests for F001 initialization
// These tests provide basic validation of config loading functionality.
// TODO: F005 (Connection Manager) will add comprehensive tests including:
// - Error handling (invalid YAML, permission errors)
// - Edge cases (empty files, malformed configs)
// - Config merging and overrides
// - Environment variable integration

func TestLoadServerConfigDefaults(t *testing.T) {
	cfg, err := LoadServerConfig("")
	if err != nil {
		t.Fatalf("Failed to load config: %v", err)
	}

	if cfg.LogLevel != "info" {
		t.Errorf("Default log level mismatch: got %s, want info", cfg.LogLevel)
	}

	if cfg.MetricsEnabled {
		t.Errorf("Metrics should be disabled by default")
	}

	if cfg.MetricsPort != 9090 {
		t.Errorf("Default metrics port mismatch: got %d, want 9090", cfg.MetricsPort)
	}

	if len(cfg.AddonPaths) != 1 || cfg.AddonPaths[0] != "./addons" {
		t.Errorf("Default addon paths mismatch: got %v, want [./addons]", cfg.AddonPaths)
	}
}

func TestLoadServerConfigFromFile(t *testing.T) {
	// Create temporary config file
	tmpfile, err := os.CreateTemp("", "config*.yaml")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(tmpfile.Name())

	configContent := `
log_level: debug
metrics_enabled: true
metrics_port: 8080
`
	if _, err := tmpfile.Write([]byte(configContent)); err != nil {
		t.Fatal(err)
	}
	if err := tmpfile.Close(); err != nil {
		t.Fatal(err)
	}

	cfg, err := LoadServerConfig(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to load config: %v", err)
	}

	if cfg.LogLevel != "debug" {
		t.Errorf("Log level mismatch: got %s, want debug", cfg.LogLevel)
	}

	if cfg.MetricsPort != 8080 {
		t.Errorf("Metrics port mismatch: got %d, want 8080", cfg.MetricsPort)
	}
}
