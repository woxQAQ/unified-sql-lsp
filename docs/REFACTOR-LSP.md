# LSP Crate 重构方案

## 一、问题分析

### 1.1 当前状态

**LSP crate 承担的职责：**

```
crates/lsp/
├── src/
│   ├── completion/
│   │   ├── context.rs         # 上下文检测（基于 CST 分析）
│   │   ├── scopes.rs          # 作用域构建（语义分析）
│   │   ├── catalog_integration.rs  # Catalog 集成
│   │   ├── render.rs          # LSP 补全项渲染
│   │   ├── keywords.rs        # 关键字提供者
│   │   └── error.rs           # 错误类型
│   ├── backend.rs             # LSP 协议处理
│   ├── symbols.rs             # 符号构建（语义分析）
│   ├── definition.rs          # 定义跳转（语义分析）
│   └── ...
```

**问题点：**

1. **职责不清**：
   - `completion/context.rs` 通过分析 CST 节点来判断补全上下文（SELECT/FROM/WHERE 等）
   - `completion/scopes.rs` 从 CST 构建语义作用域
   - `symbols.rs` 从 CST 提取符号信息
   - 这些都是**语义分析**的职责，不应在 LSP 层

2. **违反设计文档**：
   ```
   DESIGN.md 定义的架构：

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
   ```

   LSP 层应该只负责 LSP 协议转换，不应该直接操作 CST 或构建作用域。

3. **重复实现**：
   - `crates/semantic/` 已有完整的语义分析器（`SemanticAnalyzer`）
   - `crates/lsp/completion/scopes.rs` 又实现了一个轻量级的 `ScopeBuilder`
   - 代码中明确写着 `TODO: (SEMANTIC-002) Replace with full semantic analyzer when available`

4. **架构耦合**：
   - `backend.rs` 直接创建 `CompletionEngine`
   - `CompletionEngine` 直接访问 `Document` 的 tree-sitter Tree
   - LSP 层与底层解析实现强耦合

### 1.2 根本原因

**语义分析器未完成时的临时方案变成了长期实现：**

1. 为了快速实现补全功能，在 LSP crate 中实现了"够用"的语义分析
2. 没有及时重构回 Semantic 层，导致技术债务累积
3. 随着功能增加，LSP crate 越来越臃肿

---

## 二、重构目标

### 2.1 职责划分

**LSP Layer（crates/lsp/）：**

唯一职责：**LSP 协议适配**

```rust
// LSP 层只做协议转换
impl LanguageServer for LspBackend {
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // 1. 获取文档
        let document = self.documents.get_document(&uri).await?;

        // 2. 调用 Semantic 层获取补全建议（语义无关）
        let suggestions = self.semantic_engine.get_completion_suggestions(
            &document,
            position
        ).await?;

        // 3. 转换为 LSP 格式
        let items = self.to_lsp_items(suggestions);
        Ok(Some(CompletionResponse::Array(items)))
    }
}
```

**Semantic Layer（crates/semantic/）：**

职责：**上下文感知 + 符号解析**

```rust
// Semantic 层返回语义丰富的补全建议
pub struct CompletionSuggestion {
    pub kind: SuggestionKind,      // Table / Column / Function / Keyword
    pub name: String,              // "users", "id"
    pub qualifier: Option<String>,  // "u" for "u.id"
    pub type_info: Option<Type>,   // INT, VARCHAR
    pub documentation: Option<String>,
}
```

**Context Layer（crates/context/）：**

职责：**基于 CST 的上下文检测**（新增 crate）

```rust
// Context 层分析 CST，判断补全触发点
pub struct CompletionContext {
    pub location: CompletionLocation,  // SelectProjection / FromClause / WhereClause
    pub visible_tables: Vec<TableRef>,
    pub qualifier: Option<String>,
}
```

### 2.2 新增 Context Crate

**创建 `crates/context/` 的原因：**

1. **上下文检测是语法层面的分析**，不是语义分析
2. **基于 CST 节点类型和位置判断**，不需要符号表
3. **可独立测试**，不依赖 Catalog 或 Semantic
4. **可复用**：Completion、Hover、Diagnostics 都需要上下文检测

**职责划分：**

| 层次       | 职责                     | 输入          | 输出                |
|------------|--------------------------|---------------|---------------------|
| Grammar    | CST 解析                 | 源代码        | tree-sitter Tree    |
| **Context** | **上下文检测**           | **Tree + 位置** | **CompletionContext** |
| Lowering   | CST → IR                 | Tree          | IR                  |
| Semantic   | 符号解析                 | IR + Catalog  | ScopeManager        |
| LSP        | 协议适配                 | Semantic 结果 | LSP CompletionItem  |

---

## 三、重构方案

### 3.1 新增 crates/context/

**目录结构：**

```
crates/context/
├── Cargo.toml
└── src/
    ├── lib.rs              # 导出公共 API
    ├── completion.rs       # 补全上下文检测
    ├── hover.rs            # Hover 上下文检测
    ├── diagnostic.rs       # 诊断上下文检测
    └── cst_utils.rs        # CST 工具函数（从 lsp/cst_utils.rs 迁移）
```

**核心 API：**

```rust
// crates/context/src/completion.rs

/// 补全上下文类型
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionLocation {
    SelectProjection { tables: Vec<String>, qualifier: Option<String> },
    FromClause { exclude_tables: Vec<String> },
    WhereClause { tables: Vec<String>, qualifier: Option<String> },
    JoinCondition { left_table: Option<String>, right_table: Option<String> },
    OrderByClause { tables: Vec<String>, qualifier: Option<String> },
    Keywords { statement_type: Option<String> },
    Unknown,
}

/// 补全上下文（纯语法分析结果）
pub struct CompletionContext {
    pub location: CompletionLocation,
    pub trigger_range: Range,
    pub current_token: Option<String>,
}

impl CompletionContext {
    /// 从 tree-sitter Tree 检测补全上下文
    pub fn detect(tree: &Tree, source: &str, position: Position) -> Self {
        // 1. 找到光标位置的节点
        let node = find_node_at_position(tree.root_node(), position);

        // 2. 向上遍历判断上下文类型
        if is_in_select_projection(&node) {
            let tables = extract_visible_tables(&node);
            let qualifier = extract_qualifier(&node);
            CompletionContext {
                location: CompletionLocation::SelectProjection { tables, qualifier },
                ...
            }
        } else if is_in_from_clause(&node) {
            // ...
        }
    }
}
```

**依赖关系：**

```toml
[dependencies]
unified-sql-grammar = { path = "../grammar" }
tree-sitter = "0.26"
```

**特点：**

- ✅ 零依赖 Semantic、Catalog、IR
- ✅ 只依赖 Grammar（tree-sitter Tree）
- ✅ 纯函数式，易于测试
- ✅ 可独立发布

### 3.2 重构 crates/semantic/

**迁移职责：**

1. **从 LSP 迁移：**
   - `lsp/completion/scopes.rs` → `context/completion.rs`（上下文检测）
   - `lsp/completion/scopes.rs` → `semantic/scope.rs`（作用域构建，增强现有实现）
   - `lsp/symbols.rs` → `semantic/symbol_extractor.rs`（符号提取）

2. **新增功能：**
   - 补全建议生成（`CompletionProvider`）
   - Hover 信息生成（`HoverProvider`）
   - 诊断信息生成（`DiagnosticProvider`）

**新 API：**

```rust
// crates/semantic/src/completion.rs

/// 补全建议（语义丰富）
#[derive(Debug, Clone)]
pub struct CompletionSuggestion {
    pub kind: CompletionSuggestionKind,
    pub label: String,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub type_info: Option<DataType>,
}

#[derive(Debug, Clone)]
pub enum CompletionSuggestionKind {
    Table { name: String, schema: String },
    Column { name: String, table: String, type_info: DataType },
    Function { name: String, func_type: FunctionType },
    Keyword { name: String },
}

/// 补全提供者
pub struct CompletionProvider {
    catalog: Arc<dyn Catalog>,
    analyzer: SemanticAnalyzer,
}

impl CompletionProvider {
    /// 获取补全建议
    pub async fn get_suggestions(
        &self,
        tree: &Tree,
        source: &str,
        position: Position,
        dialect: Dialect,
    ) -> Result<Vec<CompletionSuggestion>, SemanticError> {
        // 1. 使用 Context 层检测上下文
        let ctx = CompletionContext::detect(tree, source, position);

        // 2. 根据上下文生成建议
        match ctx.location {
            CompletionLocation::SelectProjection { tables, qualifier } => {
                self.complete_select_projection(&tables, qualifier.as_deref()).await
            }
            CompletionLocation::FromClause { exclude_tables } => {
                self.complete_from_clause(&exclude_tables).await
            }
            // ...
        }
    }
}
```

**依赖关系：**

```toml
[dependencies]
unified-sql-lsp-context = { path = "../context" }
unified-sql-lsp-lowering = { path = "../lowering" }
unified-sql-lsp-catalog = { path = "../catalog" }
```

### 3.3 精简 crates/lsp/

**保留职责：**

1. LSP 协议处理（`backend.rs`）
2. 文档管理（`document.rs`，`DocumentStore`）
3. LSP 格式转换（`completion/lsp_adapter.rs`）

**移除职责：**

1. ❌ CST 操作 → `context/cst_utils.rs`
2. ❌ 上下文检测 → `context/completion.rs`
3. ❌ 作用域构建 → `semantic/`
4. ❌ Catalog 集成 → `semantic/`
5. ❌ 关键字提供 → `context/keywords.rs`

**新的目录结构：**

```
crates/lsp/
├── src/
│   ├── lib.rs
│   ├── backend.rs              # LSP 协议处理（精简后）
│   ├── document.rs             # 文档管理
│   ├── config.rs               # 配置管理
│   ├── catalog_manager.rs      # Catalog 管理器
│   └── lsp_adapter/
│       ├── completion.rs       # Semantic → LSP 转换
│       ├── hover.rs            # Semantic → LSP 转换
│       └── diagnostic.rs       # Semantic → LSP 转换
└── Cargo.toml
```

**精简后的 backend.rs：**

```rust
// crates/lsp/src/backend.rs

use unified_sql_lsp_semantic::CompletionProvider;

pub struct LspBackend {
    client: Client,
    documents: Arc<DocumentStore>,
    catalog_manager: Arc<RwLock<CatalogManager>>,
    completion_provider: Arc<CompletionProvider>,  // 从 Semantic 层获取
}

impl LanguageServer for LspBackend {
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // 1. 获取文档
        let document = self.documents.get_document(&uri).await?;

        // 2. 获取 Catalog
        let config = self.get_config().await?;
        let catalog = self.catalog_manager.read().await.get_catalog(&config).await?;

        // 3. 调用 Semantic 层获取补全建议
        let suggestions = self.completion_provider
            .get_suggestions(document.tree(), document.get_content(), position, config.dialect)
            .await?;

        // 4. 转换为 LSP 格式
        let items = self.to_lsp_completion_items(suggestions);
        Ok(Some(CompletionResponse::Array(items)))
    }
}
```

**LSP 适配器：**

```rust
// crates/lsp/src/lsp_adapter/completion.rs

use tower_lsp::lsp_types::CompletionItem;
use unified_sql_lsp_semantic::{CompletionSuggestion, CompletionSuggestionKind};

pub fn to_lsp_completion_items(suggestions: Vec<CompletionSuggestion>) -> Vec<CompletionItem> {
    suggestions.into_iter().map(|suggestion| {
        let (kind, detail) = match suggestion.kind {
            CompletionSuggestionKind::Table { .. } => (CompletionItemKind::Class, Some("Table".to_string())),
            CompletionSuggestionKind::Column { type_info, .. } => (CompletionItemKind::Field, Some(type_info.to_string())),
            CompletionSuggestionKind::Function { .. } => (CompletionItemKind::Function, None),
            CompletionSuggestionKind::Keyword { .. } => (CompletionItemKind::Keyword, None),
        };

        CompletionItem {
            label: suggestion.label,
            kind: Some(kind),
            detail,
            documentation: suggestion.documentation.map(|d| lsp_types::Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: d,
            })),
            ..Default::default()
        }
    }).collect()
}
```

### 3.4 依赖关系变化

**重构前：**

```
lsp/
├── grammar/     (tree-sitter)
├── ir/
├── lowering/
├── semantic/    (部分使用)
└── catalog/     (直接访问)
```

**重构后：**

```
lsp/
└── semantic/    (只依赖 Semantic 的公共 API)

semantic/
├── context/     (新增)
├── lowering/
└── catalog/

context/
└── grammar/     (tree-sitter)
```

---

## 四、实施计划

### 4.1 阶段一：创建 Context Crate（1-2 天）

**任务清单：**

- [ ] 创建 `crates/context/` 目录结构
- [ ] 实现 `context/completion.rs`（迁移 `lsp/completion/context.rs`）
- [ ] 实现 `context/cst_utils.rs`（迁移 `lsp/cst_utils.rs`）
- [ ] 实现 `context/keywords.rs`（迁移 `lsp/completion/keywords.rs`）
- [ ] 编写单元测试
- [ ] 更新 workspace Cargo.toml

**验收标准：**

- ✅ Context crate 可以独立编译和测试
- ✅ 补全上下文检测准确率 > 95%
- ✅ 无依赖 Semantic、Catalog、IR

### 4.2 阶段二：增强 Semantic Crate（2-3 天）

**任务清单：**

- [ ] 实现 `semantic/completion.rs`（新增）
- [ ] 实现 `CompletionProvider::complete_select_projection()`
- [ ] 实现 `CompletionProvider::complete_from_clause()`
- [ ] 实现 `CompletionProvider::complete_where_clause()`
- [ ] 实现 `CompletionProvider::complete_join_condition()`
- [ ] 迁移 `lsp/completion/scopes.rs` 的作用域构建逻辑
- [ ] 集成 Context crate 进行上下文检测
- [ ] 编写集成测试

**验收标准：**

- ✅ CompletionProvider 可以返回语义丰富的补全建议
- ✅ 单元测试覆盖率 > 80%
- ✅ 集成测试覆盖所有补全场景

### 4.3 阶段三：重构 LSP Crate（2-3 天）

**任务清单：**

- [ ] 移除 `lsp/completion/context.rs`（已迁移到 context）
- [ ] 移除 `lsp/completion/scopes.rs`（已迁移到 semantic）
- [ ] 移除 `lsp/completion/keywords.rs`（已迁移到 context）
- [ ] 移除 `lsp/cst_utils.rs`（已迁移到 context）
- [ ] 精简 `lsp/backend.rs`，使用 Semantic 层 API
- [ ] 新增 `lsp/lsp_adapter/completion.rs`
- [ ] 更新 LSP 层测试
- [ ] 运行 E2E 测试确保兼容性

**验收标准：**

- ✅ LSP crate 只包含协议适配逻辑
- ✅ 所有现有测试通过
- ✅ E2E 测试无回归

### 4.4 阶段四：文档和清理（1 天）

**任务清单：**

- [ ] 更新 DESIGN.md 反映新架构
- [ ] 更新 CLAUDE.md 的模块说明
- [ ] 添加架构决策记录（ADR）
- [ ] 清理未使用的依赖
- [ ] 移除废弃的代码

---

## 五、风险评估

### 5.1 风险点

| 风险                     | 影响 | 缓解措施                         |
|--------------------------|------|----------------------------------|
| 重构引入新 Bug           | 高   | 完善的单元测试 + E2E 测试        |
| 性能下降                 | 中   | 性能基准测试，对比重构前后       |
| API 变更影响使用者       | 低   | LSP 层对外 API 保持不变          |
| 开发周期延长             | 中   | 分阶段实施，每阶段独立交付       |

### 5.2 回滚计划

每个阶段独立 Git 分支：

- `refactor/context-crate` （阶段一）
- `refactor/semantic-completion` （阶段二）
- `refactor/lsp-adapter` （阶段三）

如果某个阶段出现问题，可以：
1. 回滚到上一个稳定分支
2. 保留已完成的工作（Context crate 可独立使用）
3. 修复问题后继续

---

## 六、预期收益

### 6.1 架构清晰

- ✅ LSP 层只负责协议适配
- ✅ Semantic 层专注语义分析
- ✅ Context 层专注上下文检测
- ✅ 各层职责明确，易于理解和维护

### 6.2 可测试性

- ✅ Context 层可独立单元测试（无外部依赖）
- ✅ Semantic 层可 Mock Catalog 进行测试
- ✅ LSP 层可 Mock Semantic 进行测试

### 6.3 可扩展性

- ✅ 新增 LSP 功能（如 Format）只需添加 LSP 适配器
- ✅ 新增上下文类型只需扩展 Context 层
- ✅ 新增语义分析只需扩展 Semantic 层

### 6.4 性能优化

- ✅ Context 层可做编译时优化（无状态函数）
- ✅ Semantic 层可缓存 ScopeManager
- ✅ LSP 层可批量处理请求

---

## 七、后续优化

### 7.1 短期（完成后 1-2 周）

1. **性能优化**：
   - 补全延迟优化（< 50ms p95）
   - 缓存策略优化（ArcSwap 无锁更新）

2. **功能完善**：
   - 补全排序（相关性评分）
   - 补全过滤（前缀匹配）
   - Snippet 支持（代码片段）

### 7.2 中期（完成后 1-2 月）

1. **新增 LSP 功能**：
   - Hover（使用 Semantic 层）
   - Diagnostics（使用 Semantic 层）
   - Code Actions（使用 Semantic 层）

2. **Schema 感知增强**：
   - 实时 Schema 同步
   - 跨文件 Schema 解析
   - Schema 缓存预热

### 7.3 长期（完成后 3-6 月）

1. **多文件分析**：
   - 跨文件引用解析
   - 项目级 ScopeManager

2. **AI 增强**：
   - 基于 ML 的补全排序
   - 上下文感知推荐

---

## 八、总结

### 8.1 重构必要性

✅ **必须重构**：当前 LSP crate 承担了过多职责，违反了设计文档的分层架构原则。

### 8.2 重构可行性

✅ **可行**：
- Context 层是新增模块，不影响现有代码
- Semantic 层已有基础，只需增强
- LSP 层对外 API 保持不变，不影响使用者

### 8.3 推荐实施顺序

```
阶段一（Context） → 阶段二（Semantic） → 阶段三（LSP） → 阶段四（文档）
  1-2 天            2-3 天               2-3 天           1 天
```

**总时间：6-9 天**

### 8.4 关键成功因素

1. ✅ 保持 LSP 对外 API 不变
2. ✅ 完善的测试覆盖（单元 + 集成 + E2E）
3. ✅ 分阶段独立交付
4. ✅ 持续集成验证

---

**文档版本**: v1.0
**创建日期**: 2025-01-13
**维护者**: unified-sql-lsp team
**状态**: 待审核
