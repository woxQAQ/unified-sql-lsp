# SQL LSP 已知限制 (Known Limitations)

本文档记录了统一 SQL LSP 服务器在实现过程中发现的已知限制、原因和变通方案。

## 目录

1. [CTE 补全限制](#cte-补全限制)
2. [子查询别名支持](#子查询别名支持)
3. [多表 JOIN 表提取](#多表-join-表提取)
4. [CST 语法错误节点](#cst-语法错误节点)

---

## CTE 补全限制

### 描述

当用户在主查询中引用 CTE 名称时，代码补全不显示该 CTE 名称。

### 示例

```sql
WITH user_cte AS (SELECT * FROM users)
SELECT | FROM user_cte
-- 期望: 补全列表中应包含 "user_cte"
-- 实际: 补全列表中不包含 "user_cte"
```

### 根本原因

**文本表提取器无法识别 FROM 子句中的 CTE 名称**

在 `crates/context/src/completion.rs` 中，`extract_tables_from_from_clause()` 函数使用 CST（具体语法树）和文本回退机制来提取表名。但是：

1. **CST 提取失败**：对于包含 CTE 的 SQL，tree-sitter CST 可能在某些情况下无法正确解析 FROM 子句
2. **文本回退不完整**：当 CST 提取返回空结果时，文本回退机制（`extract_tables_from_source()`）使用正则表达式提取表名，但无法区分 CTE 名称和普通表名

### 技术细节

```rust
// crates/context/src/completion.rs:extract_tables_from_from_clause()
// 该函数依赖于 CST 解析，但对于某些 SQL 模式可能失败
fn extract_tables_from_from_clause(select_node: &Node, source: &str) -> Vec<String> {
    // CST 解析逻辑...
    // 如果失败，回退到文本提取
    if tables.is_empty() {
        tables = extract_tables_from_source(source);
    }
}
```

### 变通方案

**无直接变通方案** - 这是架构性限制，需要以下任一改进：

1. **增强 CST 解析器**：改进 tree-sitter 语法以更好地处理 CTE
2. **文本模式改进**：在文本提取器中添加 CTE 识别模式
3. **完整语义分析**：实现完整的 SQL 语义分析器（SEMANTIC-002）

### 影响范围

- **影响的数据库**：所有支持的数据源（MySQL, PostgreSQL, TiDB, MariaDB, CockroachDB）
- **影响的测试**：`tests/e2e-rs/tests/mysql-8.0/completion/cte.yaml` 中有相关测试用例
- **回归风险**：中等 - 这是已存在的限制，不是本次实现引入

---

## 子查询别名支持

### 描述

当使用子查询作为表源并带有别名时，代码补全可能无法识别子查询的别名。

### 示例

```sql
SELECT u.id
FROM (SELECT id, name FROM users) AS u
JOIN orders AS o ON u.id = o.user_id
```

期望：应该能够补全 `u.*` 和 `o.*`
实际：可能无法识别 `u` 作为表别名

### 根本原因

**Tree-sitter 语法为子查询创建 ERROR 节点**

通过调试发现，tree-sitter 语法无法正确解析带有别名的子查询，导致 CST 中出现 ERROR 节点：

```
[ERROR]: ") AS u"
    [)]: ")"
    [AS]: "AS"
    [ERROR]: "u"
```

这说明 grammar.js 语法文件中缺少对 `(SELECT ...) AS alias` 模式的完整支持。

### 技术细节

**CST 结构分析**：
```sql
SELECT * FROM (SELECT id, name FROM users) AS u
```

生成的 CST 节点：
- 主 SELECT 语句被解析为多个独立的 `statement` 节点
- 子查询 `(SELECT id, name FROM users)` 是一个独立的 `select_statement`
- 别名部分 `) AS u` 被标记为 ERROR 节点
- 别名 "u" 无法被正确关联到子查询

### 受影响的代码路径

1. **CST 路径**：`crates/context/src/scope_builder.rs`
   - `build_from_select()` 无法找到正确的 FROM 子句
   - `extract_table_references()` 无法识别子查询别名

2. **文本回退路径**：`crates/lsp/src/completion/mod.rs`
   - 依赖 CST 失败后使用文本提取
   - 文本提取器也无法正确解析子查询别名

### 变通方案

**方案 1：使用文本模式检测子查询**
```sql
-- 当前不支持的模式
SELECT * FROM (SELECT id FROM users) AS u

-- 变通：不使用别名
SELECT * FROM (SELECT id FROM users)
```

**方案 2：使用 CTE 替代子查询**
```sql
-- CTE 方式（更好的可读性）
WITH user_cte AS (SELECT id FROM users)
SELECT * FROM user_cte AS u
```

### 实现路径

要完全支持子查询别名，需要修改 tree-sitter 语法：

1. **文件位置**：`crates/grammar/src/dialect/mysql.js` 或 `grammar.js`
2. **需要添加的规则**：支持 `derived_table` 或 `subquery` 节点类型
3. **复杂度**：中等 - 需要了解 tree-sitter 语法定义
4. **风险**：语法修改可能影响其他解析路径

### 测试覆盖

已添加测试验证此行为：
- `crates/context/src/scope_builder.rs:543-565` - `test_extract_subquery_with_alias()`
- 测试验证 CST 无法处理子查询，返回 `NoFromClause` 错误（符合预期）

---

## 多表 JOIN 表提取

### 描述

当 FROM 子句包含多个 JOIN 时，文本回退提取可能无法找到所有被 JOIN 的表。

### 示例

```sql
SELECT u., o., oi.
FROM users AS u
JOIN orders AS o ON u.id = o.user_id
JOIN order_items AS oi ON o.id = oi.order_id
```

期望：`context_tables` 应包含 `["users", "u", "orders", "o", "order_items", "oi"]`
实际：只提取到 `["users", "u", "orders", "o"]`，缺少 `"order_items"` 和 `"oi"`

### 根本原因

**文本表提取器的正则表达式模式不完整**

在 `crates/context/src/completion.rs` 中，`extract_tables_from_source()` 使用正则表达式提取表名，但：

1. **模式只匹配前两个表**：正则可能设计为只处理 `t1 JOIN t2` 的情况
2. **复杂的 JOIN 链**：对于 `t1 JOIN t2 JOIN t3 JOIN t4`，提取可能在某个点停止
3. **关键字检测不完整**：可能无法正确识别所有 JOIN 关键字

### 技术细节

```rust
// crates/context/src/completion.rs:extract_tables_from_source()
// 文本提取使用正则表达式模式匹配表名
fn extract_tables_from_source(source: &str) -> Vec<String> {
    // 正则模式可能只匹配:
    // - FROM table
    // - FROM table JOIN table
    // 但不匹配:
    // - FROM table1 JOIN table2 JOIN table3
}
```

### 影响的 E2E 测试

**测试文件**：`tests/e2e-rs/tests/mysql-8.0/completion/join_aliases.yaml`

**失败的测试用例**：
```yaml
- name: "multiple joins"
  description: "Should handle three-way JOINs with different aliases"
  sql: "SELECT u., o., oi.| FROM users AS u JOIN orders AS o ON u.id = o.user_id JOIN order_items AS oi ON o.id = oi.order_id"
  expect_completion:
    contains:
      - "oi.id"
      - "oi.quantity"
```

### 变通方案

**方案 1：使用完全限定的列名**
```sql
-- 避免 JOIN 别名补全问题
SELECT users.id, orders.total_amount, order_items.quantity
FROM users
JOIN orders ON users.id = orders.user_id
JOIN order_items ON orders.id = order_items.order_id
```

**方案 2：分段构建查询**
```sql
-- 先测试两个 JOIN，再添加第三个
-- 确保每个 JOIN 都能正常工作后再扩展
```

### 实现路径

要完全支持多表 JOIN：

1. **改进正则模式**：增强 `extract_tables_from_source()` 中的正则表达式
2. **迭代提取**：循环查找所有 JOIN 关键字并提取后续表名
3. **使用 CST 路径**：优先使用 CST 解析，文本提取只作为最后手段

---

## CST 语法错误节点

### 描述

Tree-sitter CST 解析器在遇到某些复杂 SQL 语法时，会创建 ERROR 节点而不是正确的语法节点。

### 受影响的 SQL 模式

1. **子查询别名**：`(SELECT ...) AS alias`
2. **某些 CTE 模式**：复杂的 WITH 子句
3. **嵌套表达式**：深层的嵌套结构

### 示例

```sql
-- 产生 ERROR 节点的模式
SELECT * FROM (SELECT id FROM users) AS u
```

生成的 CST：
```
[ERROR]: ") AS u"
  [)]: ")"
```

### 根本原因

**Tree-sitter 语法文件不完整**

- **位置**：`crates/grammar/src/dialect/mysql.js` 和 `grammar.js`
- **问题**：语法规则没有涵盖所有合法的 SQL 语法
- **优先级**：低 - 语法是分阶段实现的，某些复杂模式可能尚未添加

### 变通方案

**使用更简单的语法模式**：
```sql
-- 避免使用容易产生 ERROR 节点的语法
-- 使用 CTE 替代子查询
WITH user_data AS (SELECT id FROM users)
SELECT * FROM user_data
```

---

## 总结对比表

| 限制 | 严重程度 | 影响范围 | 变通方案 | 修复优先级 |
|------|----------|----------|----------|-----------|
| CTE 补全 | 中 | 所有数据库 | 无直接方案 | 中 |
| 子查询别名 | 高 | 所有数据库 | 使用 CTE 替代 | 高 |
| 多表 JOIN | 中 | 所有数据库 | 使用完整表名 | 低 |
| CST ERROR 节点 | 低 | 特定语法 | 简化语法 | 低 |

---

## 相关文件

- CST 范围构建器：`crates/context/src/scope_builder.rs`
- 补全上下文：`crates/lsp/src/completion/mod.rs`
- Tree-sitter 语法：`crates/grammar/src/`

## 测试覆盖

每个限制都有对应的测试用例：

1. **CTE 限制**：`crates/context/src/scope_builder.rs:543-565`
2. **子查询限制**：同上
3. **JOIN 限制**：`tests/e2e-rs/tests/mysql-8.0/completion/join_aliases.yaml`（multiple joins 测试）

---

## 更新日志

- **2025-01-19**: 初始版本，记录 table alias enhancement 实现过程中发现的 4 个主要限制
