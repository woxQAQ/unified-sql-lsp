package addon

import (
	"fmt"
	"os"
	"path/filepath"

	"gopkg.in/yaml.v3"
)

// Manifest represents the add-on manifest.yaml structure.
type Manifest struct {
	Name              string            `yaml:"name"`
	Version           string            `yaml:"version"`
	Engine            string            `yaml:"engine"`
	SupportedVersions []string          `yaml:"supported_versions"`
	Wasm              WasmConfig        `yaml:"wasm"`
	Capabilities      []string          `yaml:"capabilities"`
	Author            string            `yaml:"author"`
	License           string            `yaml:"license"`

	// Internal fields
	dir string // Directory containing manifest
}

// WasmConfig holds Wasm module configuration.
type WasmConfig struct {
	File string `yaml:"file"`
	Size int    `yaml:"size"` // KB
}

// ParseManifest reads and parses manifest.yaml from a directory.
func ParseManifest(dir string) (*Manifest, error) {
	manifestPath := filepath.Join(dir, "manifest.yaml")

	data, err := os.ReadFile(manifestPath)
	if err != nil {
		return nil, &ManifestNotFoundError{
			Path: manifestPath,
			Err:  err,
		}
	}

	var m Manifest
	if err := yaml.Unmarshal(data, &m); err != nil {
		return nil, &ManifestParseError{
			Path: manifestPath,
			Err:  err,
		}
	}

	m.dir = dir

	// Validate manifest
	if err := m.Validate(); err != nil {
		return nil, err
	}

	return &m, nil
}

// Validate checks manifest fields.
func (m *Manifest) Validate() error {
	// Check required fields
	if m.Name == "" {
		return &ManifestValidationError{
			Path:    m.Path(),
			Field:   "name",
			Message: "name is required",
		}
	}

	if m.Version == "" {
		return &ManifestValidationError{
			Path:    m.Path(),
			Field:   "version",
			Message: "version is required",
		}
	}

	if m.Engine == "" {
		return &ManifestValidationError{
			Path:    m.Path(),
			Field:   "engine",
			Message: "engine is required",
		}
	}

	// Validate engine type
	validEngines := map[string]bool{
		"PostgreSQL": true,
		"MySQL":      true,
	}
	if !validEngines[m.Engine] {
		return &ManifestValidationError{
			Path:    m.Path(),
			Field:   "engine",
			Message: fmt.Sprintf("unsupported engine: %s (must be one of: PostgreSQL, MySQL)", m.Engine),
		}
	}

	if len(m.SupportedVersions) == 0 {
		return &ManifestValidationError{
			Path:    m.Path(),
			Field:   "supported_versions",
			Message: "at least one supported version is required",
		}
	}

	if m.Wasm.File == "" {
		return &ManifestValidationError{
			Path:    m.Path(),
			Field:   "wasm.file",
			Message: "wasm.file is required",
		}
	}

	// Validate capabilities
	if len(m.Capabilities) == 0 {
		return &ManifestValidationError{
			Path:    m.Path(),
			Field:   "capabilities",
			Message: "at least one capability is required",
		}
	}

	validCaps := map[string]bool{
		"completion":           true,
		"diagnostics":          true,
		"schema_introspection": true,
	}
	for _, cap := range m.Capabilities {
		if !validCaps[cap] {
			return &ManifestValidationError{
				Path:    m.Path(),
				Field:   "capabilities",
				Message: fmt.Sprintf("unknown capability: %s (must be one of: completion, diagnostics, schema_introspection)", cap),
			}
		}
	}

	// Validate Wasm file exists
	wasmPath := m.WasmPath()
	if _, err := os.Stat(wasmPath); os.IsNotExist(err) {
		return &WasmNotFoundError{
			ManifestPath: m.Path(),
			WasmFile:     m.Wasm.File,
		}
	}

	return nil
}

// Path returns the manifest file path.
func (m *Manifest) Path() string {
	return filepath.Join(m.dir, "manifest.yaml")
}

// WasmPath returns the absolute path to the Wasm file.
func (m *Manifest) WasmPath() string {
	return filepath.Join(m.dir, m.Wasm.File)
}

// Dir returns the directory containing the manifest.
func (m *Manifest) Dir() string {
	return m.dir
}
