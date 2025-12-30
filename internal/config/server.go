package config

import (
	"github.com/spf13/viper"
)

type ServerConfig struct {
	AddonPaths     []string `mapstructure:"addon_paths"`
	LogLevel       string   `mapstructure:"log_level"`
	MetricsEnabled bool     `mapstructure:"metrics_enabled"`
	MetricsPort    int      `mapstructure:"metrics_port"`
}

func LoadServerConfig(configPath string) (*ServerConfig, error) {
	v := viper.New()

	// Set defaults
	v.SetDefault("addon_paths", []string{"./addons"})
	v.SetDefault("log_level", "info")
	v.SetDefault("metrics_enabled", false)
	v.SetDefault("metrics_port", 9090)

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
