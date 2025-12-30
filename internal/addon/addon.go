package addon

import (
	"time"

	"github.com/woxQAQ/unified-sql-lsp/internal/wasm"
)

// Addon represents a loaded add-on with its manifest and compiled Wasm module.
type Addon struct {
	// Manifest is the parsed add-on metadata
	Manifest *Manifest

	// Compiled is the compiled Wasm module
	Compiled *wasm.CompiledModule

	// LoadedAt is the timestamp when the add-on was loaded
	LoadedAt time.Time
}

// Name returns the add-on name.
func (a *Addon) Name() string {
	return a.Manifest.Name
}

// Engine returns the database engine this add-on supports.
func (a *Addon) Engine() string {
	return a.Manifest.Engine
}

// Version returns the add-on version.
func (a *Addon) Version() string {
	return a.Manifest.Version
}

// Capabilities returns the list of capabilities provided by this add-on.
func (a *Addon) Capabilities() []string {
	return a.Manifest.Capabilities
}

// SupportsVersion checks if the add-on supports a specific database version.
func (a *Addon) SupportsVersion(version string) bool {
	for _, v := range a.Manifest.SupportedVersions {
		if v == version {
			return true
		}
	}
	return false
}
