package addon

import (
	"sync"

	"go.uber.org/zap"
)

// Registry manages loaded add-ons.
type Registry struct {
	sync.RWMutex
	addons   map[string]*Addon     // name -> addon
	byEngine map[string][]*Addon   // engine -> addons
	logger   *zap.Logger
}

// NewRegistry creates a new add-on registry.
func NewRegistry(logger *zap.Logger) *Registry {
	return &Registry{
		addons:   make(map[string]*Addon),
		byEngine: make(map[string][]*Addon),
		logger:   logger.With(zap.String("component", "addon-registry")),
	}
}

// Register adds an add-on to the registry.
func (r *Registry) Register(addon *Addon) error {
	r.Lock()
	defer r.Unlock()

	name := addon.Manifest.Name

	// Check for duplicates
	if _, exists := r.addons[name]; exists {
		return &AddonAlreadyRegisteredError{AddonName: name}
	}

	r.addons[name] = addon

	// Index by engine
	engine := addon.Manifest.Engine
	r.byEngine[engine] = append(r.byEngine[engine], addon)

	r.logger.Info("Add-on registered",
		zap.String("name", name),
		zap.String("engine", engine),
	)

	return nil
}

// Get retrieves an add-on by name.
func (r *Registry) Get(name string) (*Addon, bool) {
	r.RLock()
	defer r.RUnlock()

	addon, ok := r.addons[name]
	return addon, ok
}

// LookupByEngine finds add-ons for a database engine.
func (r *Registry) LookupByEngine(engine string) []*Addon {
	r.RLock()
	defer r.RUnlock()

	addons, ok := r.byEngine[engine]
	if !ok || len(addons) == 0 {
		return []*Addon{}
	}
	// Return copy to avoid race conditions
	result := make([]*Addon, len(addons))
	copy(result, addons)
	return result
}

// List returns all registered add-ons.
func (r *Registry) List() []*Addon {
	r.RLock()
	defer r.RUnlock()

	result := make([]*Addon, 0, len(r.addons))
	for _, addon := range r.addons {
		result = append(result, addon)
	}
	return result
}

// Unregister removes an add-on from the registry.
func (r *Registry) Unregister(name string) {
	r.Lock()
	defer r.Unlock()

	addon, ok := r.addons[name]
	if !ok {
		return
	}

	// Remove from engine index
	engine := addon.Manifest.Engine
	addons := r.byEngine[engine]
	for i, a := range addons {
		if a.Manifest.Name == name {
			// Remove from slice
			r.byEngine[engine] = append(addons[:i], addons[i+1:]...)
			break
		}
	}

	// Remove from main map
	delete(r.addons, name)

	r.logger.Info("Add-on unregistered", zap.String("name", name))
}

// Count returns the number of registered add-ons.
func (r *Registry) Count() int {
	r.RLock()
	defer r.RUnlock()

	return len(r.addons)
}
