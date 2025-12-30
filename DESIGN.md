# Unified SQL LSP 架构设计方案

## 一、项目概述

### 1.1 目标

构建一个支持多种 SQL 数据库引擎的统一 Language Server Protocol (LSP) 实现，具备以下特性：

- **多引擎支持**：支持几十种不同的数据库引擎及其版本
- **高性能**：基于 Tree-sitter 增量解析，满足实时编辑的响应速度要求
- **上下文感知**：在正确的位置提供最相关的补全建议
- **Schema 感知**：基于真实数据库 schema 提供智能补全，过滤无关内容
- **生产级**：单实例支持多连接、多引擎并发服务

### 1.2 核心设计理念

```
Tree-sitter（语法事实）→ IR（统一抽象）→ Semantic（语义模型）→ LSP（服务接口）
```

**关键原则**：

1. **Tree-sitter 只负责语法事实**：不做语义判断，只提供准确的 CST
2. **IR 层屏蔽方言差异**：所有方言都转换为统一的中间表示
3. **轻量语义模型**：建立作用域和符号表，不做完整类型检查
4. **可插拔架构**：引擎、Schema 源、LSP 功能均可独立扩展

---

## 二、系统架构

### 2.1 总体分层

```
┌─────────────────────────────────────────────────────────────────┐
│                        LSP Server Layer                         │
│  - Completion (核心)                                            │
│  - Hover / Diagnostics (扩展)                                   │
│  - Multi-connection & Multi-engine management                   │
├─────────────────────────────────────────────────────────────────┤
│                      Semantic / Context Layer                   │
│  - Scope & Namespace (表别名、列解析)                           │
│  - Symbol Resolution (列归属、歧义检测)                         │
│  - Context Awareness (补全触发点判断)                           │
├─────────────────────────────────────────────────────────────────┤
│                  Dialect Adaptation Layer                       │
│  - MySQL (5.7, 8.0+) / PostgreSQL / TiDB / ...                  │
├─────────────────────────────────────────────────────────────────┤
│                       SQL IR / AST Layer                        │
│  - Unified Query / Expr / Statement types                      │
├─────────────────────────────────────────────────────────────────┤
│                    Tree-sitter Grammar Layer                    │
│  - Incremental CST parsing                                      │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 模块依赖关系

```
lsp/ ──────┐
           ├─── semantic/
semantic/ ────┤
           └── ir/
lowering/ ──────
  └── grammar/
```

---

## 三、核心模块设计

### 3.1 Grammar Layer

**职责**：封装 Tree-sitter 解析器，提供 CST 查询接口，管理方言特定 Grammar

**设计方案**：自建通用 Grammar + 多方言扩展

参考 DerekStride/tree-sitter-sql 的通用 SQL 设计，构建可配置的多方言 Grammar。

**目录结构**：
```
tree-sitter-unified-sql/
├── grammar.js              # 主 Grammar（方言无关）
├── dialect/                # 方言扩展
│   ├── base.js            # 基础 SQL
│   ├── mysql.js           # MySQL
│   ├── postgresql.js      # PostgreSQL
│   ├── tidb.js            # TiDB (继承 MySQL)
│   └── mariadb.js         # MariaDB (继承 MySQL)
└── scanner.c              # 词法分析器
```

**核心设计原则**：

1. **配置化方言支持**：编译时选择目标方言（`DIALECT=mysql tree-sitter generate`）
2. **方言继承机制**：TiDB 继承 MySQL，添加特定扩展
3. **统一节点命名**：所有方言使用相同的节点类型（`select_statement`, `from_clause`）

**优势**：
- ✅ 统一节点类型，简化 Lowering 层
- ✅ 单一代码库，易于维护
- ✅ 编译时优化，零运行时开销
- ✅ 完全控制，不受上游影响

**参考资源**：
- DerekStride/tree-sitter-sql（主要参考，180+ stars）
- m-novikov/tree-sitter-sql（PostgreSQL 专用，110 stars）

### 3.2 Lowering Layer

**职责**：将 CST 转换为统一 IR，吸收方言语法差异

**错误处理策略**：

采用降级策略，确保 LSP 不崩溃：

- `Success`: 成功转换
- `Partial`: 部分成功（某些子句无法转换）
- `Failed`: 完全失败，降级到 `SyntaxBased` 或 `KeywordsOnly` 补全

**版本兼容**：使用 `semver` crate 进行健壮的版本比较

### 3.3 IR Layer

**现有类型**（已有基础）：
- `Stmt`, `Query`, `SelectStmt`
- `Expr`, `ObjectName`, `TableRef`, `Join`

**核心 SQL 子集定义**：

**必须支持**：SELECT/INSERT/UPDATE/DELETE, WHERE/ORDER BY/LIMIT/GROUP BY, JOIN, 基础表达式

**可选支持**（版本相关）：CTE, 窗口函数, LATERAL 连接, FULL OUTER JOIN

**明确不支持**（Phase 1-5）：DDL, 事务控制, 过程化 SQL, 权限管理

### 3.4 Semantic Layer

**职责**：构建作用域和符号表，解析列引用和表别名，提供补全触发点判断

**核心数据结构**：

- `Scope`: 包含当前可见的表和列
- `TableSymbol`: 表名、实际表名、列列表
- `ColumnSymbol`: 列名、所属表、数据类型

**补全触发点判断**：

基于光标位置的语法节点判断上下文：
- `SelectProjection`: SELECT 子句中的列
- `FromTable`: FROM 子句中的表
- `QualifiedColumn`: table. 后的列
- `JoinCondition`: JOIN ON 条件中的列

### 3.5 Catalog Layer

**职责**：提供数据库 Schema 抽象，支持多种来源

**Catalog trait**：
```rust
async fn list_tables(&self) -> Result<Vec<TableMetadata>>;
async fn get_columns(&self, table: &str) -> Result<Vec<ColumnMetadata>>;
async fn list_functions(&self) -> Result<Vec<FunctionMetadata>>;
```

**实现**：
- `LiveCatalog`: 动态连接（默认 10 连接，5s 超时，健康检查）
- `StaticCatalog`: 文件定义（YAML/JSON）
- `CachedCatalog`: LRU 缓存 + TTL（默认 5 分钟）

**连接池配置**：
- 最大连接数：10（可配置）
- 查询超时：5 秒
- 连接超时：3 秒
- 健康检查：后台定期检查，失败自动重连

### 3.6 LSP Layer

**核心功能**：Completion 实现

**Completion 流程**：

```
cursor position → syntax context (tree-sitter) → semantic context (IR + scope) → catalog → candidates
```

**文档同步与增量解析**：
- 使用 Ropey 进行文本操作
- Tree-sitter 内置增量解析
- 粗粒度缓存失效（Phase 1-3）

**多连接管理**：
- 单实例支持多文档
- 每个文档独立引擎配置
- Catalog 按连接字符串管理，支持复用

---

## 四、多引擎支持策略

### 4.1 引擎抽象

**Dialect 枚举**：
```rust
pub enum Dialect {
    MySQL { version: (u8, u8) },
    PostgreSQL { version: (u8, u8) },
    TiDB { version: semver::Version },
    // ...
}
```

### 4.2 特性支持查询

使用 `FeatureSupport` trait 判断方言支持的特性：

- Window Functions, CTEs, LATERAL Joins, JSON Functions

### 4.3 方言复用策略

- **完全兼容**：直接复用（TiDB → MySQL）
- **部分兼容**：继承 Grammar，自定义 Lowering
- **独立方言**：全新实现（Oracle, MSSQL）

---

## 五、性能优化

### 5.1 增量解析

Tree-sitter 提供内置增量解析，文本更改时只需重新解析受影响部分。

### 5.2 缓存策略

**三级缓存**：
- Tree 缓存
- IR 缓存（使用 ArcSwap 无锁更新）
- Semantic 缓存

**缓存失效**（分阶段）：

**Phase 1-3**：粗粒度失效（推荐初始实现）
- 任何文本更改 → 失效整个文档
- 实现简单，安全可靠
- SQL 语句通常较短，完整重解析开销可接受

**Phase 4-5**：细粒度失效（性能优化）
- Tree 更新 → IR 缓存失效（仅受影响语句）
- IR 更新 → Semantic 缓存失效（仅受影响作用域）

**注意**：SQL 作用域复杂（CTE、子查询），细粒度失效需要完整语义理解，建议先粗后细，通过性能测试决定是否优化。

### 5.3 并发处理

- 后台异步语义分析（不阻塞 LSP 主线程）
- DashMap 并发安全
- ArcSwap 无锁更新

### 5.4 Catalog 优化

- LRU 缓存 + TTL（默认 5 分钟）
- 批量查询（`get_columns_batch`）
- 预加载（用户输入 "FROM " 时预加载表列表）

---

## 六、Schema 过滤策略

### 6.1 SchemaFilter

```rust
pub struct SchemaFilter {
    pub allowed_schemas: Option<Vec<String>>,
    pub allowed_tables: Option<Vec<String>>,   // glob 模式
    pub excluded_tables: Option<Vec<String>>,  // glob 模式
}
```

支持 glob 模式：`users_*`, `temp.*`

### 6.2 FilteredCatalog

在 Catalog 层实现过滤，确保用户只看到有权限访问的表和列。

---

## 七、错误处理与诊断

### 7.1 诊断类型

- `SyntaxError`: 语法错误
- `UndefinedTable`: 未定义的表
- `UndefinedColumn`: 未定义的列（提供候选）
- `AmbiguousColumn`: 歧义列（列出可能的表）

### 7.2 诊断生成

Semantic Analyzer 在分析时生成诊断信息，通过 LSP 推送给客户端。

---

## 八、实施路线图

### Phase 1: 基础设施（2-3 周）

- [ ] Grammar Layer
  - [ ] Fork DerekStride/tree-sitter-sql
  - [ ] 实现 MySQL/PostgreSQL 方言
  - [ ] 编写单元测试

- [ ] IR Layer（已有基础，小幅扩展）
  - [ ] 添加 Window 子句
  - [ ] 添加 DialectExtensions

### Phase 2: 核心功能（3-4 周）

- [ ] Lowering Layer
  - [ ] 实现 Lowering trait
  - [ ] MySQL/PostgreSQL Lowering
  - [ ] 错误处理与降级策略

- [ ] Semantic Layer
  - [ ] Scope, TableSymbol, ColumnSymbol
  - [ ] SemanticAnalyzer
  - [ ] 补全触发点判断

- [ ] Catalog Layer
  - [ ] Catalog trait
  - [ ] LiveCatalog 实现
  - [ ] CachedCatalog

### Phase 3: LSP 集成（2-3 周）

- [ ] LSP Server
  - [ ] Backend 结构
  - [ ] Completion 处理
  - [ ] 文档同步（did_open, did_change）
  - [ ] 增量解析与缓存

### Phase 4: 多引擎扩展（持续）

- [ ] TiDB, MariaDB, CockroachDB
- [ ] 版本特性支持
- [ ] 引擎配置文件格式

### Phase 5: 优化与扩展（持续）

- [ ] 性能优化
- [ ] Hover, Diagnostics
- [ ] Schema 缓存持久化

**总计**：8-12 周完成核心功能

---

## 九、测试策略

### 9.1 测试矩阵

为确保多方言版本兼容性，建立测试矩阵：

| Dialect | Version | Feature | Expected Result |
|---------|---------|---------|-----------------|
| MySQL | 5.7 | Window Functions | Error/Partial |
| MySQL | 8.0+ | Window Functions | Success |
| PostgreSQL | 12 | CTEs | Success |
| PostgreSQL | 9.3 | LATERAL Joins | Success |
| All | - | Basic SELECT | Success |

### 9.2 测试覆盖

- **单元测试**：Grammar, Lowering, Semantic
- **集成测试**：Completion 流程
- **性能测试**：10k 行 < 100ms
- **错误场景**：未定义表、歧义列、语法错误
- **边缘情况**：空文件、纯注释、嵌套查询

---

## 十、配置示例

### 10.1 引擎配置（YAML）

```yaml
engines:
  mysql-prod:
    dialect: mysql
    version: "8.0"
    connection_string: "mysql://user:pass@prod-db:3306/app"
    schema_filter:
      allowed_schemas: ["app", "reporting"]
      excluded_tables: ["temp_*", "_*"]

  pg-staging:
    dialect: postgres
    version: "14"
    connection_string: "postgresql://user:pass@staging-db:5432/app"
```

### 10.2 配置验证

```bash
unified-sql-lsp --validate-config config/engines.yaml
unified-sql-lsp --test-connection mysql-prod
unified-sql-lsp --show-config
```

**默认值**：
- `max_connections`: 10
- `query_timeout`: 5 秒
- `cache_ttl`: 300 秒

---

## 十一、关键决策记录

### 11.1 为什么选择 Tree-sitter？

**优势**：增量解析、错误恢复、多语言支持、可组合性

**替代方案**：
- ❌ sqlparser-rs：不支持增量解析
- ❌ 手写 Parser：开发成本高

### 11.2 为什么需要 IR 层？

**原因**：
1. 方言隔离：LSP 逻辑不处理方言差异
2. 可测试性：IR 独立测试
3. 可扩展性：新增方言只需实现 Lowering

### 11.3 为什么不支持 Jump Definition？

**原因**：
- SQL 的"定义"概念模糊
- 实现复杂度高（跨文件分析）
- 优先级低（Completion 是核心）

### 11.4 为什么使用 DashMap？

**优势**：并发安全、高性能（分片锁）、API 友好

---

## 十二、风险与挑战

### 12.1 性能风险

**风险**：大文件解析慢，Catalog 查询延迟

**缓解**：增量解析 + 缓存 + 后台异步 + 性能测试

### 12.2 方言兼容性

**风险**：某些方言差异巨大，IR 难以统一

**缓解**：DialectExtensions 保留方言特定信息，文档明确支持范围

### 12.3 扩展性挑战

**风险**：支持 30+ 引擎，开发工作量大

**缓解**：优先主流引擎，社区贡献，工具辅助生成

---

## 十三、后续扩展

### 13.1 高级功能

- Hover, Diagnostics, Signature Help, Code Actions, Format

### 13.2 企业级特性

- Schema Cache 持久化, 多租户隔离, 审计日志, Metrics（Prometheus）

### 13.3 Rust Feature Flags

使用条件编译减少二进制大小：

```toml
[features]
default = ["mysql", "postgresql"]
mysql = []
postgresql = []
tidb = ["mysql"]
full = ["mysql", "postgresql", "tidb", "hover", "diagnostics"]
```

### 13.4 LSP Capability Negotiation

向客户端声明支持的能力（text_document_sync, completion_provider, hover_provider, diagnostic_provider）

---

## 附录

### A. 参考资源

- [Tree-sitter 官方文档](https://tree-sitter.github.io/tree-sitter/)
- [DerekStride/tree-sitter-sql](https://github.com/DerekStride/tree-sitter-sql) - 主要参考
- [m-novikov/tree-sitter-sql](https://github.com/m-novikov/tree-sitter-sql) - PostgreSQL 参考
- [LSP 规范](https://microsoft.github.io/language-server-protocol/)
- [tower-lsp 文档](https://docs.rs/tower-lsp/)

### B. 相关项目

- [jedi-language-server](https://github.com/pappasam/jedi-language-server) - Python LSP（参考架构）
- [rust-analyzer](https://github.com/rust-analyzer/rust-analyzer) - 缓存策略参考
- [gopls](https://github.com/golang/tools/tree/master/gopls) - 模块化设计参考

### C. 方言扩展指南（简要版）

**步骤**：

1. **评估方言特性**：收集语法差异、版本范围、兼容性信息
2. **创建方言扩展**：在 `dialect/` 目录创建新方言文件，继承基础方言
3. **实现 Lowering**：复用基础方言的 Lowering，添加特定扩展处理
4. **编写测试**：单元测试、集成测试、性能测试
5. **更新文档与配置**：版本支持、特定功能、限制说明

**示例**：TiDB 继承 MySQL，添加 TIDB_SNAPSHOT 等特定关键字

### D. 性能基准

所有方言必须达到：

| 指标 | 目标 |
|------|------|
| 10k 行解析 | < 100ms |
| Completion 延迟 | < 50ms (p95) |
| 内存占用 | < 50MB |
| 缓存命中率 | > 80% |

---

**文档版本**: v2.0 (精简版)
**最后更新**: 2025-12-31
**维护者**: unified-sql-lsp team
