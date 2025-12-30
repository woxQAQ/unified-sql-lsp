package config

import (
	"github.com/spf13/viper"
)

type ServerConfig struct {
	AddonPaths     []string   `mapstructure:"addon_paths"`
	LogLevel       string     `mapstructure:"log_level"`
	MetricsEnabled bool       `mapstructure:"metrics_enabled"`
	MetricsPort    int        `mapstructure:"metrics_port"`
	Wasm           WasmConfig `mapstructure:"wasm"`
}

// WasmConfig holds Wasm runtime configuration.
type WasmConfig struct {
	// Memory limit per module (in pages, 64KB each).
	MemoryPages uint32 `mapstructure:"memory_pages"`
	// Enable debug logging.
	Debug bool `mapstructure:"debug"`
	// Compilation cache directory.
	CacheDir string `mapstructure:"cache_dir"`
	// Maximum concurrent instances.
	MaxInstances int `mapstructure:"max_instances"`
	// Module execution timeout (seconds).
	ExecutionTimeout int `mapstructure:"execution_timeout"`
}

func LoadServerConfig(configPath string) (*ServerConfig, error) {
	v := viper.New()

	// Set defaults
	v.SetDefault("addon_paths", []string{"./addons"})
	v.SetDefault("log_level", "info")
	v.SetDefault("metrics_enabled", false)
	v.SetDefault("metrics_port", 9090)

	// Wasm defaults
	v.SetDefault("wasm.memory_pages", 256) // 16MB
	v.SetDefault("wasm.debug", false)
	v.SetDefault("wasm.cache_dir", "./build/wasm-cache")
	v.SetDefault("wasm.max_instances", 100)
	v.SetDefault("wasm.execution_timeout", 30)

	if configPath != "" {
		v.SetConfigFile(configPath)
		if err := v.ReadInConfig(); err != nil {
			return nil, err
		}
	}

	var cfg ServerConfig
	if err := v.Unmarshal(&cfg); err != nil {
		return nil, err
	}

	return &cfg, nil
}
