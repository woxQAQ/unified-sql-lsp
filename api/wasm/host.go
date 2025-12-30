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
	// TODO: F012 will enhance with connection pooling, prepared statements, query logging
	ExecuteQuery(ctx context.Context, db *sql.DB, query string) (*sql.Rows, error)
}

// SchemaInfo represents database schema metadata
// TODO: F012 will add full support for sequences, materialized views, enums, triggers
type SchemaInfo struct {
	CatalogName string
	SchemaName  string
	Tables      []TableInfo
	Views       []ViewInfo     // Separate from tables for clarity
	Sequences   []SequenceInfo // PostgreSQL sequences
	Functions   []FunctionInfo
	Types       []TypeInfo
	Enums       []EnumInfo // PostgreSQL enum types
}

// TableInfo represents table metadata
type TableInfo struct {
	Name           string
	Schema         string
	Columns        []ColumnInfo
	IsView         bool
	IsSystem       bool
	IsMaterialized bool     // For PostgreSQL materialized views
	PrimaryKey     []string // Column names in primary key
	ForeignKeys    []ForeignKeyInfo
	Indexes        []IndexInfo
	// TODO: F012 will add inheritance info, partition info, check constraints
}

// ColumnInfo represents column metadata
type ColumnInfo struct {
	Name         string
	DataType     string
	Nullable     bool
	DefaultValue *string // NULL if no default
	IsPrimaryKey bool
	IsUnique     bool
	IsIndexed    bool
	// TODO: F012 will add check constraints, generated columns, collation
}

// FunctionInfo represents function metadata
type FunctionInfo struct {
	Name        string
	Schema      string
	Args        []FunctionArg
	ReturnType  string
	IsAggregate bool   // For aggregate functions like SUM, COUNT
	IsWindow    bool   // For window functions
	Volatility  string // IMMUTABLE, STABLE, or VOLATILE
	// TODO: F012 will add parallel safety, support for variadic args, table functions
}

// FunctionArg represents a function argument
type FunctionArg struct {
	Name    string
	Type    string
	Default *string
}

// TypeInfo represents type metadata
type TypeInfo struct {
	Name        string
	Schema      string
	Category    string       // 'A' for array, 'C' for composite, 'E' for enum, etc.
	ElementType *string      // For array types
	Fields      []ColumnInfo // For composite types
	Labels      []string     // For enum types
}

// ViewInfo represents view metadata
type ViewInfo struct {
	Name        string
	Schema      string
	Columns     []ColumnInfo
	IsSystem    bool
	IsUpdatable bool
	// TODO: F012 will add view definition query, check options
}

// SequenceInfo represents sequence metadata
type SequenceInfo struct {
	Name       string
	Schema     string
	StartValue int64
	Increment  int64
	MaxValue   int64
	MinValue   int64
	Cycle      bool // Maps to PostgreSQL's CYCLE option (NO CYCLE is default)
	// See: https://www.postgresql.org/docs/current/sql-createsequence.html
}

// EnumInfo represents enum type metadata
type EnumInfo struct {
	Name   string
	Schema string
	Labels []string
}

// ForeignKeyInfo represents foreign key metadata
type ForeignKeyInfo struct {
	Columns    []string
	RefTable   string
	RefColumns []string
	OnDelete   string
	OnUpdate   string
}

// IndexInfo represents index metadata
type IndexInfo struct {
	Name      string
	Columns   []string
	IsUnique  bool
	IsPrimary bool
	// TODO: F012 will add index type (btree, hash, gist, etc.), partial indexes, expression indexes
}

// Additional schema metadata types will be added in F012 (PostgreSQL Schema Introspection)
// as needed, including: triggers, rules, policies, extensions, and other database objects
