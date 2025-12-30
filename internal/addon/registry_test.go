package addon

import (
	"testing"

	"go.uber.org/zap"
)

func TestRegistry_Register(t *testing.T) {
	logger := zap.NewNop()
	registry := NewRegistry(logger)

	// Create a mock addon
	manifest := &Manifest{
		Name:   "test-addon",
		Engine: "PostgreSQL",
		dir:    "/tmp/test",
	}

	addon := &Addon{
		Manifest: manifest,
	}

	err := registry.Register(addon)
	if err != nil {
		t.Fatalf("Register() failed: %v", err)
	}

	// Check count
	if registry.Count() != 1 {
		t.Errorf("expected count 1, got %d", registry.Count())
	}
}

func TestRegistry_Duplicate(t *testing.T) {
	logger := zap.NewNop()
	registry := NewRegistry(logger)

	// Create a mock addon
	manifest := &Manifest{
		Name:   "test-addon",
		Engine: "PostgreSQL",
		dir:    "/tmp/test",
	}

	addon1 := &Addon{
		Manifest: manifest,
	}

	addon2 := &Addon{
		Manifest: manifest,
	}

	// Register first addon
	err := registry.Register(addon1)
	if err != nil {
		t.Fatalf("First Register() failed: %v", err)
	}

	// Try to register duplicate
	err = registry.Register(addon2)
	if err == nil {
		t.Fatal("Register() should fail for duplicate add-on")
	}

	_, ok := err.(*AddonAlreadyRegisteredError)
	if !ok {
		t.Errorf("expected AddonAlreadyRegisteredError, got %T", err)
	}
}

func TestRegistry_Get(t *testing.T) {
	logger := zap.NewNop()
	registry := NewRegistry(logger)

	// Create a mock addon
	manifest := &Manifest{
		Name:   "test-addon",
		Engine: "PostgreSQL",
		dir:    "/tmp/test",
	}

	addon := &Addon{
		Manifest: manifest,
	}

	// Try to get before registering
	_, ok := registry.Get("test-addon")
	if ok {
		t.Error("Get() should return false for non-existent add-on")
	}

	// Register addon
	registry.Register(addon)

	// Get after registering
	retrieved, ok := registry.Get("test-addon")
	if !ok {
		t.Fatal("Get() should return true for existing add-on")
	}

	if retrieved.Name() != "test-addon" {
		t.Errorf("expected name 'test-addon', got '%s'", retrieved.Name())
	}
}

func TestRegistry_LookupByEngine(t *testing.T) {
	logger := zap.NewNop()
	registry := NewRegistry(logger)

	// Create mock addons
	pgAddon := &Addon{
		Manifest: &Manifest{
			Name:   "postgresql",
			Engine: "PostgreSQL",
			dir:    "/tmp/pg",
		},
	}

	mysqlAddon := &Addon{
		Manifest: &Manifest{
			Name:   "mysql",
			Engine: "MySQL",
			dir:    "/tmp/mysql",
		},
	}

	// Register addons
	registry.Register(pgAddon)
	registry.Register(mysqlAddon)

	// Lookup PostgreSQL
	pgAddons := registry.LookupByEngine("PostgreSQL")
	if len(pgAddons) != 1 {
		t.Errorf("expected 1 PostgreSQL add-on, got %d", len(pgAddons))
	}

	if len(pgAddons) > 0 && pgAddons[0].Name() != "postgresql" {
		t.Errorf("expected name 'postgresql', got '%s'", pgAddons[0].Name())
	}

	// Lookup MySQL
	mysqlAddons := registry.LookupByEngine("MySQL")
	if len(mysqlAddons) != 1 {
		t.Errorf("expected 1 MySQL add-on, got %d", len(mysqlAddons))
	}

	// Lookup non-existent engine
	sqliteAddons := registry.LookupByEngine("SQLite")
	if len(sqliteAddons) != 0 {
		t.Errorf("expected 0 SQLite add-ons, got %d", len(sqliteAddons))
	}
}

func TestRegistry_List(t *testing.T) {
	logger := zap.NewNop()
	registry := NewRegistry(logger)

	// Initially empty
	list := registry.List()
	if len(list) != 0 {
		t.Errorf("expected 0 add-ons, got %d", len(list))
	}

	// Register addons
	registry.Register(&Addon{
		Manifest: &Manifest{
			Name:   "addon1",
			Engine: "PostgreSQL",
			dir:    "/tmp/a1",
		},
	})

	registry.Register(&Addon{
		Manifest: &Manifest{
			Name:   "addon2",
			Engine: "MySQL",
			dir:    "/tmp/a2",
		},
	})

	// List should return both
	list = registry.List()
	if len(list) != 2 {
		t.Errorf("expected 2 add-ons, got %d", len(list))
	}
}

func TestRegistry_Unregister(t *testing.T) {
	logger := zap.NewNop()
	registry := NewRegistry(logger)

	// Create and register addon
	manifest := &Manifest{
		Name:   "test-addon",
		Engine: "PostgreSQL",
		dir:    "/tmp/test",
	}

	addon := &Addon{
		Manifest: manifest,
	}

	registry.Register(addon)

	// Verify it's registered
	if registry.Count() != 1 {
		t.Errorf("expected count 1, got %d", registry.Count())
	}

	// Unregister
	registry.Unregister("test-addon")

	// Verify it's gone
	if registry.Count() != 0 {
		t.Errorf("expected count 0, got %d", registry.Count())
	}

	_, ok := registry.Get("test-addon")
	if ok {
		t.Error("Get() should return false after unregister")
	}
}
