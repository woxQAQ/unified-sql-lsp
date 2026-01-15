# E2E Test Data Fixtures

This directory contains comprehensive test data fixtures for end-to-end testing of the Unified SQL LSP project.

## Directory Structure

```
fixtures/
├── schema/
│   ├── mysql/
│   │   └── 01_create_tables.sql
│   └── postgresql/
│       └── 01_create_tables.sql
├── data/
│   ├── mysql/
│   │   ├── 02_insert_basic_data.sql
│   │   └── 03_insert_edge_case_data.sql
│   └── postgresql/
│       ├── 02_insert_basic_data.sql
│       └── 03_insert_edge_case_data.sql
├── metadata/
│   ├── mysql/
│   │   ├── users.json
│   │   ├── products.json
│   │   ├── orders.json
│   │   ├── order_items.json
│   │   ├── employees.json
│   │   ├── posts.json
│   │   ├── tags.json
│   │   ├── post_tags.json
│   │   └── logs.json
│   └── postgresql/
│       └── (corresponding .json files)
└── README.md
```

## Overview

The test fixtures are designed to cover:

1. **Basic CRUD Operations**: Simple table structures with various data types
2. **Relationships**: One-to-many, many-to-many, and self-referencing relationships
3. **Edge Cases**: NULL values, boundary conditions, special characters, Unicode
4. **Dialect-Specific Features**: MySQL ENUMs vs PostgreSQL custom types, JSON vs JSONB
5. **Performance Testing**: Medium-sized datasets (100-500 rows for key tables)

## Database Schema

### Basic Tables

#### users
User account information with various data types, timestamps, and account status.

**Columns**: id, username, email, full_name, age, balance, is_active, created_at, updated_at, last_login, bio, profile_image, phone

**PostgreSQL additions**: status (user_status enum), tags (TEXT[]), preferences (JSONB)

**Row count**: 20 basic + 10 edge cases = 30 total

**Tests**:
- VARCHAR length limits
- DECIMAL precision
- BOOLEAN values
- TIMESTAMP handling
- UNIQUE constraints
- Index completion

#### products
Product catalog with pricing, inventory, and categorization.

**Columns**: id, name, description, price, cost, quantity_in_stock, category, is_available, weight, sku, tags, attributes

**PostgreSQL additions**: tags (TEXT[]), attributes (JSONB)

**Row count**: 25 basic + 7 edge cases = 32 total

**Tests**:
- ENUM types (MySQL) / custom types (PostgreSQL)
- DECIMAL precision and constraints
- CHECK constraints (price >= 0, quantity >= 0)
- Category-based filtering
- Stock management queries

#### orders
Order tracking with status workflow and payment information.

**Columns**: id, user_id, order_date, total_amount, status, payment_method, shipping_address, billing_address, notes, metadata, shipped_at, delivered_at

**PostgreSQL additions**: metadata (JSONB)

**Row count**: 30 basic + 10 edge cases = 40 total

**Tests**:
- Foreign key relationships
- ENUM status workflow
- Date range queries
- Status-based filtering
- Composite indexes (user_id, status)
- CHECK constraints (delivered after shipped)

#### order_items
Order line items with quantity discounts and computed subtotal.

**Columns**: id, order_id, product_id, quantity, unit_price, discount_percent, subtotal (computed), notes, metadata

**PostgreSQL additions**: metadata (JSONB)

**Row count**: 50 basic + 8 edge cases = 58 total

**Tests**:
- Composite foreign keys
- Many-to-one relationships
- Generated/computed columns
- Quantity and discount calculations
- UNIQUE constraint on (order_id, product_id)

### Advanced Tables

#### employees
Organizational hierarchy with self-referencing manager relationship.

**Columns**: id, first_name, last_name, email, manager_id, department, position, salary, hire_date, is_active, skills, metadata

**PostgreSQL additions**: skills (TEXT[]), metadata (JSONB)

**Row count**: 12 basic + 7 edge cases = 19 total

**Tests**:
- Self-referencing foreign keys
- Hierarchical queries (recursive CTEs)
- Organizational structure traversal
- Salary range queries
- Department-based grouping

#### posts
Blog/content management with publication workflow.

**Columns**: id, title, slug, content, excerpt, author_id, status, tags, view_count, metadata, published_at, created_at, updated_at

**PostgreSQL additions**: tags (TEXT[]), metadata (JSONB)

**Row count**: 7 basic + 7 edge cases = 14 total

**Tests**:
- URL slug handling
- Publication status workflow
- View count analytics
- Tag-based filtering
- Content search queries
- Publication date ranges

#### tags
Categorization tags for posts.

**Columns**: id, name, slug, description, color, metadata, created_at

**PostgreSQL additions**: metadata (JSONB)

**Row count**: 10 basic + 10 edge cases = 20 total

**Tests**:
- UNIQUE constraints on name and slug
- Color code handling (hex format)
- Simple reference table

#### post_tags
Many-to-many junction table for posts and tags.

**Columns**: post_id, tag_id, tagged_at, metadata

**PostgreSQL additions**: metadata (JSONB)

**Row count**: 25 basic + 5 edge cases = 30 total

**Tests**:
- Composite primary key
- Many-to-many relationships
- Junction table patterns
- CASCADE delete behavior

#### logs
Application logging with time-series data and JSON context.

**Columns**: id, level, message, context, source, created_at

**PostgreSQL additions**: context (JSONB), native table partitioning by year

**Row count**: 30 basic + 10 edge cases = 40 total

**Tests**:
- ENUM log levels
- JSON/JSONB context data
- Time-series queries
- Date range filtering
- High-volume inserts

## Edge Cases Covered

### NULL Values
- NULL in nullable columns
- NULL vs empty string comparisons
- NULL in foreign key columns
- NULL in computed expressions

### Boundary Values
- Minimum/maximum DECIMAL values
- Zero quantities and amounts
- Maximum VARCHAR lengths
- Date/time boundaries

### Special Characters
- Single quotes (O'Connor)
- Double quotes ("quoted")
- Unicode characters (François, 日本語, café)
- Emoji (😀, 🎉)
- Newlines and whitespace

### Data Types
- ENUM/Custom types (status, category, payment_method)
- JSON/JSONB structured data
- ARRAY types (PostgreSQL)
- TIMESTAMP WITH TIME ZONE (PostgreSQL)
- GENERATED/STORED columns

### Constraints
- UNIQUE constraints (single and composite)
- CHECK constraints (positive values, logical conditions)
- FOREIGN KEY constraints (CASCADE, RESTRICT, SET NULL)
- NOT NULL constraints

## Dialect-Specific Features

### MySQL
- ENUM types (inline column definitions)
- AUTO_INCREMENT
- ON UPDATE CURRENT_TIMESTAMP
- SET type (for future use)
- Generated stored columns

### PostgreSQL
- Custom ENUM types (CREATE TYPE)
- SERIAL/BIGSERIAL
- ARRAY types (TEXT[], INTEGER[])
- JSONB with GIN indexes
- TIMESTAMPTZ (TIMESTAMP WITH TIME ZONE)
- Table partitioning (PARTITION BY RANGE)
- TRIGGER functions for automatic timestamp updates
- Materialized views
- Partial indexes (WHERE clause)

## Loading the Test Data

### MySQL

```bash
# Connect to MySQL
mysql -u root -p test_db

# Create schema
source tests/e2e/fixtures/schema/mysql/01_create_tables.sql

# Load basic data
source tests/e2e/fixtures/data/mysql/02_insert_basic_data.sql

# Load edge cases
source tests/e2e/fixtures/data/mysql/03_insert_edge_case_data.sql
```

### PostgreSQL

```bash
# Connect to PostgreSQL
psql -U postgres test_db

# Create schema
\i tests/e2e/fixtures/schema/postgresql/01_create_tables.sql

# Load basic data
\i tests/e2e/fixtures/data/postgresql/02_insert_basic_data.sql

# Load edge cases
\i tests/e2e/fixtures/data/postgresql/03_insert_edge_case_data.sql
```

## Metadata Files

JSON files in `metadata/` directory describe table structures for test assertions:

- `users.json`: User table schema and row counts
- `products.json`: Product catalog schema
- `orders.json`: Order management schema
- `order_items.json`: Order line items schema
- `employees.json`: Employee hierarchy schema
- `posts.json`: Blog posts schema
- `tags.json`: Tag categories schema
- `post_tags.json`: Junction table schema
- `logs.json`: Application logs schema

Each metadata file includes:
- `table_name`: Name of the table
- `columns`: Array of column definitions (name, type, nullable, key, default, extra)
- `indexes`: Array of index definitions (name, columns, type, unique)
- `foreign_keys`: Array of foreign key constraints (if applicable)
- `constraints`: Array of CHECK constraints (if applicable)
- `row_count`: Breakdown of basic vs edge case rows

## Usage in E2E Tests

```python
# Example: Loading test fixtures in Python
import json
import psycopg2

# Load metadata
with open('tests/e2e/fixtures/metadata/postgresql/users.json') as f:
    users_metadata = json.load(f)

# Connect and query
conn = psycopg2.connect('dbname=test_db user=postgres')
cur = conn.cursor()

# Verify row count
cur.execute('SELECT COUNT(*) FROM users')
actual_count = cur.fetchone()[0]
assert actual_count == users_metadata['row_count']['total']

# Verify schema
cur.execute('''
    SELECT column_name, data_type, is_nullable
    FROM information_schema.columns
    WHERE table_name = 'users'
''')
# ... assertions against metadata
```

## Maintenance

When adding new test scenarios:

1. Update the appropriate SQL files in `data/` directories
2. Update row counts in metadata JSON files
3. Document new edge cases in this README
4. Ensure both MySQL and PostgreSQL versions are updated
5. Test loading the fixtures in both database systems

## Test Coverage Goals

The fixtures support testing of:

- **LSP Completion**: Table names, column names, JOIN suggestions
- **LSP Hover**: Type information, constraint details, relationship hints
- **LSP Diagnostics**: Syntax errors, type mismatches, constraint violations
- **Semantic Analysis**: Scope resolution, symbol table building
- **Lowering**: CST to IR conversion for various SQL constructs
- **Catalog Integration**: LiveCatalog and StaticCatalog population

## Notes

- All foreign key relationships are properly indexed
- Test data is deterministic and reproducible
- IDs don't have gaps in basic data (except for auto-increment)
- Edge cases intentionally create gaps and unusual scenarios
- All datetime values use ISO 8601 format
- JSON/JSONB data includes various nesting levels and types
- Email addresses use example.com TLD (RFC 2606)
- Phone numbers follow E.123 format but are fictional
