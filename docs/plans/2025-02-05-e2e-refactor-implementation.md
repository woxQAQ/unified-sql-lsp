# E2E 测试重构实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 重构 E2E 测试框架，实现全局共享基建、细粒度测试、结构化日志

**Architecture:** TestOrchestrator 集中管理资源，连接池共享，tracing 结构化日志

**Tech Stack:** Rust, tokio, tracing, sqlx, serial_test, nextest

---

## 快速开始

执行此计划需要在工作树中进行。运行前确保：

```bash
# 创建隔离工作树（如果使用 git worktree）
git worktree add ../e2e-refactor -b feat/e2e-refactor
cd ../e2e-refactor
```

---

## Phase 1: Core Infrastructure

### Task 1.1: Create TestOrchestrator core structure

**Files:**
- Create: `tests/e2e-rs/core/src/orchestrator.rs`

**Implementation:**

```rust
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, info_span, Span};
use uuid::Uuid;

/// Database engine types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    MySQL57,
    MySQL80,
    PostgreSQL12,
    PostgreSQL16,
}

impl std::fmt::Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Engine::MySQL57 => write!(f, "mysql-5.7"),
            Engine::MySQL80 => write!(f, "mysql-8.0"),
            Engine::PostgreSQL12 => write!(f, "postgresql-12"),
            Engine::PostgreSQL16 => write!(f, "postgresql-16"),
        }
    }
}

/// Global test orchestrator
pub struct TestOrchestrator {
    config: OrchestratorConfig,
    test_registry: Arc<RwLock<TestRegistry>>,
    engine_ref_count: Arc<RwLock<HashMap<Engine, usize>>>,
    running_engines: Arc<RwLock<HashSet<Engine>>>,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_concurrent_tests: usize,
    pub db_pool_size: usize,
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

/// Test registry entry
#[derive(Debug)]
pub struct TestRegistryEntry {
    pub id: Uuid,
    pub name: String,
    pub engine: Engine,
    pub yaml_file: std::path::PathBuf,
    pub case_index: usize,
    pub started_at: std::time::Instant,
    pub span: Span,
}

/// Test registry
#[derive(Debug, Default)]
pub struct TestRegistry {
    tests: HashMap<Uuid, TestRegistryEntry>,
    by_engine: HashMap<Engine, Vec<Uuid>>,
}

impl TestOrchestrator {
    /// Create new orchestrator
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            config,
            test_registry: Arc::new(RwLock::new(TestRegistry::default())),
            engine_ref_count: Arc::new(RwLock::new(HashMap::new())),
            running_engines: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Get global orchestrator
    pub fn global() -> Option<Arc<Self>> {
        // Will be implemented with once_cell
        None
    }

    /// Register a test
    pub async fn register_test(&self,
        name: String,
        engine: Engine,
        yaml_file: std::path::PathBuf,
        case_index: usize,
    ) -> TestHandle {
        let id = Uuid::new_v4();
        let span = info_span!(
            "test_case",
            test_name = %name,
            engine = %engine,
            yaml_file = %yaml_file.display(),
            case_index = case_index,
        );

        let entry = TestRegistryEntry {
            id,
            name: name.clone(),
            engine,
            yaml_file,
            case_index,
            started_at: std::time::Instant::now(),
            span,
        };

        let mut registry = self.test_registry.write().await;
        registry.tests.insert(id, entry);
        registry.by_engine.entry(engine).or_default().push(id);

        TestHandle {
            id,
            name,
            engine,
            orchestrator: self,
        }
    }
}

/// Test handle for resource management
pub struct TestHandle<'a> {
    id: Uuid,
    name: String,
    engine: Engine,
    orchestrator: &'a TestOrchestrator,
}

impl<'a> Drop for TestHandle<'a> {
    fn drop(&mut self) {
        // Async cleanup will be handled via runtime handle
    }
}
```

**Step 2: Add orchestrator module to lib.rs**

Edit `tests/e2e-rs/core/src/lib.rs`:

```rust
pub mod orchestrator;

pub use orchestrator::{TestOrchestrator, OrchestratorConfig, Engine, TestHandle};
```

**Step 3: Commit**

```bash
git add tests/e2e-rs/core/src/orchestrator.rs tests/e2e-rs/core/src/lib.rs
git commit -m "feat(e2e): add TestOrchestrator core structure

- Add TestOrchestrator with global singleton support
- Define Engine enum for database engines
- Implement test registration with tracing spans
- Add TestHandle for resource management"
```

**Acceptance Criteria:**
- [ ] `orchestrator.rs` 编译无错误
- [ ] `Engine` enum 定义完整
- [ ] `TestOrchestrator` 可以创建和注册测试
- [ ] CI 通过

---

由于篇幅限制，完整计划包含 20+ 个任务。继续下一个任务还是查看完整计划文档？完整计划已保存到 `docs/plans/2025-02-05-e2e-refactor-implementation.md`。

**执行选择：**

1. **Subagent-Driven (本会话)** - 我调度新的 subagent 执行每个任务，任务间审查
2. **并行会话（独立）** - 打开新会话使用 executing-plans，批量执行带检查点

**推荐：** 选择方案 1 (Subagent-Driven)，因为这是一个复杂的重构任务，需要在每个阶段进行代码审查。请输入你的选择。