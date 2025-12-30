package addon

import (
	"fmt"
)

// ManifestNotFoundError occurs when manifest.yaml is not found in a directory.
type ManifestNotFoundError struct {
	Path string
	Err  error
}

func (e *ManifestNotFoundError) Error() string {
	return fmt.Sprintf("manifest not found at '%s': %v", e.Path, e.Err)
}

func (e *ManifestNotFoundError) Unwrap() error {
	return e.Err
}

// ManifestParseError occurs when manifest.yaml cannot be parsed as valid YAML.
type ManifestParseError struct {
	Path string
	Err  error
}

func (e *ManifestParseError) Error() string {
	return fmt.Sprintf("failed to parse manifest at '%s': %v", e.Path, e.Err)
}

func (e *ManifestParseError) Unwrap() error {
	return e.Err
}

// ManifestValidationError occurs when manifest.yaml fails validation.
type ManifestValidationError struct {
	Path    string
	Field   string
	Message string
}

func (e *ManifestValidationError) Error() string {
	if e.Field != "" {
		return fmt.Sprintf("manifest validation failed at '%s': %s (field: %s)",
			e.Path, e.Message, e.Field)
	}
	return fmt.Sprintf("manifest validation failed at '%s': %s", e.Path, e.Message)
}

// WasmNotFoundError occurs when the Wasm file referenced in manifest doesn't exist.
type WasmNotFoundError struct {
	ManifestPath string
	WasmFile     string
}

func (e *WasmNotFoundError) Error() string {
	return fmt.Sprintf("Wasm file '%s' not found (referenced in manifest '%s')",
		e.WasmFile, e.ManifestPath)
}

// AddonLoadError occurs when add-on loading fails.
type AddonLoadError struct {
	AddonName string
	Err       error
}

func (e *AddonLoadError) Error() string {
	return fmt.Sprintf("failed to load add-on '%s': %v", e.AddonName, e.Err)
}

func (e *AddonLoadError) Unwrap() error {
	return e.Err
}

// AddonNotFoundError occurs when an add-on is not found in the registry.
type AddonNotFoundError struct {
	AddonName string
}

func (e *AddonNotFoundError) Error() string {
	return fmt.Sprintf("add-on '%s' not found", e.AddonName)
}

// AddonAlreadyRegisteredError occurs when attempting to register a duplicate add-on.
type AddonAlreadyRegisteredError struct {
	AddonName string
}

func (e *AddonAlreadyRegisteredError) Error() string {
	return fmt.Sprintf("add-on '%s' is already registered", e.AddonName)
}

// NoAddonsFoundError occurs when no add-ons are found in the configured paths.
type NoAddonsFoundError struct {
	Paths []string
}

func (e *NoAddonsFoundError) Error() string {
	return fmt.Sprintf("no add-ons found in paths: %v", e.Paths)
}
