#!/bin/bash
# Script to generate metadata JSON files for all tables
# Usage: ./generate_metadata.sh

METADATA_DIR_MYSQL="/home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/metadata/mysql"
METADATA_DIR_PG="/home/woxQAQ/unified-sql-lsp/tests/e2e/fixtures/metadata/postgresql"

# Create MySQL metadata files
cat > "$METADATA_DIR_MYSQL/products.json" << 'EOF'
{
  "table_name": "products",
  "columns": [
    {"name": "id", "type": "INT", "nullable": false, "key": "PRIMARY KEY", "extra": "AUTO_INCREMENT"},
    {"name": "name", "type": "VARCHAR(100)", "nullable": false, "key": null},
    {"name": "description", "type": "TEXT", "nullable": true, "key": null},
    {"name": "price", "type": "DECIMAL(10, 2)", "nullable": false, "key": "INDEX"},
    {"name": "cost", "type": "DECIMAL(10, 2)", "nullable": true, "key": null},
    {"name": "quantity_in_stock", "type": "INT", "nullable": true, "key": null, "default": "0"},
    {"name": "category", "type": "ENUM", "nullable": true, "key": "INDEX", "values": ["electronics", "clothing", "books", "home", "sports", "toys"]},
    {"name": "is_available", "type": "BOOLEAN", "nullable": true, "key": "INDEX", "default": "TRUE"},
    {"name": "weight", "type": "DECIMAL(8, 3)", "nullable": true, "key": null},
    {"name": "sku", "type": "VARCHAR(50)", "nullable": true, "key": "UNIQUE"},
    {"name": "created_at", "type": "TIMESTAMP", "nullable": true, "default": "CURRENT_TIMESTAMP"},
    {"name": "updated_at", "type": "TIMESTAMP", "nullable": true, "default": "CURRENT_TIMESTAMP", "extra": "ON UPDATE CURRENT_TIMESTAMP"}
  ],
  "constraints": [
    {"name": "chk_price_positive", "type": "CHECK", "definition": "price >= 0"},
    {"name": "chk_quantity_non_negative", "type": "CHECK", "definition": "quantity_in_stock >= 0"},
    {"name": "chk_cost_not_greater_than_price", "type": "CHECK", "definition": "cost IS NULL OR cost <= price"}
  ],
  "row_count": {"basic": 25, "edge_cases": 7, "total": 32}
}
EOF

cat > "$METADATA_DIR_MYSQL/orders.json" << 'EOF'
{
  "table_name": "orders",
  "columns": [
    {"name": "id", "type": "INT", "nullable": false, "key": "PRIMARY KEY", "extra": "AUTO_INCREMENT"},
    {"name": "user_id", "type": "INT", "nullable": false, "key": "INDEX"},
    {"name": "order_date", "type": "DATETIME", "nullable": false, "key": "INDEX", "default": "CURRENT_TIMESTAMP"},
    {"name": "total_amount", "type": "DECIMAL(12, 2)", "nullable": false, "key": "INDEX"},
    {"name": "status", "type": "ENUM", "nullable": false, "key": "INDEX", "default": "'pending'", "values": ["pending", "processing", "shipped", "delivered", "cancelled", "refunded"]},
    {"name": "payment_method", "type": "ENUM", "nullable": true, "key": null, "values": ["credit_card", "debit_card", "paypal", "bank_transfer", "cash"]},
    {"name": "shipping_address", "type": "TEXT", "nullable": true, "key": null},
    {"name": "billing_address", "type": "TEXT", "nullable": true, "key": null},
    {"name": "notes", "type": "TEXT", "nullable": true, "key": null},
    {"name": "shipped_at", "type": "DATETIME", "nullable": true, "key": null},
    {"name": "delivered_at", "type": "DATETIME", "nullable": true, "key": null}
  ],
  "foreign_keys": [
    {"name": "fk_orders_user_id", "columns": ["user_id"], "ref_table": "users", "ref_columns": ["id"], "on_delete": "RESTRICT", "on_update": "CASCADE"}
  ],
  "constraints": [
    {"name": "chk_total_amount_positive", "type": "CHECK", "definition": "total_amount >= 0"},
    {"name": "chk_delivered_after_shipped", "type": "CHECK", "definition": "delivered_at IS NULL OR shipped_at IS NOT NULL"}
  ],
  "row_count": {"basic": 30, "edge_cases": 10, "total": 40}
}
EOF

cat > "$METADATA_DIR_MYSQL/order_items.json" << 'EOF'
{
  "table_name": "order_items",
  "columns": [
    {"name": "id", "type": "INT", "nullable": false, "key": "PRIMARY KEY", "extra": "AUTO_INCREMENT"},
    {"name": "order_id", "type": "INT", "nullable": false, "key": "UNIQUE"},
    {"name": "product_id", "type": "INT", "nullable": false, "key": "UNIQUE"},
    {"name": "quantity", "type": "INT", "nullable": false, "key": null, "default": "1"},
    {"name": "unit_price", "type": "DECIMAL(10, 2)", "nullable": false, "key": null},
    {"name": "discount_percent", "type": "DECIMAL(5, 2)", "nullable": true, "key": null, "default": "0.00"},
    {"name": "subtotal", "type": "DECIMAL(10, 2)", "nullable": true, "key": null, "extra": "GENERATED ALWAYS AS"},
    {"name": "notes", "type": "TEXT", "nullable": true, "key": null}
  ],
  "foreign_keys": [
    {"name": "fk_order_items_order_id", "columns": ["order_id"], "ref_table": "orders", "ref_columns": ["id"], "on_delete": "CASCADE", "on_update": "CASCADE"},
    {"name": "fk_order_items_product_id", "columns": ["product_id"], "ref_table": "products", "ref_columns": ["id"], "on_delete": "RESTRICT", "on_update": "CASCADE"}
  ],
  "row_count": {"basic": 50, "edge_cases": 8, "total": 58}
}
EOF

cat > "$METADATA_DIR_MYSQL/employees.json" << 'EOF'
{
  "table_name": "employees",
  "columns": [
    {"name": "id", "type": "INT", "nullable": false, "key": "PRIMARY KEY", "extra": "AUTO_INCREMENT"},
    {"name": "first_name", "type": "VARCHAR(50)", "nullable": false, "key": null},
    {"name": "last_name", "type": "VARCHAR(50)", "nullable": false, "key": "INDEX"},
    {"name": "email", "type": "VARCHAR(100)", "nullable": false, "key": "UNIQUE"},
    {"name": "manager_id", "type": "INT", "nullable": true, "key": "INDEX"},
    {"name": "department", "type": "VARCHAR(50)", "nullable": true, "key": "INDEX"},
    {"name": "position", "type": "VARCHAR(100)", "nullable": true, "key": null},
    {"name": "salary", "type": "DECIMAL(12, 2)", "nullable": true, "key": null},
    {"name": "hire_date", "type": "DATE", "nullable": false, "key": null},
    {"name": "is_active", "type": "BOOLEAN", "nullable": true, "key": null, "default": "TRUE"}
  ],
  "foreign_keys": [
    {"name": "fk_employees_manager_id", "columns": ["manager_id"], "ref_table": "employees", "ref_columns": ["id"], "on_delete": "SET NULL", "on_update": "CASCADE"}
  ],
  "row_count": {"basic": 12, "edge_cases": 7, "total": 19}
}
EOF

cat > "$METADATA_DIR_MYSQL/posts.json" << 'EOF'
{
  "table_name": "posts",
  "columns": [
    {"name": "id", "type": "INT", "nullable": false, "key": "PRIMARY KEY", "extra": "AUTO_INCREMENT"},
    {"name": "title", "type": "VARCHAR(200)", "nullable": false, "key": null},
    {"name": "slug", "type": "VARCHAR(200)", "nullable": true, "key": "UNIQUE"},
    {"name": "content", "type": "TEXT", "nullable": false, "key": null},
    {"name": "excerpt", "type": "TEXT", "nullable": true, "key": null},
    {"name": "author_id", "type": "INT", "nullable": false, "key": "INDEX"},
    {"name": "status", "type": "ENUM", "nullable": true, "key": "INDEX", "default": "'draft'", "values": ["draft", "published", "archived"]},
    {"name": "view_count", "type": "INT", "nullable": true, "key": "INDEX", "default": "0"},
    {"name": "published_at", "type": "DATETIME", "nullable": true, "key": "INDEX"},
    {"name": "created_at", "type": "TIMESTAMP", "nullable": true, "default": "CURRENT_TIMESTAMP"},
    {"name": "updated_at", "type": "TIMESTAMP", "nullable": true, "default": "CURRENT_TIMESTAMP", "extra": "ON UPDATE CURRENT_TIMESTAMP"}
  ],
  "foreign_keys": [
    {"name": "fk_posts_author_id", "columns": ["author_id"], "ref_table": "users", "ref_columns": ["id"], "on_delete": "CASCADE", "on_update": "CASCADE"}
  ],
  "row_count": {"basic": 7, "edge_cases": 7, "total": 14}
}
EOF

cat > "$METADATA_DIR_MYSQL/tags.json" << 'EOF'
{
  "table_name": "tags",
  "columns": [
    {"name": "id", "type": "INT", "nullable": false, "key": "PRIMARY KEY", "extra": "AUTO_INCREMENT"},
    {"name": "name", "type": "VARCHAR(50)", "nullable": false, "key": "UNIQUE"},
    {"name": "slug", "type": "VARCHAR(50)", "nullable": true, "key": "UNIQUE"},
    {"name": "description", "type": "TEXT", "nullable": true, "key": null},
    {"name": "color", "type": "VARCHAR(7)", "nullable": true, "key": null},
    {"name": "created_at", "type": "TIMESTAMP", "nullable": true, "default": "CURRENT_TIMESTAMP"}
  ],
  "row_count": {"basic": 10, "edge_cases": 10, "total": 20}
}
EOF

cat > "$METADATA_DIR_MYSQL/post_tags.json" << 'EOF'
{
  "table_name": "post_tags",
  "columns": [
    {"name": "post_id", "type": "INT", "nullable": false, "key": "PRIMARY KEY"},
    {"name": "tag_id", "type": "INT", "nullable": false, "key": "PRIMARY KEY"},
    {"name": "tagged_at", "type": "TIMESTAMP", "nullable": true, "default": "CURRENT_TIMESTAMP"}
  ],
  "foreign_keys": [
    {"name": "fk_post_tags_post_id", "columns": ["post_id"], "ref_table": "posts", "ref_columns": ["id"], "on_delete": "CASCADE", "on_update": "CASCADE"},
    {"name": "fk_post_tags_tag_id", "columns": ["tag_id"], "ref_table": "tags", "ref_columns": ["id"], "on_delete": "CASCADE", "on_update": "CASCADE"}
  ],
  "row_count": {"basic": 25, "edge_cases": 5, "total": 30}
}
EOF

cat > "$METADATA_DIR_MYSQL/logs.json" << 'EOF'
{
  "table_name": "logs",
  "columns": [
    {"name": "id", "type": "BIGINT", "nullable": false, "key": "PRIMARY KEY", "extra": "AUTO_INCREMENT"},
    {"name": "level", "type": "ENUM", "nullable": false, "key": "INDEX", "values": ["debug", "info", "warning", "error", "critical"]},
    {"name": "message", "type": "TEXT", "nullable": false, "key": null},
    {"name": "context", "type": "JSON", "nullable": true, "key": null},
    {"name": "source", "type": "VARCHAR(100)", "nullable": true, "key": "INDEX"},
    {"name": "created_at", "type": "TIMESTAMP", "nullable": false, "key": "INDEX", "default": "CURRENT_TIMESTAMP"}
  ],
  "row_count": {"basic": 30, "edge_cases": 10, "total": 40}
}
EOF

# Create PostgreSQL metadata files (similar structure with PG-specific types)
cat > "$METADATA_DIR_PG/users.json" << 'EOF'
{
  "table_name": "users",
  "columns": [
    {"name": "id", "type": "SERIAL", "nullable": false, "key": "PRIMARY KEY"},
    {"name": "username", "type": "VARCHAR(50)", "nullable": false, "key": "UNIQUE"},
    {"name": "email", "type": "VARCHAR(100)", "nullable": false, "key": "UNIQUE"},
    {"name": "full_name", "type": "VARCHAR(100)", "nullable": true, "key": null},
    {"name": "age", "type": "INTEGER", "nullable": true, "key": null},
    {"name": "balance", "type": "NUMERIC(10, 2)", "nullable": true, "key": null, "default": "0.00"},
    {"name": "is_active", "type": "BOOLEAN", "nullable": true, "key": "INDEX", "default": "TRUE"},
    {"name": "status", "type": "user_status", "nullable": true, "key": "INDEX", "default": "'active'", "enum_type": true, "values": ["active", "inactive", "suspended"]},
    {"name": "created_at", "type": "TIMESTAMP WITH TIME ZONE", "nullable": true, "key": "INDEX", "default": "CURRENT_TIMESTAMP"},
    {"name": "updated_at", "type": "TIMESTAMP WITH TIME ZONE", "nullable": true, "key": null, "default": "CURRENT_TIMESTAMP"},
    {"name": "last_login", "type": "TIMESTAMP WITH TIME ZONE", "nullable": true, "key": null},
    {"name": "bio", "type": "TEXT", "nullable": true, "key": null},
    {"name": "profile_image", "type": "VARCHAR(255)", "nullable": true, "key": null},
    {"name": "phone", "type": "VARCHAR(20)", "nullable": true, "key": null},
    {"name": "tags", "type": "TEXT[]", "nullable": true, "key": null, "array_type": true},
    {"name": "preferences", "type": "JSONB", "nullable": true, "key": null, "jsonb_type": true}
  ],
  "row_count": {"basic": 20, "edge_cases": 10, "total": 30}
}
EOF

echo "Metadata JSON files generated successfully!"
echo "MySQL metadata: $METADATA_DIR_MYSQL"
echo "PostgreSQL metadata: $METADATA_DIR_PG"
