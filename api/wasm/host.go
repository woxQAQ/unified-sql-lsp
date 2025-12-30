//go:build !wasm

package wasm

import (
	"context"
	"database/sql"
)

// HostFunctions defines the interface for host-provided functions
type HostFunctions interface {
	// Schema introspection
	GetSchema(ctx context.Context, db *sql.DB, schemaName string) (*SchemaInfo, error)

	// Logging
	LogMessage(level, ptr, length uint32)

	// Database queries
	ExecuteQuery(ctx context.Context, db *sql.DB, query string) (*sql.Rows, error)
}

// SchemaInfo represents database schema metadata
type SchemaInfo struct {
	CatalogName string
	SchemaName  string
	Tables      []TableInfo
	Functions   []FunctionInfo
	Types       []TypeInfo
}

// TableInfo represents table metadata
type TableInfo struct {
	Name     string
	Columns  []ColumnInfo
	IsView   bool
	IsSystem bool
}

// ColumnInfo represents column metadata
type ColumnInfo struct {
	Name     string
	DataType string
	Nullable bool
}

// FunctionInfo represents function metadata
type FunctionInfo struct {
	Name       string
	Args       []string
	ReturnType string
}

// TypeInfo represents type metadata
type TypeInfo struct {
	Name   string
	Fields []ColumnInfo
}
