# E2E 测试重构实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 重构 E2E 测试框架，实现全局共享基建、每个 YAML TestCase 生成独立测试函数、tracing 结构化日志、动态引擎启动

**Architecture:** 引入 TestOrchestrator 集中管理所有数据库和 LSP 服务器，使用连接池共享资源，tracing 提供结构化日志，宏生成细粒度测试函数

**Tech Stack:** Rust, tokio, tracing, tracing-subscriber, serial_test, sqlx, tower-lsp, nextest

---

## Phase 1: Core Infrastructure

### Task 1.1: 创建 TestOrchestrator 基础结构

**Files:**
- Create: `tests/e2e-rs/core/src/orchestrator.rs`

**Step 1: Write TestOrchestrator 结构定义**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, info_span, Span};
use uuid::Uuid;

use crate::db_pool::DatabasePoolManager;
use crate::lsp_pool::LspClientPool;

/// 全局测试编排器
pub struct TestOrchestrator {
    /// 配置
    config: OrchestratorConfig,

    /// 数据库连接池管理器
    db_manager: Arc<RwLock<Option<DatabasePoolManager>>>,

    /// LSP 客户端池
    lsp_pool: Arc<RwLock<Option<LspClientPool>>>,

    /// 测试注册表
    test_registry: Arc<RwLock<TestRegistry>>,

    /// 引擎使用计数器
    engine_ref_count: Arc<RwLock<HashMap<Engine, usize>>>,

    /// 已启动的引擎集合
    running_engines: Arc<RwLock<HashSet<Engine>>>,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// 最大并发测试数
    pub max_concurrent_tests: usize,
    /// 数据库连接池大小
    pub db_pool_size: usize,
    /// LSP 连接池大小
    pub lsp_pool_size: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tests: 10,
            db_pool_size: 10,
            lsp_pool_size: 5,
        }
    }
}
```

**Step 2: 实现全局单例模式**

```rust
use once_cell::sync::Lazy;
use std::sync::Mutex;

static ORCHESTRATOR: Lazy<Mutex<Option<Arc<TestOrchestrator>>>> =
    Lazy::new(|| Mutex::new(None));

impl TestOrchestrator {
    /// 初始化全局编排器
    pub fn initialize(config: OrchestratorConfig) -> Result<Arc<Self>> {
        let mut guard = ORCHESTRATOR.lock().map_err(|_| {
            anyhow::anyhow!("Failed to lock orchestrator mutex")
        })?;

        if guard.is_some() {
            return Err(anyhow::anyhow!("Orchestrator already initialized"));
        }

        let orchestrator = Arc::new(TestOrchestrator {
            config,
            db_manager: Arc::new(RwLock::new(None)),
            lsp_pool: Arc::new(RwLock::new(None)),
            test_registry: Arc::new(RwLock::new(TestRegistry::default())),
            engine_ref_count: Arc::new(RwLock::new(HashMap::new())),
            running_engines: Arc::new(RwLock::new(HashSet::new())),
        });

        *guard = Some(Arc::clone(&orchestrator));
        Ok(orchestrator)
    }

    /// 获取全局编排器实例
    pub fn global() -> Result<Arc<Self>> {
        let guard = ORCHESTRATOR.lock().map_err(|_| {
            anyhow::anyhow!("Failed to lock orchestrator mutex")
        })?;

        guard.as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Orchestrator not initialized. Call initialize() first."))
    }
}
```

**Step 3: Commit**

```bash
git add tests/e2e-rs/core/src/orchestrator.rs
git commit -m "feat(e2e): add TestOrchestrator core structure with global singleton"
```

**Acceptance Criteria:**
- `TestOrchestrator` 结构定义完整
- 全局单例模式正常工作
- 可以通过 `TestOrchestrator::global()` 访问实例

---

### Task 1.2: 实现数据库连接池管理器

**Files:**
- Create: `tests/e2e-rs/core/src/db_pool.rs`

**Step 1: 实现 DatabasePoolManager**

```rust
use std::collections::HashMap;
use sqlx::{Any, Pool, pool::PoolConnection};
use tracing::{info, debug};

use crate::orchestrator::Engine;

/// 数据库配置
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub connection_string: String,
    pub pool_size: usize,
}

/// 数据库连接池管理器
pub struct DatabasePoolManager {
    pools: HashMap<Engine, Pool<Any>>,
}

impl DatabasePoolManager {
    /// 创建新的管理器
    pub async fn new() -> Result<Self> {
        Ok(Self {
            pools: HashMap::new(),
        })
    }

    /// 为指定引擎创建连接池
    pub async fn create_pool(
        &mut self,
        engine: Engine,
        config: &DatabaseConfig,
    ) -> Result<()> {
        if self.pools.contains_key(&engine) {
            return Ok(());
        }

        info!(%engine, "Creating database connection pool");

        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(config.pool_size as u32)
            .min_connections(2)
            .connect_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(300))
            .connect(&config.connection_string)
            .await
            .with_context(|| format!("Failed to create pool for {}", engine))?;

        // 测试连接
        let _ = pool.acquire().await?;

        self.pools.insert(engine, pool);
        info!(%engine, "Database connection pool ready");

        Ok(())
    }

    /// 获取指定引擎的连接
    pub async fn acquire(&self, engine: Engine) -> Result<PoolConnection<Any>> {
        let pool = self.pools.get(&engine)
            .ok_or_else(|| anyhow::anyhow!("No pool for engine: {}", engine))?;

        let conn = pool.acquire().await
            .with_context(|| format!("Failed to acquire connection for {}", engine))?;

        Ok(conn)
    }

    /// Truncate 所有表（清理数据但保留结构）
    pub async fn truncate_tables(&self, engine: Engine) -> Result<()> {
        let mut conn = self.acquire(engine).await?;

        let tables: Vec<String> = match engine {
            Engine::MySQL57 | Engine::MySQL80 => {
                sqlx::query_scalar(
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE'"
                )
                .fetch_all(&mut *conn)
                .await?
            }
            Engine::PostgreSQL12 | Engine::PostgreSQL16 => {
                sqlx::query_scalar(
                    "SELECT tablename FROM pg_tables
                     WHERE schemaname = 'public'"
                )
                .fetch_all(&mut *conn)
                .await?
            }
        };

        for table in tables {
            let sql = format!("TRUNCATE TABLE `{}`", table);
            sqlx::query(&sql).execute(&mut *conn).await?;
        }

        Ok(())
    }

    /// 关闭指定引擎的连接池
    pub async fn close_pool(&mut self, engine: Engine) -> Result<()> {
        if let Some(pool) = self.pools.remove(&engine) {
            pool.close().await;
            info!(%engine, "Database connection pool closed");
        }
        Ok(())
    }
}
```

**Step 2: Commit**

```bash
git add tests/e2e-rs/core/src/db_pool.rs
git commit -m "feat(e2e): add DatabasePoolManager for shared database connections"
```

**Acceptance Criteria:**
- `DatabasePoolManager` 支持多引擎连接池
- 可以实现 `truncate_tables` 清理数据
- 连接可以正确获取和释放

---

由于篇幅限制，我将提供一个完整的实现计划文档总结。完整的详细任务列表需要更多步骤。你希望我先提供一个概要性的实现路线图，还是继续详细的 Phase 2-5 设计？