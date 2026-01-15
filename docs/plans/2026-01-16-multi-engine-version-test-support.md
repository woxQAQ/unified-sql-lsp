# Multi-Engine/Version Test Support Design

**Date:** 2026-01-16
**Status:** Draft
**Author:** Claude (Brainstorming Session)

## Problem Statement

Current E2E tests only support MySQL 5.7. To validate the LSP server against different SQL dialects and versions (MySQL 5.7, 8.0, PostgreSQL 12, 16, etc.), the test framework needs refactoring to support multiple database engines simultaneously.

## Proposed Solution

Reorganize tests by dialect/version in separate directories, with each test suite targeting a specific database engine and version.

## Directory Structure

```
tests/e2e-rs/
├── tests/
│   ├── mysql-5.7/
│   │   ├── completion/
│   │   │   ├── select_clause.yaml
│   │   │   ├── from_clause.yaml
│   │   │   └── ...
│   │   └── diagnostics/
│   │       └── basic_diagnostics.yaml
│   ├── mysql-8.0/
│   │   └── completion/
│   │       └── window_functions.yaml  # MySQL 8.0 specific features
│   ├── postgresql-12/
│   │   ├── completion/
│   │   │   └── select_clause.yaml
│   │   └── diagnostics/
│   └── postgresql-16/
│       └── completion/
│           └── advanced_features.yaml
├── fixtures/
│   ├── schemas/
│   │   ├── mysql-5.7/
│   │   │   └── 01_create_tables.sql
│   │   ├── mysql-8.0/
│   │   ├── postgresql-12/
│   │   └── postgresql-16/
│   └── data/
│       ├── mysql-5.7/
│       ├── mysql-8.0/
│       ├── postgresql-12/
│       └── postgresql-16/
```

## Docker Compose Configuration

Update `docker-compose.yml` to run all databases simultaneously on different ports:

```yaml
services:
  mysql-5.7:
    image: mysql:5.7
    container_name: unified-sql-lsp-mysql-57
    ports:
      - "3307:3306"
    environment:
      MYSQL_ROOT_PASSWORD: root_password
      MYSQL_DATABASE: test_db
      MYSQL_USER: test_user
      MYSQL_PASSWORD: test_password
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "localhost", "-u", "root", "-proot_password"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - sql_lsp_network

  mysql-8.0:
    image: mysql:8.0
    container_name: unified-sql-lsp-mysql-80
    ports:
      - "3308:3306"
    environment:
      MYSQL_ROOT_PASSWORD: root_password
      MYSQL_DATABASE: test_db
      MYSQL_USER: test_user
      MYSQL_PASSWORD: test_password
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "localhost", "-u", "root", "-proot_password"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - sql_lsp_network

  postgresql-12:
    image: postgres:12
    container_name: unified-sql-lsp-postgresql-12
    ports:
      - "5433:5432"
    environment:
      POSTGRES_USER: test_user
      POSTGRES_PASSWORD: test_password
      POSTGRES_DB: test_db
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U test_user"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - sql_lsp_network

  postgresql-16:
    image: postgres:16
    container_name: unified-sql-lsp-postgresql-16
    ports:
      - "5434:5432"
    environment:
      POSTGRES_USER: test_user
      POSTGRES_PASSWORD: test_password
      POSTGRES_DB: test_db
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U test_user"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - sql_lsp_network

volumes:
  mysql_57_data:
  mysql_80_data:
  postgres_12_data:
  postgres_16_data:

networks:
  sql_lsp_network:
    driver: bridge
```

## Implementation Changes

### 1. Database Adapter Factory (`src/db/adapter.rs`)

Add path-based adapter selection:

```rust
impl DatabaseAdapter for Arc<dyn DatabaseAdapter> {
    fn connection_string(&self) -> &str {
        // existing method
    }
}

pub fn adapter_from_test_path(test_path: &Path) -> Result<Arc<dyn DatabaseAdapter>> {
    let path_str = test_path.to_string_lossy();

    if path_str.contains("/mysql-5.7/") || path_str.contains("\\mysql-5.7\\") {
        Ok(Arc::new(MySQLAdapter::new(
            "mysql://test_user:test_password@127.0.0.1:3307/test_db"
        )) as Arc<dyn DatabaseAdapter>)
    } else if path_str.contains("/mysql-8.0/") {
        Ok(Arc::new(MySQLAdapter::new(
            "mysql://test_user:test_password@127.0.0.1:3308/test_db"
        )) as Arc<dyn DatabaseAdapter>)
    } else if path_str.contains("/postgresql-12/") {
        Ok(Arc::new(PostgreSQLAdapter::new(
            "postgresql://test_user:test_password@127.0.0.1:5433/test_db"
        )) as Arc<dyn DatabaseAdapter>)
    } else if path_str.contains("/postgresql-16/") {
        Ok(Arc::new(PostgreSQLAdapter::new(
            "postgresql://test_user:test_password@127.0.0.1:5434/test_db"
        )) as Arc<dyn DatabaseAdapter>)
    } else {
        // Fallback to default (MySQL 5.7 for backward compatibility)
        Ok(Arc::new(MySQLAdapter::from_default_config()) as Arc<dyn DatabaseAdapter>)
    }
}
```

### 2. Remove Global Database Adapter (`src/lib.rs`)

**Before:**
```rust
static DB_ADAPTER: LazyLock<Arc<RwLock<Option<Arc<dyn DatabaseAdapter>>>>> = ...;
```

**After:**
```rust
// No global adapter - each test determines its own adapter from path
```

Update `init_database()` to only start Docker services (all of them):

```rust
pub async fn init_database() -> Result<()> {
    // Check if already initialized
    {
        let guard = DOCKER_COMPOSE.read().await;
        if guard.is_some() {
            info!("Docker Compose already initialized");
            return Ok(());
        }
    }

    // Start ALL Docker Compose services
    info!("Starting Docker Compose services...");
    let mut docker_compose = DockerCompose::from_default_config()?;
    docker_compose.start().await?;

    {
        let mut compose_guard = DOCKER_COMPOSE.write().await;
        *compose_guard = Some(docker_compose);
    }

    info!("All Docker services started successfully");
    Ok(())
}
```

### 3. Update Test Execution (`src/lib.rs`)

Modify `run_test()` to use path-based adapter:

```rust
pub async fn run_test(
    suite: &TestSuite,
    test: &yaml_parser::TestCase,
    suite_path: &std::path::Path,
) -> Result<()> {
    info!("=== Running test: {} ===", test.name);

    // 1. Determine adapter from test path
    let adapter = db::adapter_from_test_path(suite_path)?;

    // 2. Setup database (load schema/data if needed)
    for schema_path in &suite.database.schemas {
        let full_path = suite_dir.join(schema_path);
        adapter.load_schema(&full_path).await?;
    }

    for data_path in &suite.database.data {
        let full_path = suite_dir.join(data_path);
        adapter.load_data(&full_path).await?;
    }

    // 3. Spawn LSP server
    let mut lsp_runner = runner::LspRunner::from_crate()?;
    lsp_runner.spawn().await?;

    // 4. Establish LSP connection
    let stdin = lsp_runner.stdin()?;
    let stdout = lsp_runner.stdout()?;
    let mut conn = LspConnection::new(stdin, stdout);

    // 5. Initialize server
    conn.initialize().await?;

    // 6. Set engine configuration
    let connection_string = adapter.connection_string();
    let dialect = suite.database.dialect.clone();
    conn.did_change_configuration(&dialect, connection_string).await?;

    // 7-10. Rest of test flow unchanged...
}
```

### 4. Create PostgreSQL Adapter (`src/db/adapter.rs`)

```rust
pub struct PostgreSQLAdapter {
    connection_string: String,
}

impl PostgreSQLAdapter {
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
        }
    }

    pub fn from_default_config() -> Self {
        Self::new("postgresql://test_user:test_password@127.0.0.1:5433/test_db")
    }
}

#[async_trait]
impl DatabaseAdapter for PostgreSQLAdapter {
    async fn load_schema(&self, path: &Path) -> Result<()> {
        // Execute SQL file using psql client
        let output = Command::new("psql")
            .args([&self.connection_string, "-f", path.to_str().unwrap()])
            .output()
            .await?;

        if !output.status.success() {
            bail!("Failed to load PostgreSQL schema: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    async fn load_data(&self, path: &Path) -> Result<()> {
        // Similar to load_schema
        self.load_schema(path).await
    }

    async fn cleanup(&self) -> Result<()> {
        // Drop and recreate test database
        Ok(())
    }

    fn connection_string(&self) -> &str {
        &self.connection_string
    }
}
```

## Migration Strategy

### Phase 1: Prepare Infrastructure
1. Update `docker-compose.yml` to include all database services
2. Create `PostgreSQLAdapter` in `src/db/adapter.rs`
3. Add `adapter_from_test_path()` function
4. Update `init_database()` to start all services

### Phase 2: Restructure Test Directories
1. Create `tests/mysql-5.7/` directory
2. Move `tests/completion/` → `tests/mysql-5.7/completion/`
3. Move `tests/diagnostics/` → `tests/mysql-5.7/diagnostics/`
4. Create empty directories: `mysql-8.0/`, `postgresql-12/`, `postgresql-16/`

### Phase 3: Update Test File Paths
1. Update schema/data paths in YAMLs from `../../fixtures/...` to `../../../fixtures/...`
2. Move fixture files to versioned directories:
   - `fixtures/schema/mysql/` → `fixtures/schemas/mysql-5.7/`
   - `fixtures/data/mysql/` → `fixtures/data/mysql-5.7/`
3. Create schema/data files for PostgreSQL versions

### Phase 4: Update Test Runner
1. Remove global `DB_ADAPTER` state
2. Modify `run_test()` to use path-based adapter selection
3. Run tests: `make test-e2e` and ensure all existing tests pass

### Phase 5: Add New Tests
1. Add MySQL 8.0 specific tests (window functions, CTE improvements)
2. Add PostgreSQL 12/16 specific tests (advanced features, different syntax)
3. Add cross-dialect compatibility tests

## Backward Compatibility

Tests outside versioned directories (e.g., directly in `tests/`) will fallback to MySQL 5.7 default behavior via the `adapter_from_test_path()` fallback logic.

## Benefits

1. **Clear Test Organization:** Tests grouped by database engine and version
2. **Easy to Extend:** Adding new database versions is straightforward
3. **Parallel Ready:** Tests can run in parallel across different databases
4. **Explicit Dependencies:** Each test explicitly declares its target database
5. **Backward Compatible:** Existing tests continue to work

## Open Questions

None - approach is straightforward.

## Next Steps

After approval, proceed with implementation following the migration strategy phases.
