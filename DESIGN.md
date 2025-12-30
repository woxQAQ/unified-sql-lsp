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
│  - MySQL (5.7, 8.0+)                                            │
│  - PostgreSQL (12+, 13+, 14+, ...)                              │
│  - SQLite (3.x)                                                 │
│  - 30+ 其他引擎                                                  │
├─────────────────────────────────────────────────────────────────┤
│                       SQL IR / AST Layer                        │
│  - Unified Query / Expr / Statement types                      │
│  - 方言无关的中间表示                                            │
├─────────────────────────────────────────────────────────────────┤
│                    Tree-sitter Grammar Layer                    │
│  - Incremental CST parsing                                      │
│  - Error recovery                                                │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 模块依赖关系

```
lsp/        ──────┐
                 ├───  semantic/
semantic/    ────┘       │
                      ───┴───  ir/
lowering/   ────────────┘
  │
  └───  grammar/
       └─── tree-sitter-* (external)
```

---

## 三、核心模块设计

### 3.1 Grammar Layer (crates/grammar/)

#### 职责

- 封装 Tree-sitter 解析器
- 提供 CST 查询接口
- 管理方言特定的 Grammar

#### 设计

```rust
// Grammar trait: 统一的语法解析接口
pub trait Grammar: Send + Sync {
    /// 解析源码生成 CST
    fn parse(&self, source: &str) -> Tree;

    /// 根据版本判断支持的特性
    fn supports_feature(&self, feature: GrammarFeature) -> bool;

    /// 获取方言类型
    fn dialect(&self) -> Dialect;
}

// 方言枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialect {
    MySQL { version: (u8, u8) },
    PostgreSQL { version: (u8, u8) },
    SQLite,
    // ... 其他 30+ 引擎
}

// 语法特性标记
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammarFeature {
    WindowFunctions,
    CTEs,
    LateralJoins,
    JsonFunctions,
    // ...
}

// Grammar 工厂: 根据引擎类型创建 Grammar
pub struct GrammarFactory {
    grammars: DashMap<Dialect, Arc<dyn Grammar>>,
}

impl GrammarFactory {
    pub fn get(&self, dialect: Dialect) -> Arc<dyn Grammar> {
        // 缓存 Grammar 实例
    }
}
```

#### Grammar 选择策略

**方案 A（推荐）: Base Grammar + Dialect Patch**

```
tree-sitter-sql-core/
  ├── base-grammar.js          # 核心 SQL 语法
  └── queries/                 # 基础查询模式

tree-sitter-sql-mysql/
  ├── grammar.js               # 继承 core
  ├── scanner.c                # MySQL 特定词法
  └── features/                # MySQL 扩展

tree-sitter-sql-postgres/
  ├── grammar.js               # 继承 core
  └── features/                # PG 扩展
```

**优势**：
- 核心语法复用，减少重复
- 方言差异隔离，易于维护
- 符合 Tree-sitter 的组合式设计理念

#### Grammar 编辑器友好设计

**关键原则**：

1. **显式命名节点**：便于查询
   ```javascript
   select_clause
   from_clause
   join_clause
   table_reference
   column_reference
   ```

2. **避免过度合并**：保持查询粒度
   ```javascript
   // ❌ 不好
   statement ::= select | insert | update | delete

   // ✅ 好
   select_statement
   insert_statement
   update_statement
   delete_statement
   ```

3. **保留错误节点**：支持错误恢复
   ```javascript
   (ERROR) @error
   ```

---

### 3.2 Lowering Layer (crates/lowering/)

#### 职责

- 将 CST 转换为统一 IR
- 吸收方言语法差异
- 处理语法糖和简化结构

#### 设计

```rust
// Lowering trait: CST → IR 转换
pub trait Lowering: Send + Sync {
    /// 转换 SELECT 语句
    fn lower_select(&self, node: Node) -> Result<SelectStmt, LoweringError>;

    /// 转换表达式
    fn lower_expr(&self, node: Node) -> Result<Expr, LoweringError>;

    /// 转换表引用
    fn lower_table_ref(&self, node: Node) -> Result<TableRef, LoweringError>;

    /// 获取方言
    fn dialect(&self) -> Dialect;
}

// 方言特定的 Lowering 实现
pub struct MySQLLowering {
    version: (u8, u8),
}

impl Lowering for MySQLLowering {
    fn lower_select(&self, node: Node) -> Result<SelectStmt, LoweringError> {
        // MySQL 特定的转换逻辑
        // 例如: MySQL 的 LIMIT syntax
    }
}

// Lowering 工厂
pub struct LoweringFactory {
    lowerings: DashMap<Dialect, Arc<dyn Lowering>>,
}

impl LoweringFactory {
    pub fn get(&self, dialect: Dialect) -> Arc<dyn Lowering> {
        // 返回方言对应的 Lowering 实现
    }
}
```

#### 版本兼容处理

```rust
impl MySQLLowering {
    fn handle_limit(&self, node: Node) -> Option<(Expr, Option<Expr>)> {
        match self.version {
            (8, 0) => {
                // MySQL 8.0+ 支持 FETCH / OFFSET
                self.lower_fetch_offset(node)
            }
            _ => {
                // MySQL 5.7 使用 LIMIT offset, count
                self.lower_limit_comma(node)
            }
        }
    }
}
```

---

### 3.3 IR Layer (crates/ir/) - 已有基础

#### 现有类型

- ✅ `Stmt`: 语句枚举
- ✅ `Query`: 查询类型
- ✅ `SelectStmt`: SELECT 语句
- ✅ `Expr`: 表达式
- ✅ `ObjectName`: 对象名称
- ✅ `TableRef`, `Join`: 表引用

#### 需要扩展

```rust
// 添加方言特定信息的扩展点
#[derive(Debug, Clone, PartialEq)]
pub struct SelectStmt {
    pub distinct: bool,
    pub projections: Vec<Expr>,
    pub from: Vec<TableRef>,
    pub joins: Vec<Join>,
    pub where_clause: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub window_clauses: Vec<WindowDef>,      // 新增: 窗口子句
    pub qualify: Option<Expr>,                // 新增: QUALIFY (Snowflake, BigQuery)
    pub dialect_extensions: DialectExtensions, // 新增: 方言扩展
}

// 窗口定义
#[derive(Debug, Clone, PartialEq)]
pub struct WindowDef {
    pub name: Option<String>,
    pub partition_by: Vec<Expr>,
    pub order_by: Vec<OrderByExpr>,
    pub window_frame: Option<WindowFrame>,
}

// 方言扩展（用于未来兼容）
#[derive(Debug, Clone, PartialEq)]
pub enum DialectExtensions {
    MySQL(MySQLExtensions),
    PostgreSQL(PostgreSQLExtensions),
    Unknown,
}
```

---

### 3.4 Semantic Layer (crates/semantic/)

#### 职责

- 构建作用域和符号表
- 解析列引用和表别名
- 提供补全触发点判断

#### 核心数据结构

```rust
/// 作用域：包含当前可见的表和列
#[derive(Debug, Clone)]
pub struct Scope {
    pub tables: Vec<TableSymbol>,
    parent: Option<Box<Scope>>,
}

/// 表符号
#[derive(Debug, Clone)]
pub struct TableSymbol {
    pub name: String,           // 表名或别名
    pub actual_name: String,    // 实际表名
    pub columns: Vec<ColumnSymbol>,
    pub span: Span,
}

/// 列符号
#[derive(Debug, Clone)]
pub struct ColumnSymbol {
    pub name: String,
    pub table_name: Option<String>,  // 所属表（如果可确定）
    pub data_type: Option<DataType>,
    pub is_nullable: bool,
}

/// 语义分析结果
#[derive(Debug, Clone)]
pub struct SemanticModel {
    pub scopes: Vec<Scope>,
    pub diagnostics: Vec<Diagnostic>,
}
```

#### 语义分析器

```rust
pub struct SemanticAnalyzer {
    catalog: Arc<dyn Catalog>,
    dialect: Dialect,
}

impl SemanticAnalyzer {
    /// 分析语句，构建语义模型
    pub fn analyze(&self, stmt: &Stmt) -> SemanticModel {
        let mut scope = Scope::new();

        match stmt {
            Stmt::Query(query) => self.analyze_query(query, &mut scope),
            Stmt::Insert { .. } => { /* ... */ }
            _ => { /* ... */ }
        }

        SemanticModel {
            scopes: vec![scope],
            diagnostics: Vec::new(),
        }
    }

    /// 分析查询，收集表和列信息
    fn analyze_query(&self, query: &Query, scope: &mut Scope) {
        let Query { body, .. } = query;

        match body {
            SetExpr::Select(select) => {
                // 1. FROM 子句：收集表
                for table_ref in &select.from {
                    self.collect_table(table_ref, scope);
                }

                // 2. JOIN：收集表
                for join in &select.joins {
                    self.collect_table(&join.relation, scope);
                }

                // 3. 解析列引用
                for proj in &select.projections {
                    self.resolve_expr(proj, scope);
                }
            }
            _ => { /* ... */ }
        }
    }

    /// 收集表信息
    fn collect_table(&self, table_ref: &TableRef, scope: &mut Scope) {
        let TableRef { name, alias, .. } = table_ref;

        let table_name = name.to_string();
        let alias_name = alias.as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| table_name.clone());

        // 从 Catalog 获取列信息
        let columns = self.catalog
            .get_columns(&table_name)
            .unwrap_or_default();

        let symbol = TableSymbol {
            name: alias_name,
            actual_name: table_name,
            columns,
            span: name.span,
        };

        scope.tables.push(symbol);
    }

    /// 解析表达式（列引用）
    fn resolve_expr(&self, expr: &Expr, scope: &Scope) -> Vec<ColumnSymbol> {
        match expr {
            Expr::Identifier(name) => {
                // 查找列定义
                self.resolve_column(name, scope)
            }
            Expr::QualifiedWildcard(table_name) => {
                // table.* → 返回该表所有列
                self.resolve_table_wildcard(table_name, scope)
            }
            _ => Vec::new(),
        }
    }

    /// 解析列引用
    fn resolve_column(&self, name: &ObjectName, scope: &Scope) -> Vec<ColumnSymbol> {
        let candidates = scope.find_column(name);

        if candidates.is_empty() {
            // 未定义的列 → 诊断
            vec![]
        } else if candidates.len() > 1 {
            // 歧义 → 诊断
            vec![]
        } else {
            candidates
        }
    }
}
```

#### 补全触发点判断

```rust
/// 补全上下文
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionContext {
    /// SELECT 子句中的列
    SelectProjection,
    /// FROM 子句中的表
    FromTable,
    /// JOIN 条件中的列
    JoinCondition,
    /// WHERE 子句中的列
    WhereClause,
    /// 限定符后的列 (table.)
    QualifiedColumn(String),
    /// 函数调用
    FunctionCall,
    /// 无上下文
    None,
}

impl SemanticAnalyzer {
    /// 判断补全触发点
    pub fn get_completion_context(
        &self,
        position: Position,
        cst: &Tree,
    ) -> CompletionContext {
        // 1. 使用 Tree-sitter 查找光标位置的最小节点
        let node = cst.root_node()
            .descendant_for_point_range(position, position)?;

        // 2. 向上遍历语法树，判断上下文
        for ancestor in node.ancestors() {
            match ancestor.kind() {
                "select_list" => return CompletionContext::SelectProjection,
                "from_clause" => return CompletionContext::FromTable,
                "join_clause" => return CompletionContext::JoinCondition,
                "where_clause" => return CompletionContext::WhereClause,
                _ => continue,
            }
        }

        CompletionContext::None
    }
}
```

---

### 3.5 Catalog Layer (crates/catalog/)

#### 职责

- 提供数据库 Schema 抽象
- 支持多种 Schema 来源（静态、动态、缓存）
- 提供表/列/函数的元数据查询

#### 设计

```rust
/// Catalog trait: Schema 查询接口
#[async_trait]
pub trait Catalog: Send + Sync {
    /// 获取所有表
    async fn list_tables(&self) -> Result<Vec<TableMetadata>;

    /// 获取表的所有列
    async fn get_columns(&self, table: &str) -> Result<Vec<ColumnMetadata>>;

    /// 获取函数列表
    async fn list_functions(&self) -> Result<Vec<FunctionMetadata>>;

    /// 搜索表/列（模糊匹配）
    async fn search(&self, pattern: &str) -> Result<Vec<SearchResult>>;
}

/// 表元数据
#[derive(Debug, Clone)]
pub struct TableMetadata {
    pub name: String,
    pub schema: Option<String>,
    pub columns: Vec<ColumnMetadata>,
    pub is_view: bool,
}

/// 列元数据
#[derive(Debug, Clone)]
pub struct ColumnMetadata {
    pub name: String,
    pub data_type: DataType,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
}

/// 函数元数据
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    pub name: String,
    pub return_type: DataType,
    pub args: Vec<FunctionArg>,
    pub is_aggregate: bool,
    pub is_window: bool,
}
```

#### Catalog 实现

**1. 动态 Catalog（实时连接）**

```rust
pub struct LiveCatalog {
    pool: AnyDatabasePool,  // 支持多种数据库连接池
}

#[async_trait]
impl Catalog for LiveCatalog {
    async fn list_tables(&self) -> Result<Vec<TableMetadata>> {
        match self.pool.dialect() {
            Dialect::MySQL => self.query_mysql_tables().await,
            Dialect::PostgreSQL => self.query_pg_tables().await,
            _ => Err(...),
        }
    }
}
```

**2. 静态 Catalog（文件定义）**

```rust
pub struct StaticCatalog {
    tables: Vec<TableMetadata>,
}

impl StaticCatalog {
    pub fn from_file(path: &Path) -> Result<Self> {
        // 从 JSON/YAML 文件加载
        let content = fs::read_to_string(path)?;
        let tables: Vec<TableMetadata> = serde_json::from_str(&content)?;
        Ok(Self { tables })
    }
}
```

**3. 缓存 Catalog（LRU + TTL）**

```rust
pub struct CachedCatalog {
    inner: Arc<dyn Catalog>,
    cache: Arc<Mutex<LruCache<String, CacheEntry>>>,
    ttl: Duration,
}

struct CacheEntry {
    data: Vec<TableMetadata>,
    timestamp: Instant,
}

#[async_trait]
impl Catalog for CachedCatalog {
    async fn list_tables(&self) -> Result<Vec<TableMetadata>> {
        let key = "tables".to_string();

        // 检查缓存
        {
            let mut cache = self.cache.lock().await;
            if let Some(entry) = cache.get(&key) {
                if entry.timestamp.elapsed() < self.ttl {
                    return Ok(entry.data.clone());
                }
            }
        }

        // 缓存未命中，查询底层
        let tables = self.inner.list_tables().await?;

        // 写入缓存
        {
            let mut cache = self.cache.lock().await;
            cache.put(key, CacheEntry {
                data: tables.clone(),
                timestamp: Instant::now(),
            });
        }

        Ok(tables)
    }
}
```

---

### 3.6 LSP Layer (crates/lsp/)

#### 职责

- 实现 LSP 服务器（Completion 核心）
- 管理多连接、多文档
- 增量解析和缓存

#### 核心数据结构

```rust
/// LSP 服务器后端
pub struct Backend {
    /// 文档缓存: Url → Document
    documents: Arc<RwLock<HashMap<Url, Document>>>,

    /// 引擎配置: Url → EngineConfig
    engines: Arc<RwLock<HashMap<Url, EngineConfig>>>,

    /// 工厂
    grammar_factory: Arc<GrammarFactory>,
    lowering_factory: Arc<LoweringFactory>,

    /// Catalog 管理器
    catalog_manager: Arc<CatalogManager>,
}

/// 文档状态
struct Document {
    text: Rope,
    tree: Tree,
    ir: Option<Arc<Stmt>>,
    semantic: Option<Arc<SemanticModel>>,
    version: i32,
}

/// 引擎配置
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub dialect: Dialect,
    pub connection_string: String,
    pub schema_filter: Option<SchemaFilter>,
}
```

#### Completion 实现

```rust
impl Backend {
    pub async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionList>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        // 1. 获取文档
        let doc = self.documents.read().await
            .get(&uri)
            .ok_or_else(|| anyhow!("Document not found"))?;

        // 2. 获取引擎配置
        let engine = self.engines.read().await
            .get(&uri)
            .ok_or_else(|| anyhow!("Engine config not found"))?;

        // 3. 获取 Grammar 和 Lowering
        let grammar = self.grammar_factory.get(engine.dialect);
        let lowering = self.lowering_factory.get(engine.dialect);

        // 4. 获取 Catalog
        let catalog = self.catalog_manager.get(&engine.connection_string).await?;

        // 5. 判断补全上下文
        let semantic_analyzer = SemanticAnalyzer::new(catalog, engine.dialect);
        let ctx = semantic_analyzer.get_completion_context(
            position.into(),
            &doc.tree,
        );

        // 6. 根据上下文生成补全项
        let items = match ctx {
            CompletionContext::SelectProjection => {
                self.complete_columns(&doc, &semantic_analyzer).await?
            }
            CompletionContext::FromTable => {
                self.complete_tables(&catalog).await?
            }
            CompletionContext::QualifiedColumn(table) => {
                self.complete_qualified_columns(&catalog, &table).await?
            }
            CompletionContext::FunctionCall => {
                self.complete_functions(&catalog).await?
            }
            _ => Vec::new(),
        };

        Ok(Some(CompletionList {
            is_incomplete: false,
            items,
        }))
    }

    /// 补全列（SELECT 子句）
    async fn complete_columns(
        &self,
        doc: &Document,
        analyzer: &SemanticAnalyzer,
    ) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // 从语义模型获取可见的列
        if let Some(semantic) = &doc.semantic {
            for scope in &semantic.scopes {
                for table in &scope.tables {
                    for column in &table.columns {
                        items.push(CompletionItem {
                            label: column.name.clone(),
                            kind: CompletionItemKind::FIELD,
                            detail: Some(column.data_type.clone().to_string()),
                            documentation: column.comment.clone().map(|c| {
                                Documentation::MarkupContent(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: c,
                                })
                            }),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        Ok(items)
    }

    /// 补全表（FROM 子句）
    async fn complete_tables(
        &self,
        catalog: &dyn Catalog,
    ) -> Result<Vec<CompletionItem>> {
        let tables = catalog.list_tables().await?;
        let mut items = Vec::new();

        for table in tables {
            items.push(CompletionItem {
                label: table.name.clone(),
                kind: if table.is_view {
                    CompletionItemKind::INTERFACE
                } else {
                    CompletionItemKind::STRUCT
                },
                detail: Some(format!("{} columns", table.columns.len())),
                ..Default::default()
            });
        }

        Ok(items)
    }
}
```

#### 文档同步与增量解析

```rust
#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = TextDocumentItem {
            uri: params.text_document.uri.clone(),
            language_id: params.text_document.language_id,
            version: params.text_document.version,
            text: params.text_document.text,
        };

        self.on_open(doc).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let changes = params.content_changes;

        self.on_change(uri, changes).await;
    }
}

impl Backend {
    async fn on_open(&self, doc: TextDocumentItem) {
        let uri = doc.uri.clone();

        // 1. 构建初始文档
        let rope = Rope::from_str(&doc.text);
        let tree = self.parse_initial(&doc.text, &uri).await;

        // 2. 存储文档
        let document = Document {
            text: rope,
            tree,
            ir: None,
            semantic: None,
            version: doc.version,
        };

        self.documents.write().await.insert(uri, document);
    }

    async fn on_change(&self, uri: Url, changes: Vec<TextDocumentContentChangeEvent>) {
        let mut docs = self.documents.write().await;
        let doc = docs.get_mut(&uri).expect("Document not found");

        // 1. 应用文本更改
        for change in changes {
            let range = change.range.map(|r| {
                let start = self.lsp_pos_to_point(r.start);
                let end = self.lsp_pos_to_point(r.end);
                (start, end)
            });

            // 2. 增量解析
            if let Some((start, end)) = range {
                doc.text.remove(start..end);
                doc.text.insert(start, &change.text);
                doc.tree.edit(&Edit {
                    start_byte: start,
                    old_end_byte: end,
                    new_end_byte: start + change.text.len(),
                    start_position: self.byte_to_point(start),
                    old_end_position: self.byte_to_point(end),
                    new_end_position: self.byte_to_point(start + change.text.len()),
                });
            }
        }

        // 3. 重新解析
        let new_text = doc.text.to_string();
        let engine = self.engines.read().await.get(&uri).unwrap();
        let grammar = self.grammar_factory.get(engine.dialect);

        doc.tree = grammar.parse(&new_text);

        // 4. 触发 IR 和 Semantic 更新（后台任务）
        // 使用 work queue 或 channel 异步处理
        self.schedule_semantic_update(uri, new_text).await;
    }
}
```

#### 多连接管理

```rust
impl Backend {
    /// 添加文档（新连接）
    pub async fn add_document(
        &self,
        uri: Url,
        text: String,
        engine: EngineConfig,
    ) -> Result<()> {
        // 1. 存储引擎配置
        self.engines.write().await.insert(uri.clone(), engine);

        // 2. 初始化 Catalog（如果不存在）
        self.catalog_manager.get_or_create(&engine.connection_string, engine.dialect).await?;

        // 3. 解析文档
        let grammar = self.grammar_factory.get(engine.dialect);
        let tree = grammar.parse(&text);

        // 4. 存储文档
        let document = Document {
            text: Rope::from_str(&text),
            tree,
            ir: None,
            semantic: None,
            version: 0,
        };

        self.documents.write().await.insert(uri, document);

        Ok(())
    }

    /// 移除文档（断开连接）
    pub async fn remove_document(&self, uri: &Url) {
        self.documents.write().await.remove(uri);
        self.engines.write().await.remove(uri);

        // 如果没有其他连接使用，可以考虑清理 Catalog
        self.catalog_manager.maybe_gc().await;
    }
}
```

---

## 四、多引擎支持策略

### 4.1 引擎抽象

```rust
/// 引擎标识
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EngineId {
    pub dialect: Dialect,
    pub version: Option<(u8, u8)>,
}

impl FromStr for EngineId {
    type Err = ParseEngineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // "mysql:8.0" → EngineId { dialect: MySQL(8, 0), version: Some((8, 0)) }
        // "postgres" → EngineId { dialect: PostgreSQL(0, 0), version: None }
        // "sqlite" → EngineId { dialect: SQLite, version: None }
    }
}
```

### 4.2 引擎注册

```rust
/// 引擎注册表
pub struct EngineRegistry {
    engines: DashMap<String, EngineConfig>,
}

impl EngineRegistry {
    /// 注册引擎
    pub fn register(&self, name: String, config: EngineConfig) {
        self.engines.insert(name, config);
    }

    /// 批量注册（从配置文件）
    pub fn register_from_config(&self, path: &Path) -> Result<()> {
        let config: EngineConfigFile = serde_yaml::from_reader(File::open(path)?)?;

        for (name, engine) in config.engines {
            self.register(name, engine);
        }

        Ok(())
    }
}
```

### 4.3 版本兼容

```rust
/// 特性支持查询
pub trait FeatureSupport {
    fn supports(&self, feature: Feature) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    WindowFunctions,
    CTEs,
    LateralJoins,
    JsonFunctions,
    FullOuterJoin,
    ArrayFunctions,
}

impl FeatureSupport for Dialect {
    fn supports(&self, feature: Feature) -> bool {
        match self {
            Dialect::MySQL { version } => match feature {
                Feature::WindowFunctions => *version >= (8, 0),
                Feature::CTEs => *version >= (8, 0),
                Feature::FullOuterJoin => false,
                _ => false,
            },
            Dialect::PostgreSQL { version } => match feature {
                Feature::WindowFunctions => *version >= (8, 4),
                Feature::CTEs => *version >= (8, 4),
                Feature::LateralJoins => *version >= (9, 3),
                _ => true,
            },
            Dialect::SQLite => match feature {
                Feature::WindowFunctions => true,  // SQLite 3.25+
                Feature::CTEs => true,             // SQLite 3.8.3+
                Feature::FullOuterJoin => false,
                _ => false,
            },
        }
    }
}
```

---

## 五、性能优化

### 5.1 增量解析

Tree-sitter 提供内置的增量解析：

```rust
// 文档更新时
tree.edit(&Edit {
    start_byte: old_range.start,
    old_end_byte: old_range.end,
    new_end_byte: new_range.end,
    start_position: old_start_pos,
    old_end_position: old_end_pos,
    new_end_position: new_end_pos,
});

// 重新解析
let new_tree = parser.parse(&new_text, Some(&tree));
```

### 5.2 缓存策略

**三级缓存**：

```rust
pub struct CacheManager {
    /// Tree-sitter Tree 缓存
    tree_cache: Arc<DashMap<Url, Arc<Tree>>>,

    /// IR 缓存
    ir_cache: Arc<DashMap<Url, Arc<ArcSwapAny<Arc<Stmt>>>>>,

    /// Semantic 缓存
    semantic_cache: Arc<DashMap<Url, Arc<ArcSwapAny<Arc<SemanticModel>>>>>,
}
```

**缓存失效策略**：

1. **文本更改** → 失效 Tree
2. **Tree 更新** → 失效 IR（受影响的语句）
3. **IR 更新** → 失效 Semantic（受影响的作用域）

**细粒度失效**：

```rust
/// 根据文本更改范围，判断需要重新解析的语句
fn affected_statements(change_range: Range, stmts: &[Stmt]) -> Vec<usize> {
    stmts.iter()
        .enumerate()
        .filter(|(_, stmt)| stmt.span().intersects(&change_range))
        .map(|(i, _)| i)
        .collect()
}
```

### 5.3 并发处理

```rust
/// 后台任务：语义分析
pub struct SemanticWorker {
    receiver: mpsc::Receiver<SemanticJob>,
    catalog_manager: Arc<CatalogManager>,
    lowering_factory: Arc<LoweringFactory>,
}

impl SemanticWorker {
    pub async fn run(mut self) {
        while let Some(job) = self.receiver.recv().await {
            // 异步处理，不阻塞 LSP 主线程
            let result = self.process_job(job).await;

            // 更新缓存
            self.update_cache(result);
        }
    }
}
```

### 5.4 Catalog 查询优化

**批量查询**：

```rust
/// 一次性获取多个表的列信息
#[async_trait]
pub trait Catalog: Send + Sync {
    async fn get_columns_batch(&self, tables: Vec<String>) -> Result<HashMap<String, Vec<ColumnMetadata>>> {
        // 使用 IN 查询或批量请求
        // 避免多次网络往返
    }
}
```

**预加载**：

```rust
/// 在用户输入 "FROM " 时，预加载表列表
pub async fn preload_tables(&self, uri: Url) {
    if let Some(engine) = self.engines.read().await.get(&uri) {
        let catalog = self.catalog_manager.get_or_create(&engine.connection_string, engine.dialect).await;

        // 后台预加载
        tokio::spawn(async move {
            let _ = catalog.list_tables().await;
        });
    }
}
```

---

## 六、Schema 过滤策略

### 6.1 用户权限过滤

```rust
#[derive(Debug, Clone)]
pub struct SchemaFilter {
    /// 可见的 schemas
    pub allowed_schemas: Option<Vec<String>>,

    /// 可见的 tables（glob 模式）
    pub allowed_tables: Option<Vec<String>>,

    /// 排除的 tables
    pub excluded_tables: Option<Vec<String>>,
}

impl SchemaFilter {
    pub fn allows_table(&self, schema: &str, table: &str) -> bool {
        // 1. 检查 schema
        if let Some(allowed) = &self.allowed_schemas {
            if !allowed.contains(&schema.to_string()) {
                return false;
            }
        }

        // 2. 检查表排除
        if let Some(excluded) = &self.excluded_tables {
            if excluded.iter().any(|p| self.matches(p, table)) {
                return false;
            }
        }

        // 3. 检查表允许
        if let Some(allowed) = &self.allowed_tables {
            allowed.iter().any(|p| self.matches(p, table))
        } else {
            true
        }
    }

    fn matches(&self, pattern: &str, text: &str) -> bool {
        // 支持 glob 模式: "users_*", "temp.*"
        // 使用 glob_match crate
    }
}
```

### 6.2 Catalog 层集成

```rust
pub struct FilteredCatalog {
    inner: Arc<dyn Catalog>,
    filter: SchemaFilter,
}

#[async_trait]
impl Catalog for FilteredCatalog {
    async fn list_tables(&self) -> Result<Vec<TableMetadata>> {
        let all_tables = self.inner.list_tables().await?;

        Ok(all_tables.into_iter()
            .filter(|t| self.filter.allows_table(
                t.schema.as_deref().unwrap_or("public"),
                &t.name,
            ))
            .collect())
    }
}
```

---

## 七、错误处理与诊断

### 7.1 诊断类型

```rust
#[derive(Debug, Clone)]
pub enum DiagnosticKind {
    /// 语法错误
    SyntaxError,

    /// 未定义的表
    UndefinedTable { name: String },

    /// 未定义的列
    UndefinedColumn { name: String, candidates: Vec<String> },

    /// 歧义列
    AmbiguousColumn { name: String, tables: Vec<String> },

    /// 类型不匹配（可选）
    TypeMismatch { expected: String, found: String },
}
```

### 7.2 诊断生成

```rust
impl SemanticAnalyzer {
    pub fn diagnose(&self, stmt: &Stmt) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        match stmt {
            Stmt::Query(query) => {
                self.diagnose_query(query, &mut diagnostics);
            }
            _ => { /* ... */ }
        }

        diagnostics
    }

    fn diagnose_query(&self, query: &Query, diagnostics: &mut Vec<Diagnostic>) {
        let Query { body, .. } = query;

        match body {
            SetExpr::Select(select) => {
                // 1. 检查表是否存在
                for table in &select.from {
                    if !self.catalog.table_exists(&table.name.to_string()).await {
                        diagnostics.push(Diagnostic {
                            range: table.name.span,
                            severity: DiagnosticSeverity::ERROR,
                            message: format!("Table '{}' does not exist", table.name),
                            ..Default::default()
                        });
                    }
                }

                // 2. 检查列引用
                self.diagnose_columns(select, diagnostics);
            }
            _ => { /* ... */ }
        }
    }
}
```

---

## 八、实施路线图

### Phase 1: 基础设施（2-3 周）

- [ ] **Grammar Layer**
  - [ ] 选择/设计 Tree-sitter SQL Grammar（建议 fork tree-sitter-sql）
  - [ ] 实现 `Grammar` trait
  - [ ] 实现 `GrammarFactory`（支持 MySQL, PostgreSQL, SQLite）

- [ ] **IR Layer**（已有基础）
  - [ ] 扩展 `SelectStmt` 支持 Window 子句
  - [ ] 添加 `DialectExtensions`

### Phase 2: 核心功能（3-4 周）

- [ ] **Lowering Layer**
  - [ ] 实现 `Lowering` trait
  - [ ] 实现 MySQL/PostgreSQL/SQLite 的 Lowering
  - [ ] 单元测试覆盖

- [ ] **Semantic Layer**
  - [ ] 实现 `Scope`, `TableSymbol`, `ColumnSymbol`
  - [ ] 实现 `SemanticAnalyzer`
  - [ ] 实现 `get_completion_context`

- [ ] **Catalog Layer**
  - [ ] 定义 `Catalog` trait
  - [ ] 实现 `LiveCatalog`（动态连接）
  - [ ] 实现 `CachedCatalog`

### Phase 3: LSP 集成（2-3 周）

- [ ] **LSP Server**
  - [ ] 实现 `Backend` 结构
  - [ ] 实现 `completion` 处理
  - [ ] 实现文档同步 (`did_open`, `did_change`)
  - [ ] 增量解析与缓存

### Phase 4: 多引擎支持（持续）

- [ ] **引擎扩展**
  - [ ] 添加更多引擎（10+）
  - [ ] 版本特性支持
  - [ ] 引擎配置文件格式

### Phase 5: 优化与扩展（持续）

- [ ] **性能优化**
  - [ ] 并发语义分析
  - [ ] Catalog 预加载
  - [ ] 性能测试

- [ ] **功能扩展**
  - [ ] Hover 支持
  - [ ] 诊断支持
  - [ ] Signature Help

---

## 九、测试策略

### 9.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mysql_lowering() {
        let sql = "SELECT id, name FROM users WHERE id = 1";
        let grammar = MySQLGrammar::new((8, 0));
        let tree = grammar.parse(sql);

        let lowering = MySQLLowering::new((8, 0));
        let stmt = lowering.lower_select(tree.root_node()).unwrap();

        assert_eq!(stmt.projections.len(), 2);
        assert_eq!(stmt.from.len(), 1);
    }

    #[tokio::test]
    async fn test_completion_context() {
        let sql = "SELECT | FROM users";
        let analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL { version: (8, 0) });

        let ctx = analyzer.get_completion_context(Position::new(0, 8), &tree);

        assert_eq!(ctx, CompletionContext::SelectProjection);
    }
}
```

### 9.2 集成测试

```rust
#[tokio::test]
async fn test_completion_flow() {
    let backend = Backend::new(...);

    // 1. 打开文档
    backend.add_document(
        Url::parse("file://test.sql").unwrap(),
        "SELECT | FROM users".to_string(),
        EngineConfig { ... },
    ).await;

    // 2. 触发补全
    let result = backend.completion(CompletionParams {
        text_document_position: TextDocumentPosition {
            text_document: TextDocumentIdentifier { uri: Url::parse("file://test.sql").unwrap() },
            position: Position::new(0, 8),
        },
        ..Default::default()
    }).await;

    // 3. 验证结果
    assert!(result.is_some());
    let items = result.unwrap().items;
    assert!(!items.is_empty());
}
```

### 9.3 性能测试

```rust
#[tokio::test]
async fn bench_large_file_parsing() {
    let sql = generate_large_sql(10_000);  // 10k lines
    let grammar = MySQLGrammar::new((8, 0));

    let start = Instant::now();
    let tree = grammar.parse(&sql);
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_millis(100));  // < 100ms
}
```

---

## 十、配置示例

### 10.1 引擎配置文件（YAML）

```yaml
# config/engines.yaml

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
    schema_filter:
      allowed_schemas: ["public", "audit"]

  sqlite-local:
    dialect: sqlite
    connection_string: "file:///path/to/local.db"
```

### 10.2 LSP 客户端配置

```json
{
  "languages": {
    "sql": {
      "lsp": {
        "command": "/usr/local/bin/unified-sql-lsp",
        "args": ["--config", "/path/to/config/engines.yaml"]
      }
    }
  }
}
```

---

## 十一、关键决策记录

### 11.1 为什么选择 Tree-sitter？

**优势**：
- ✅ 增量解析：适合编辑器场景
- ✅ 错误恢复：即使语法错误也能继续解析
- ✅ 多语言支持：Rust 生态成熟
- ✅ 可组合性：支持 Grammar 继承和组合

**替代方案**：
- ❌ sqlparser-rs：不支持增量解析，单一 Grammar
- ❌ 手写 Parser：开发成本高，维护困难

### 11.2 为什么需要 IR 层？

**原因**：
1. **方言隔离**：LSP 逻辑不需要处理方言差异
2. **可测试性**：IR 可以独立测试
3. **可扩展性**：新增方言只需实现 Lowering

### 11.3 为什么不支持 Jump Definition？

**原因**：
- SQL 的"定义"概念模糊（表别名、列引用）
- 用户需求不明确（Hover 可能更有价值）
- 实现复杂度高（需要跨文件分析）
- 优先级低（Completion 是核心）

### 11.4 为什么使用 DashMap？

**优势**：
- ✅ 并发安全：支持多线程读写
- ✅ 高性能：分片锁，比 Mutex<RwLock<HashMap>> 快
- ✅ API 友好：类似 HashMap

---

## 十二、风险与挑战

### 12.1 性能风险

**风险**：大型 SQL 文件解析慢，Catalog 查询延迟

**缓解**：
- 增量解析 + 细粒度缓存
- Catalog LRU 缓存 + TTL
- 后台异步语义分析
- 性能测试基准

### 12.2 方言兼容性

**风险**：某些方言语法差异巨大，IR 难以统一

**缓解**：
- 使用 `DialectExtensions` 保留方言特定信息
- IR 只覆盖核心 SQL（DDL/DML 可能方言特定）
- 文档明确支持的特性范围

### 12.3 扩展性挑战

**风险**：支持 30+ 引擎，Grammar 和 Lowering 开发工作量大

**缓解**：
- 优先支持主流引擎（MySQL, PG, SQLite）
- 社区贡献：提供清晰的方言扩展指南
- 代码生成：工具辅助生成 Grammar 模板

---

## 十三、后续扩展

### 13.1 高级功能

- **Hover**: 显示表/列的元数据
- **Diagnostics**: 实时错误检测（未定义表、歧义列）
- **Signature Help**: 函数参数提示
- **Code Actions**: 快速修复（自动导入表等）
- **Format**: SQL 格式化（基于 sqlformatter-rs）

### 13.2 企业级特性

- **Schema Cache 持久化**: 避免重复查询数据库
- **多租户隔离**: 不同项目使用不同的 Schema
- **审计日志**: 记录补全触发和 Catalog 查询
- **Metrics**: Prometheus 集成，监控性能

---

## 附录

### A. 参考资源

- [Tree-sitter 官方文档](https://tree-sitter.github.io/tree-sitter/)
- [tree-sitter-sql Grammar](https://github.com/m-novikov/tree-sitter-sql)
- [LSP 规范](https://microsoft.github.io/language-server-protocol/)
- [sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs)

### B. 相关项目

- [sqlfluff](https://github.com/sqlfluff/sqlfluff): SQL Linter（Python）
- [jedi-language-server](https://github.com/pappasam/jedi-language-server): Python LSP（参考架构）

### C. 贡献指南

（待补充：方言扩展指南、Grammar 编写指南、测试指南）

---

**文档版本**: v1.0
**最后更新**: 2025-12-31
**维护者**: unified-sql-lsp team
