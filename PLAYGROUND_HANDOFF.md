# Playground LSP 实现交接文档

**创建时间**: 2025-01-19
**状态**: 已完成基础实现，存在已知限制
**优先级**: P0 - 严重功能限制

## 概述

当前 playground 实现使用 JavaScript mock LSP 服务器，提供基本的代码补全功能。但存在严重的功能限制，导致实际可用性较低。

---

## 问题 1: 只有 SELECT 语句有提示

### 问题描述
**严重程度**: 🔴 高

当前实现中，只有当 SQL 文本包含 `SELECT` 关键字时才会显示补全建议。其他 SQL 语句类型（INSERT, UPDATE, DELETE, CREATE TABLE, ALTER TABLE 等）完全没有补全支持。

### 复现步骤
1. 输入 `INSERT INTO` → 无补全建议
2. 输入 `UPDATE` → 无补全建议
3. 输入 `CREATE TABLE` → 无补全建议
4. 输入 `SELECT` → 显示补全建议 ✓

### 根本原因

**文件**: `playground/src/lib/wasm-interface.ts`

```typescript
// 当前实现 - 第 83-92 行
if (text.includes("SELECT") && !text.includes("FROM")) {
  items.push({
    label: "FROM",
    kind: 14,
    detail: "Keyword",
    documentation: "Specifies the table to query from",
    insertText: "\nFROM ",
  })
}
```

当前逻辑：
- 使用简单的字符串匹配 `text.includes("SELECT")`
- 没有真正的 SQL 解析
- 没有上下文状态机

### 影响范围
- ❌ 无法补全 INSERT 语句
- ❌ 无法补全 UPDATE 语句
- ❌ 无法补全 DELETE 语句
- ❌ 无法补全 DDL 语句 (CREATE, ALTER, DROP)
- ❌ 无法补全事务语句 (BEGIN, COMMIT, ROLLBACK)

### 改进方案

#### 方案 A: 基于状态的简单解析器 (快速实现)

```typescript
enum SqlContext {
  SelectStatement,
  InsertStatement,
  UpdateStatement,
  DeleteStatement,
  CreateStatement,
  // ...
}

function detectSqlContext(text: string): SqlContext {
  const trimmed = text.trim().toUpperCase()

  if (trimmed.startsWith("SELECT")) return SqlContext.SelectStatement
  if (trimmed.startsWith("INSERT")) return SqlContext.InsertStatement
  if (trimmed.startsWith("UPDATE")) return SqlContext.UpdateStatement
  if (trimmed.startsWith("DELETE")) return SqlContext.DeleteStatement
  if (trimmed.startsWith("CREATE")) return SqlContext.CreateStatement

  return SqlContext.Unknown
}

// 根据上下文返回不同的关键词
function getKeywordsForContext(context: SqlContext): CompletionItem[] {
  switch (context) {
    case SqlContext.SelectStatement:
      return [SELECT_KEYWORD, FROM_KEYWORD, WHERE_KEYWORD, ...]

    case SqlContext.InsertStatement:
      return [INTO_KEYWORD, VALUES_KEYWORD, ...]

    case SqlContext.UpdateStatement:
      return [SET_KEYWORD, WHERE_KEYWORD, ...]

    // ...
  }
}
```

**优点**:
- 实现快速 (1-2 天)
- 不需要完整解析器
- 覆盖常见 SQL 语句

**缺点**:
- 不够精确
- 无法处理复杂嵌套
- 无法处理子查询

#### 方案 B: 集成 Tree-sitter 解析器 (推荐)

使用项目已有的 `unified-grammar` crate 进行真正的 SQL 解析。

```rust
// 在 crates/lsp-wasm/src/lib.rs 中
use unified_sql_grammar::{Parser, NodeKind};

fn get_completion_at_position(
    text: &str,
    line: usize,
    col: usize
) -> Vec<CompletionItem> {
    let parser = Parser::new();
    let tree = parser.parse(text).unwrap();

    let node = tree.node_at_position(line, col);

    match node.kind() {
        NodeKind::SelectStatement => get_select_completions(),
        NodeKind::InsertStatement => get_insert_completions(),
        NodeKind::UpdateStatement => get_update_completions(),
        NodeKind::Identifier => get_column_completions(&node),
        // ...
    }
}
```

**优点**:
- 精确的上下文理解
- 支持复杂嵌套
- 支持子查询
- 可扩展到方言特定功能

**缺点**:
- 需要编译 Tree-sitter 到 WASM
- 需要更多开发时间 (5-10 天)

---

## 问题 2: 无法识别表别名

### 问题描述
**严重程度**: 🔴 高

当查询中使用表别名时，补全系统无法识别别名，导致：
- 无法补全 `SELECT u.` 后的列名
- 无法识别 `FROM users u` 中的 `u` 是表别名
- 无法提供表名到别名的映射

### 复现步骤

1. 输入 `SELECT u.` → 应该显示 users 表的列，但实际无提示
2. 输入 `SELECT u.n` → 应该补全 `name`，但实际无提示
3. 输入 `FROM users u` → 别名 `u` 应该被记住，但实际被忽略

### 根本原因

**文件**: `playground/src/lib/wasm-interface.ts`

```typescript
// 第 9-94 行 - MockLspServer 类
class MockLspServer {
  constructor(_dialect: string) {}  // ❌ 没有状态存储

  completion(text: string, _line: number, _col: number): string {
    const items = [
      // 硬编码的补全项，没有上下文分析
      { label: "users", kind: 5, detail: "Table" },
      { label: "id", kind: 5, detail: "INT" },
      // ❌ 没有区分哪个表的列
    ]
    return JSON.stringify(items)
  }
}
```

**问题**:
1. 没有解析 FROM 子句中的表别名
2. 没有维护符号表 (symbol table)
3. 没有列到表的映射关系
4. 补全是全局的，不是上下文敏感的

### 影响范围

```sql
-- 场景 1: 别名后的列补全
SELECT u.    -- ❌ 应该提示 id, name, email
FROM users u

-- 场景 2: JOIN 别名
SELECT u.n, o.t    -- ❌ 应该提示 u.name, o.total
FROM users u
JOIN orders o ON u.id = o.user_id

-- 场景 3: 子查询别名
SELECT * FROM (SELECT * FROM users) AS u    -- ❌ u 应该被识别为表别名
WHERE u.id = 1
```

### 改进方案

#### 阶段 1: 基础别名解析 (必需)

```typescript
interface TableInfo {
  name: string
  alias: string | null
  columns: ColumnInfo[]
}

interface SqlScope {
  tables: Map<string, TableInfo>  // alias -> TableInfo
}

function parseFromClause(sql: string): SqlScope {
  const scope: SqlScope = { tables: new Map() }

  // 使用正则解析 FROM 子句 (简化版)
  const fromMatch = sql.match(/FROM\s+(\w+)(?:\s+(?:AS\s+)?(\w+))?/i)
  if (fromMatch) {
    const tableName = fromMatch[1]
    const alias = fromMatch[2] || null

    const tableInfo = {
      name: tableName,
      alias: alias,
      columns: getColumnsForTable(tableName)
    }

    // 用别名或表名作为 key
    const key = alias || tableName
    scope.tables.set(key, tableInfo)
  }

  return scope
}

function completion(text: string, line: number, col: number): CompletionItem[] {
  const scope = parseFromClause(text)

  // 检测光标前的标识符
  const prefix = getTextBeforeCursor(text, line, col)
  const match = prefix.match(/(\w+)\.\s*$/)

  if (match) {
    const alias = match[1]
    const tableInfo = scope.tables.get(alias)

    if (tableInfo) {
      // 返回该表的列
      return tableInfo.columns.map(col => ({
        label: col.name,
        kind: CompletionItemKind.Field,
        detail: col.type,
        insertText: col.name
      }))
    }
  }

  // 返回所有表
  return getAllTables()
}
```

#### 阶段 2: 完整符号解析 (推荐)

参考主仓库中的 `ScopeManager` 和 `AliasResolver` 实现：

**文件**: `crates/semantic/src/scope.rs`
**文件**: `crates/semantic/src/alias.rs`

这些组件已经实现了：
- ✅ 表别名解析 (4 种策略)
- ✅ 列可见性分析
- ✅ 作用域层级管理

**需要移植到 WASM**:

```rust
// 在 crates/lsp-wasm/src/scope.rs 中
use unified_sql_lsp_semantic::{ScopeManager, AliasResolver};

#[wasm_bindgen]
pub struct WasmScopeManager {
    inner: ScopeManager,
}

#[wasm_bindgen]
impl WasmScopeManager {
    pub fn new() -> Self {
        Self {
            inner: ScopeManager::new(),
        }
    }

    pub fn update_scope(&mut self, sql: &str) {
        // 使用 tree-sitter 解析
        // 更新符号表
    }

    pub fn get_columns_at_position(
        &self,
        line: usize,
        col: usize
    ) -> JsValue {
        // 返回当前位置可见的列
    }
}
```

**挑战**:
- `unified-sql-lsp-semantic` 依赖 tokio (不兼容 WASM)
- 需要提取平台无关的核心逻辑
- 需要重构以移除 async 依赖

---

## 问题 3: 退化成字符串匹配，缺乏真正的 LSP 能力

### 问题描述
**严重程度**: 🔴 高

当前实现只是简单的字符串匹配和全局列表，而不是真正的语言服务器协议实现：

1. **无位置感知**: 补全不考虑光标位置和当前上下文
2. **无增量更新**: 每次调用都重新分析整个文本
3. **无错误恢复**: 语法错误时完全失败
4. **无语义分析**: 不理解表结构、列类型、关系等

### 复现步骤

1. 在任意位置输入 `id` → 都会补全，不考虑是否在合理的位置
2. 在注释中输入 `SELECT` → 仍然提示关键词（应该跳过注释）
3. 语法错误的 SQL → 没有智能补全，甚至崩溃

### 根本原因

```typescript
// wasm-interface.ts - 第 125-144 行
export async function initWasm(dialect: string = 'mysql'): Promise<any> {
  initPromise = (async () => {
    // ❌ 直接使用 MockLspServer
    // ❌ 没有解析，没有分析，没有状态
    wasmInstance = new MockLspServer(dialect)
    return wasmInstance
  })()
}
```

**缺失的 LSP 核心能力**:

| 能力 | 当前状态 | 应该有 |
|------|---------|--------|
| AST 解析 | ❌ 无 | ✅ Tree-sitter |
| 符号表 | ❌ 无 | ✅ ScopeManager |
| 别名解析 | ❌ 无 | ✅ AliasResolver |
| 类型推导 | ❌ 无 | ✅ Type inference |
| 错误恢复 | ❌ 无 | ✅ Partial success mode |
| 增量更新 | ❌ 无 | ✅ Document sync |
| 诊断 | 🔶 极简 | ✅ Semantic + Syntax |

### 对比: 真正的 LSP 应该做什么

#### 当前实现 (Mock)

```typescript
completion(text: string, line: number, col: number): string {
  // ❌ 忽略 line 和 col 参数
  // ❌ 只检查 text 中是否有 "SELECT"
  // ❌ 返回硬编码的列表
  return JSON.stringify([
    { label: "SELECT", kind: 14 },
    { label: "users", kind: 5 },
    // ...
  ])
}
```

#### 应该的实现 (Full LSP)

```rust
// 在 crates/lsp/src/completion/mod.rs 中
pub fn completion(
    state: &ServerState,
    params: CompletionParams,
) -> Result<Vec<CompletionItem>> {
    // 1. 获取文档
    let doc = state.document_store.get_document(&params.text_document)?;

    // 2. 查找当前光标位置的节点
    let node = doc.parse_tree().node_at_position(params.position)?;

    // 3. 确定补全上下文
    let context = CompletionContext::from_node(&node)?;

    match context {
        CompletionContext::SelectColumns => {
            // 4. 获取可见的表
            let visible_tables = state.scope_manager.get_visible_tables(&node)?;

            // 5. 收集所有列（去重）
            let mut columns = Vec::new();
            for table in visible_tables {
                let catalog = state.catalog_manager.get_catalog(table)?;
                columns.extend(catalog.get_columns()?);
            }

            Ok(columns)
        }

        CompletionContext::FromClause => {
            // 返回所有表
            state.catalog_manager.get_all_tables()
        }

        // ...
    }
}
```

### 改进方案

#### 阶段 1: 实现基础 LSP 能力 (最小可用版本)

**目标**: 2-3 周内实现

1. **解析器集成**
   - 将 Tree-sitter parser 编译到 WASM
   - 实现 `get_node_at_position()`
   - 实现 `CompletionContext` 检测

2. **符号管理**
   - 实现 `ScopeManager` 的 WASM 版本
   - 跟踪 FROM 子句中的表和别名
   - 维护列的可见性

3. **目录集成**
   - 实现静态 schema (当前 mock data)
   - 支持表和列查询
   - 支持类型信息

**预期效果**:
- ✅ 支持 SELECT 语句的所有子句
- ✅ 识别表别名
- ✅ 位置感知的列补全
- ✅ 基本的错误恢复

#### 阶段 2: 完整 LSP 实现 (生产级)

**目标**: 1-2 月

1. **完整 SQL 支持**
   - INSERT, UPDATE, DELETE
   - DDL (CREATE, ALTER, DROP)
   - 事务 (BEGIN, COMMIT, ROLLBACK)
   - 子查询和 CTE

2. **语义分析**
   - 类型检查
   - 列引用验证
   - 类型推导
   - 函数签名验证

3. **方言特性**
   - MySQL 特定语法
   - PostgreSQL 特定语法
   - 函数差异处理

4. **性能优化**
   - 增量解析
   - 缓存策略
   - 延迟加载

---

## 架构建议

### 当前架构问题

```
┌─────────────────────────────────────┐
│   playground (JavaScript/TypeScript) │
│  ┌──────────────────────────────┐   │
│  │  MockLspServer (JS)          │   │
│  │  - 硬编码数据               │   │
│  │  - 字符串匹配               │   │
│  │  - 无状态                    │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
         ❌ 无法扩展
         ❌ 无复用
         ❌ 功能受限
```

### 推荐架构

```
┌─────────────────────────────────────────────────────────┐
│                    playground                            │
│  ┌──────────────────────────────────────────────┐      │
│  │  WASM LSP (Rust)                               │      │
│  │  ┌─────────────────────────────────────────┐  │      │
│  │  │  Parser (tree-sitter)                   │  │      │
│  │  │  - AST                                  │  │      │
│  │  │  - Node lookup                          │  │      │
│  │  └─────────────────────────────────────────┘  │      │
│  │  ┌─────────────────────────────────────────┐  │      │
│  │  │  ScopeManager                          │  │      │
│  │  │  - 表别名追踪                          │  │      │
│  │  │  - 列可见性                            │  │      │
│  │  └─────────────────────────────────────────┘  │      │
│  │  ┌─────────────────────────────────────────┐  │      │
│  │  │  CompletionEngine                      │  │      │
│  │  │  - 上下文检测                          │  │      │
│  │  │  - 建议生成                            │  │      │
│  │  └─────────────────────────────────────────┘  │      │
│  │  ┌─────────────────────────────────────────┐  │      │
│  │  │  Catalog                               │  │      │
│  │  │  - Schema 数据                         │  │      │
│  │  │  - 类型信息                            │  │      │
│  │  └─────────────────────────────────────────┘  │      │
│  └──────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────┘
         ✅ 可扩展
         ✅ 代码复用
         ✅ 功能完整
```

### 模块划分

```
crates/
├── lsp-wasm/                 # 新建独立的 WASM LSP crate
│   ├── Cargo.toml            # 独立于 workspace (无 tokio 依赖)
│   ├── src/
│   │   ├── lib.rs            # WASM 导出
│   │   ├── parser.rs         # Tree-sitter 包装
│   │   ├── scope.rs          # ScopeManager WASM 版
│   │   ├── completion.rs     # 补全逻辑
│   │   └── catalog.rs        # 静态 schema 数据
│   └── build.rs              # WASM 构建脚本
│
├── grammar/                   # 已存在
│   └── parser.c              # Tree-sitter parser
│
├── semantic/                 # 已存在，需要重构
│   └── src/
│       ├── scope.rs          # ← 提取平台无关部分
│       └── alias.rs          # ← 移除 async 依赖
│
└── catalog/                   # 已存在
    └── src/
        └── static.rs         # ← 可直接用于 WASM
```

---

## 实现路线图

### 第 1 阶段: 基础补全增强 (1-2 周)

**目标**: 支持 SELECT/INSERT/UPDATE/DELETE 语句的基础补全

- [ ] 实现基于状态机的语句类型检测
- [ ] 支持 INSERT INTO ... VALUES 补全
- [ ] 支持 UPDATE ... SET 补全
- [ ] 支持 DELETE FROM ... WHERE 补全
- [ ] 改进关键词补全的上下文感知

**验收标准**:
```sql
-- 应该工作
SELECT |  FROM users;
INSERT INTO |
UPDATE users SET |
DELETE FROM users WHERE |

-- 应该提示正确的内容
INSERT INTO users (|)  → 列名列表
UPDATE users SET |   → 列名列表
```

### 第 2 阶段: 表别名支持 (2-3 周)

**目标**: 识别和跟踪表别名

- [ ] 实现 FROM 子句解析 (正则或简单解析)
- [ ] 维护别名到表的映射
- [ ] 支持 `table_alias.` 形式的列补全
- [ ] 支持 JOIN 的别名解析

**验收标准**:
```sql
SELECT u.| FROM users u;  → 提示 users 表的列
FROM users u JOIN orders o ON | → 提示 id, user_id 等
```

### 第 3 阶段: Tree-sitter 集成 (3-4 周)

**目标**: 使用真正的 SQL 解析器

- [ ] 将 Tree-sitter 编译到 WASM
- [ ] 实现节点查找 API
- [ ] 实现 CompletionContext 检测
- [ ] 错误恢复机制

**验收标准**:
- 正确处理嵌套查询
- 正确处理子查询
- 部分语法错误时仍然提供补全

### 第 4 阶段: 完整 LSP 功能 (4-6 周)

**目标**: 达到生产级 LSP 服务器

- [ ] 所有 SQL 语句类型支持
- [ ] 语义诊断 (列不存在、类型错误)
- [ ] Hover 信息 (表结构、函数签名)
- [ ] 代码格式化
- [ ] 重构支持

---

## 技术债务清单

| ID | 问题 | 优先级 | 估算时间 | 依赖 |
|----|------|--------|----------|------|
| P0-1 | 只支持 SELECT 语句 | P0 | 1-2 周 | - |
| P0-2 | 无法识别表别名 | P0 | 2-3 周 | P0-1 |
| P0-3 | 退化成字符串匹配 | P0 | 3-4 周 | - |
| P1-1 | 无错误恢复 | P1 | 1 周 | P0-3 |
| P1-2 | 无位置感知 | P1 | 1 周 | P0-3 |
| P1-3 | 无增量更新 | P2 | 1 周 | P0-3 |
| P2-1 | 性能优化 | P2 | 1 周 | 全部 |
| P2-2 | 方言差异 | P2 | 2-3 周 | 全部 |

---

## 参考资料

### 已有实现

**主仓库中的相关代码**:

1. **Completion Engine**
   - `crates/lsp/src/completion/`
   - 已实现完整的补全逻辑
   - 需要提取 WASM 兼容部分

2. **Scope Manager**
   - `crates/semantic/src/scope.rs`
   - 已实现别名追踪和列可见性
   - 需要移除 async 依赖

3. **Alias Resolver**
   - `crates/semantic/src/alias.rs`
   - 4 种别名解析策略
   - 可以直接移植

4. **Catalog**
   - `crates/catalog/src/static.rs`
   - 静态 schema 支持
   - WASM 兼容

### 相关文档

- **LSP 规范**: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
- **Tree-sitter**: https://tree-sitter.github.io/tree-sitter/
- **Monaco Editor API**: https://microsoft.github.io/monaco-editor/api/index.html

---

## 下一步行动

### 立即行动 (本周)

1. **评估方案选择**
   - [ ] 决定使用方案 A (简单解析器) 还是方案 B (Tree-sitter)
   - [ ] 评估资源需求
   - [ ] 确定时间表

2. **技术验证**
   - [ ] Tree-sitter WASM 编译测试
   - [ ] 性能基准测试
   - [ ] Bundle 大小评估

### 短期 (2-4 周)

1. **实现基础增强**
   - 选择方案 A 或 B
   - 实现多语句类型支持
   - 实现基础别名解析

2. **集成到 playground**
   - 更新 wasm-interface.ts
   - 移除 JavaScript mock
   - 端到端测试

### 中期 (1-2 月)

1. **完整 LSP 功能**
   - Tree-sitter 深度集成
   - 所有语句类型支持
   - 语义分析

2. **生产就绪**
   - 性能优化
   - 错误处理
   - 文档完善

---

## 联系信息

**实现者**: Claude (AI Assistant)
**代码位置**:
- Playground: `/home/woxQAQ/unified-sql-lsp/.worktrees/playground/`
- 主仓库: `/home/woxQAQ/unified-sql-lsp/`

**相关提交**:
- playground: `26e3c09` - feat(playground): complete SQL LSP playground implementation
- main: `231fc7c` - feat(lsp): add WASM support infrastructure for playground

**问题反馈**: 请在 GitHub Issues 中提交，标签 `playground` 和 `lsp-wasm`
