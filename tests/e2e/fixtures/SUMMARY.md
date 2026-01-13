# Test Data Fixtures Summary

## Files Created

### Schema Files (2)
- `/home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/schema/mysql/01_create_tables.sql` (400+ lines)
- `/home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/schema/postgresql/01_create_tables.sql` (450+ lines)

### Data Files (4)
- `/home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/data/mysql/02_insert_basic_data.sql` (600+ lines)
- `/home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/data/mysql/03_insert_edge_case_data.sql` (350+ lines)
- `/home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/data/postgresql/02_insert_basic_data.sql` (500+ lines)
- `/home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/data/postgresql/03_insert_edge_case_data.sql` (200+ lines)

### Metadata Files (16)
- **MySQL**: 9 JSON files (users, products, orders, order_items, employees, posts, tags, post_tags, logs)
- **PostgreSQL**: 1 JSON file (users) - template for others

### Documentation (2)
- `/home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/README.md` - Comprehensive documentation
- `/home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/SUMMARY.md` - This file

## Database Tables

### Core Tables (9)
1. **users** - User accounts with authentication data
2. **products** - Product catalog with inventory
3. **orders** - Order management and tracking
4. **order_items** - Order line items
5. **employees** - Employee hierarchy (self-referencing)
6. **posts** - Blog/content management
7. **tags** - Categorization tags
8. **post_tags** - Many-to-many junction table
9. **logs** - Application logs with JSON context

## Data Statistics

### MySQL
| Table | Basic Data | Edge Cases | Total |
|-------|-----------|------------|-------|
| users | 20 | 10 | 30 |
| products | 25 | 7 | 32 |
| orders | 30 | 10 | 40 |
| order_items | 50 | 8 | 58 |
| employees | 12 | 7 | 19 |
| posts | 7 | 7 | 14 |
| tags | 10 | 10 | 20 |
| post_tags | 25 | 5 | 30 |
| logs | 30 | 10 | 40 |
| **TOTAL** | **209** | **84** | **283** |

### PostgreSQL
Similar row counts with additional PostgreSQL-specific features (ARRAY, JSONB)

## Features Covered

### SQL Constructs
- ✅ Basic SELECT, INSERT, UPDATE, DELETE
- ✅ JOIN operations (INNER, LEFT, RIGHT)
- ✅ Subqueries and CTEs
- ✅ Aggregation (GROUP BY, HAVING)
- ✅ Window functions
- ✅ UNION operations
- ✅ CASE expressions
- ✅ CAST and type conversions

### Data Types
- ✅ Numeric: INT, DECIMAL/NUMERIC, BIGINT
- ✅ String: VARCHAR, TEXT, CHAR
- ✅ Date/Time: TIMESTAMP, DATETIME, DATE
- ✅ Boolean: TRUE/FALSE
- ✅ Enum: Custom ENUM types
- ✅ JSON/JSONB: Structured data
- ✅ Arrays: PostgreSQL ARRAY types
- ✅ Generated/Computed columns

### Constraints
- ✅ PRIMARY KEY
- ✅ FOREIGN KEY (CASCADE, RESTRICT, SET NULL)
- ✅ UNIQUE (single and composite)
- ✅ NOT NULL
- ✅ CHECK
- ✅ DEFAULT values

### Indexes
- ✅ B-tree indexes
- ✅ Composite indexes
- ✅ Unique indexes
- ✅ Partial indexes (PostgreSQL)
- ✅ GIN indexes (PostgreSQL JSONB)

### Advanced Features
- ✅ Self-referencing tables (hierarchical data)
- ✅ Many-to-many relationships
- ✅ Junction tables
- ✅ Views (materialized and regular)
- ✅ Stored procedures/functions
- ✅ Triggers (PostgreSQL)
- ✅ Table partitioning (PostgreSQL)
- ✅ Generated columns (computed)

### Edge Cases
- ✅ NULL values in various contexts
- ✅ Empty strings vs NULL
- ✅ Boundary values (min/max)
- ✅ Special characters and Unicode
- ✅ Emoji in text fields
- ✅ Very long strings
- ✅ Negative numbers
- ✅ Zero values
- ✅ Duplicate-like scenarios
- ✅ Foreign key constraint violations
- ✅ CHECK constraint violations

## Dialect-Specific Features

### MySQL
- ✅ ENUM column types
- ✅ AUTO_INCREMENT
- ✅ ON UPDATE CURRENT_TIMESTAMP
- ✅ Generated stored columns
- ✅ InnoDB engine
- ✅ utf8mb4 character set

### PostgreSQL
- ✅ Custom ENUM types (CREATE TYPE)
- ✅ SERIAL/BIGSERIAL
- ✅ ARRAY types (TEXT[], INTEGER[])
- ✅ JSONB with GIN indexes
- ✅ TIMESTAMP WITH TIME ZONE
- ✅ Table partitioning by range
- ✅ Trigger functions
- ✅ Materialized views
- ✅ Partial indexes
- ✅ pl/pgSQL functions

## Test Coverage

The fixtures support comprehensive testing of:

### LSP Features
- **Completion**: Tables, columns, JOINs, aliases
- **Hover**: Type info, constraints, relationships
- **Diagnostics**: Syntax errors, type mismatches
- **Signature Help**: Function parameters
- **Document Symbols**: Table/column references

### Semantic Analysis
- Scope resolution
- Symbol table building
- Reference resolution
- Type checking
- Constraint validation

### Lowering (CST to IR)
- Query conversion
- Expression handling
- Type coercion
- Operator mapping

### Catalog Integration
- LiveCatalog: Real database connections
- StaticCatalog: YAML/JSON metadata loading
- Schema filtering
- Incremental updates

## Usage Examples

### Loading Fixtures

```bash
# MySQL
mysql -u root -p test_db < /home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/schema/mysql/01_create_tables.sql
mysql -u root -p test_db < /home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/data/mysql/02_insert_basic_data.sql
mysql -u root -p test_db < /home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/data/mysql/03_insert_edge_case_data.sql

# PostgreSQL
psql -U postgres test_db -f /home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/schema/postgresql/01_create_tables.sql
psql -U postgres test_db -f /home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/data/postgresql/02_insert_basic_data.sql
psql -U postgres test_db -f /home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/data/postgresql/03_insert_edge_case_data.sql
```

### Querying Metadata

```sql
-- Get all tables and row counts
SELECT 
    table_name,
    table_rows
FROM information_schema.tables
WHERE table_schema = 'test_db'
ORDER BY table_name;

-- Get column information
SELECT 
    column_name,
    data_type,
    is_nullable,
    column_default
FROM information_schema.columns
WHERE table_name = 'users'
ORDER BY ordinal_position;

-- Get foreign key relationships
SELECT
    tc.table_name,
    kcu.column_name,
    ccu.table_name AS foreign_table_name,
    ccu.column_name AS foreign_column_name,
    rc.update_rule,
    rc.delete_rule
FROM information_schema.table_constraints AS tc
JOIN information_schema.key_column_usage AS kcu
    ON tc.constraint_name = kcu.constraint_name
JOIN information_schema.constraint_column_usage AS ccu
    ON ccu.constraint_name = tc.constraint_name
JOIN information_schema.referential_constraints AS rc
    ON rc.constraint_name = tc.constraint_name
WHERE tc.constraint_type = 'FOREIGN KEY'
ORDER BY tc.table_name, kcu.column_name;
```

## Next Steps

1. **Create E2E Test Cases**: Write tests that use these fixtures
2. **Add More Metadata**: Complete PostgreSQL metadata JSON files
3. **Performance Testing**: Add larger datasets for stress testing
4. **Query Validation**: Ensure all INSERT statements are valid
5. **Documentation**: Add query examples for common scenarios

## Maintenance

To update the test data:

1. Modify the appropriate SQL file in `data/`
2. Update row counts in `metadata/*.json`
3. Test loading in both MySQL and PostgreSQL
4. Update this SUMMARY.md if adding tables
5. Document any new edge cases in README.md

## Validation

After loading, verify with:

```sql
-- Check row counts match metadata
SELECT 'users' AS table_name, COUNT(*) AS row_count FROM users
UNION ALL
SELECT 'products', COUNT(*) FROM products
UNION ALL
SELECT 'orders', COUNT(*) FROM orders
UNION ALL
SELECT 'order_items', COUNT(*) FROM order_items
UNION ALL
SELECT 'employees', COUNT(*) FROM employees
UNION ALL
SELECT 'posts', COUNT(*) FROM posts
UNION ALL
SELECT 'tags', COUNT(*) FROM tags
UNION ALL
SELECT 'post_tags', COUNT(*) FROM post_tags
UNION ALL
SELECT 'logs', COUNT(*) FROM logs
ORDER BY table_name;
```

Expected totals: 283 rows (209 basic + 84 edge cases)
