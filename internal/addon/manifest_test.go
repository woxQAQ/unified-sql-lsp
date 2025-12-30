package addon

import (
	"path/filepath"
	"testing"
)

func TestParseManifest_Valid(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "valid-postgresql")

	manifest, err := ParseManifest(dir)
	if err != nil {
		t.Fatalf("ParseManifest() failed: %v", err)
	}

	if manifest.Name != "postgresql" {
		t.Errorf("expected Name 'postgresql', got '%s'", manifest.Name)
	}

	if manifest.Version != "1.0.0" {
		t.Errorf("expected Version '1.0.0', got '%s'", manifest.Version)
	}

	if manifest.Engine != "PostgreSQL" {
		t.Errorf("expected Engine 'PostgreSQL', got '%s'", manifest.Engine)
	}

	if len(manifest.SupportedVersions) != 3 {
		t.Errorf("expected 3 supported versions, got %d", len(manifest.SupportedVersions))
	}

	if manifest.Wasm.File != "postgresql.wasm" {
		t.Errorf("expected Wasm.File 'postgresql.wasm', got '%s'", manifest.Wasm.File)
	}

	if len(manifest.Capabilities) != 2 {
		t.Errorf("expected 2 capabilities, got %d", len(manifest.Capabilities))
	}
}

func TestParseManifest_NotFound(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "nonexistent")

	_, err := ParseManifest(dir)
	if err == nil {
		t.Fatal("ParseManifest() should fail for nonexistent directory")
	}

	_, ok := err.(*ManifestNotFoundError)
	if !ok {
		t.Errorf("expected ManifestNotFoundError, got %T", err)
	}
}

func TestParseManifest_InvalidYAML(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "invalid-yaml")

	_, err := ParseManifest(dir)
	if err == nil {
		t.Fatal("ParseManifest() should fail for invalid YAML")
	}

	// Invalid YAML can result in either ParseError or ValidationError
	// depending on whether it's a syntax error or fails validation
	switch err.(type) {
	case *ManifestParseError, *ManifestValidationError:
		// Expected error types
	default:
		t.Errorf("expected ManifestParseError or ManifestValidationError, got %T", err)
	}
}

func TestParseManifest_MissingRequiredFields(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "missing-fields")

	_, err := ParseManifest(dir)
	if err == nil {
		t.Fatal("ParseManifest() should fail for missing required fields")
	}

	validationErr, ok := err.(*ManifestValidationError)
	if !ok {
		t.Errorf("expected ManifestValidationError, got %T", err)
		return
	}

	if validationErr.Field != "name" {
		t.Errorf("expected Field 'name', got '%s'", validationErr.Field)
	}
}

func TestParseManifest_WasmNotFound(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "missing-wasm")

	_, err := ParseManifest(dir)
	if err == nil {
		t.Fatal("ParseManifest() should fail for missing Wasm file")
	}

	_, ok := err.(*WasmNotFoundError)
	if !ok {
		t.Errorf("expected WasmNotFoundError, got %T", err)
	}
}

func TestParseManifest_BadEngineType(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "bad-engine-type")

	_, err := ParseManifest(dir)
	if err == nil {
		t.Fatal("ParseManifest() should fail for unsupported engine type")
	}

	validationErr, ok := err.(*ManifestValidationError)
	if !ok {
		t.Errorf("expected ManifestValidationError, got %T", err)
		return
	}

	if validationErr.Field != "engine" {
		t.Errorf("expected Field 'engine', got '%s'", validationErr.Field)
	}
}

func TestParseManifest_BadCapability(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "bad-capability")

	_, err := ParseManifest(dir)
	if err == nil {
		t.Fatal("ParseManifest() should fail for unknown capability")
	}

	validationErr, ok := err.(*ManifestValidationError)
	if !ok {
		t.Errorf("expected ManifestValidationError, got %T", err)
		return
	}

	if validationErr.Field != "capabilities" {
		t.Errorf("expected Field 'capabilities', got '%s'", validationErr.Field)
	}
}

func TestParseManifest_MissingVersions(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "missing-versions")

	_, err := ParseManifest(dir)
	if err == nil {
		t.Fatal("ParseManifest() should fail for empty supported_versions")
	}

	validationErr, ok := err.(*ManifestValidationError)
	if !ok {
		t.Errorf("expected ManifestValidationError, got %T", err)
		return
	}

	if validationErr.Field != "supported_versions" {
		t.Errorf("expected Field 'supported_versions', got '%s'", validationErr.Field)
	}
}

func TestManifest_Path(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "valid-postgresql")

	manifest, err := ParseManifest(dir)
	if err != nil {
		t.Fatalf("ParseManifest() failed: %v", err)
	}

	expectedPath := filepath.Join(dir, "manifest.yaml")
	if manifest.Path() != expectedPath {
		t.Errorf("expected Path '%s', got '%s'", expectedPath, manifest.Path())
	}
}

func TestManifest_WasmPath(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "valid-postgresql")

	manifest, err := ParseManifest(dir)
	if err != nil {
		t.Fatalf("ParseManifest() failed: %v", err)
	}

	expectedPath := filepath.Join(dir, "postgresql.wasm")
	if manifest.WasmPath() != expectedPath {
		t.Errorf("expected WasmPath '%s', got '%s'", expectedPath, manifest.WasmPath())
	}
}

func TestManifest_Dir(t *testing.T) {
	dir := filepath.Join("testdata", "addons", "valid-postgresql")

	manifest, err := ParseManifest(dir)
	if err != nil {
		t.Fatalf("ParseManifest() failed: %v", err)
	}

	if manifest.Dir() != dir {
		t.Errorf("expected Dir '%s', got '%s'", dir, manifest.Dir())
	}
}
