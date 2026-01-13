# Unified SQL LSP - 语法限制报告

## 概述

本文档记录了当前 tree-sitter SQL 语法的已知限制和待改进项。

**生成日期**: 2025-01-14
**相关测试**: `crates/grammar/tests/api_tests.rs`

---

## 1. 表别名与 JOIN 处理限制

### 问题描述
当前语法在解析包含表别名和 JOIN 的复杂查询时存在解析错误。

### 失败测试用例
```rust
#[test]
#[ignore = "TODO: Grammar needs improvement for table aliases and JOIN handling"]
fn test_parse_complex_query() {
    // 失败的 SQL 示例
    let source = r#"
        SELECT u.id, u.name, COUNT(o.id) AS order_count
        FROM users AS u
        LEFT JOIN orders AS o ON u.id = o.user_id
        WHERE u.created_at > '2024-01-01'
        GROUP BY u.id, u.name
        HAVING COUNT(o.id) > 5
        ORDER BY order_count DESC
        LIMIT 10
    "#;
}
```

### 根本原因分析

#### 1.1 table_reference 规则的歧义性

**当前实现** (`grammar.js:271-275`):
```javascript
table_reference: $ => choice(
  seq($.table_name, /[Aa][Ss]/, $.alias),
  $.table_name,
  $.join_clause
),
```

**问题**:
- `table_name` 和 `alias` 都被定义为 `identifier`
- 当 tree-sitter 尝试匹配 `seq($.table_name, $.alias)` 时，会产生歧义
- 解析器无法确定 `users AS u` 中的 `u` 是别名还是下一个 token

**错误表现**:
```
(from_clause
  (ERROR (table_reference (table_name (identifier)) (alias (identifier))))
  (table_reference (join_clause ...))
)
```

第一个 `table_reference` 被标记为 ERROR，尽管结构看起来是正确的。

#### 1.2 join_clause 的限制

**当前实现** (`grammar.js:277-284`):
```javascript
join_clause: $ => seq(
  optional($.join_type),
  /[Jj][Oo][Ii][Nn]/,
  $.table_name,
  optional(seq(/[Aa][Ss]/, $.alias)),
  /[Oo][Nn]/,
  $.expression
),
```

**限制**:
- JOIN 的右侧表别名处理不一致
- 当 `FROM` 子句有多个 `table_reference` 时，JOIN 子句的解析会产生冲突
- 无法正确处理嵌套 JOIN 或复杂 JOIN 链

### 解决方案选项

#### 选项 1: 使用字段标注区分表名和别名
```javascript
table_reference: $ => choice(
  field('aliased', seq($.table_name, /[Aa][Ss]/, $.alias)),
  field('unaliased', $.table_name),
  $.join_clause
),
```

#### 选项 2: 分离表引用规则
```javascript
table_reference: $ => choice(
  $.aliased_table,
  $.unaliased_table,
  $.join_clause
),

aliased_table: $ => seq($.table_name, /[Aa][Ss]/, $.alias),
unaliased_table: $ => $.table_name,
```

#### 选项 3: 使用 prec 动态优先级
```javascript
table_reference: $ => choice(
  prec(2, seq($.table_name, /[Aa][Ss]/, $.alias)),
  prec(1, $.table_name),
  $.join_clause
),
```

**注意**: 选项 3 已尝试但未解决问题，说明问题更深层。

### 影响范围

| 功能 | 状态 | 说明 |
|------|------|------|
| 单表查询 | ✅ 正常 | `SELECT * FROM users` |
| 表别名 (无 AS) | ❌ 失败 | `FROM users u` 产生 ERROR |
| 表别名 (显式 AS) | ⚠️ 部分工作 | `FROM users AS u` 在 FROM 子句失败 |
| INNER JOIN | ⚠️ 部分工作 | JOIN 部分解析成功，FROM 子句部分失败 |
| LEFT/RIGHT JOIN | ⚠️ 部分工作 | 同上 |
| 多表 JOIN | ❌ 失败 | 无法正确解析多个 table_reference |

---

## 2. 多语句解析限制

### 问题描述
语法无法正确解析多个用分号分隔的 SQL 语句。

### 失败测试用例
```rust
#[test]
#[ignore = "TODO: Grammar needs improvement for multiple statement parsing"]
fn test_parse_multiple_statements() {
    let source = "CREATE TABLE users (id INT); INSERT INTO users VALUES (1); SELECT * FROM users;";
    // 解析结果: has_error() == true
}
```

### 根本原因分析

#### 2.1 缺少分号处理

**当前 source_file 规则** (`grammar.js:83-85`):
```javascript
source_file: $ => repeat($._statement),
_statement: $ => $.statement,
```

**问题**:
- 没有对语句分隔符（分号）进行显式处理
- `repeat($._statement)` 期望语句之间没有分隔符，或分隔符由语句本身处理
- 当前 `statement` 规则不包含尾随的分号

#### 2.2 CREATE TABLE 语句的不完整支持

虽然添加了 `create_table_statement` 规则，但可能存在以下问题：
- 列定义可能缺少约束支持
- 多列定义的分隔处理
- 表创建选项缺失

### 解决方案

#### 方案 1: 在 source_file 中处理分号
```javascript
source_file: $ => seq(
  $._statement,
  repeat(seq(';', $._statement)),
  optional(';')
),
```

#### 方案 2: 在 statement 规则中包含分号
```javascript
_statement: $ => seq($.statement, optional(';')),
```

#### 方案 3: 创建专门的语句列表规则
```javascript
source_file: $ => repeat(seq($._statement, optional(';'))),
```

### 影响范围

| 功能 | 状态 | 说明 |
|------|------|------|
| 单语句解析 | ✅ 正常 | 单个 SELECT/INSERT/UPDATE/DELETE |
| 分号结尾 | ⚠️ 未明确 | 语句末尾的分号不消耗 |
| 多语句 | ❌ 失败 | 无法正确解析多个语句 |
| 脚本文件 | ❌ 失败 | SQL 脚本无法完整解析 |

---

## 3. 列别名解析的已知问题

### 已修复的问题
列别名的解析已经通过以下改进得到修复：

#### 3.1 修复内容

**之前**:
```javascript
projection: $ => choice(
  '*',
  seq($.expression, repeat(seq(',', $.expression)))
),
```

**之后** (`grammar.js:122-131`):
```javascript
_projection_item: $ => choice(
  seq($.expression, /[Aa][Ss]/, $.alias),
  seq($.expression, $.alias),
  $.expression
),

projection: $ => choice(
  '*',
  seq($._projection_item, repeat(seq(',', $._projection_item)))
),
```

#### 3.2 案例大小写不敏感

SQL 关键字现在使用正则表达式支持大小写不敏感：
- `'AS'` → `/[Aa][Ss]/`
- `'JOIN'` → `/[Jj][Oo][Ii][Nn]/`
- `'ON'` → `/[Oo][Nn]/`

### 当前支持状态

| 功能 | 状态 | 示例 |
|------|------|------|
| 列别名 (显式 AS) | ✅ 正常 | `COUNT(id) AS total` |
| 列别名 (隐式) | ✅ 正常 | `COUNT(id) total` |
| 无别名 | ✅ 正常 | `SELECT id, name` |
| 大小写不敏感 | ✅ 正常 | `as`, `As`, `AS` 都支持 |

---

## 4. CREATE TABLE 语句支持

### 已实现功能

#### 4.1 新增规则 (`grammar.js:205-236`)

```javascript
create_table_statement: $ => seq(
  'CREATE',
  'TABLE',
  $.table_name,
  '(',
  $.column_definition,
  repeat(seq(',', $.column_definition)),
  optional(','),
  ')'
),

column_definition: $ => seq(
  $.column_name,
  $.data_type
),

data_type: $ => choice(
  // 整数类型
  'INT', 'INTEGER', 'TINYINT', 'SMALLINT', 'MEDIUMINT', 'BIGINT',
  // 字符串类型
  'CHAR', 'VARCHAR', 'TEXT', 'TINYTEXT', 'MEDIUMTEXT', 'LONGTEXT',
  // 布尔类型
  'BOOLEAN', 'BOOL',
  // 小数类型
  'DECIMAL', 'NUMERIC', 'FLOAT', 'DOUBLE', 'REAL',
  // 日期/时间类型
  'DATE', 'TIME', 'DATETIME', 'TIMESTAMP', 'YEAR',
  // 二进制类型
  'BINARY', 'VARBINARY', 'BLOB', 'TINYBLOB', 'MEDIUMBLOB', 'LONGBLOB',
  // JSON
  'JSON'
),
```

### 当前限制

| 功能 | 状态 | 说明 |
|------|------|------|
| 基本列定义 | ✅ 正常 | `id INT` |
| 多列定义 | ✅ 正常 | `(id INT, name VARCHAR(50))` |
| 列约束 | ❌ 缺失 | `NOT NULL`, `PRIMARY KEY`, `UNIQUE` |
| 类型参数 | ⚠️ 部分 | `VARCHAR(50)` 不支持 |
| 表选项 | ❌ 缺失 | `ENGINE=InnoDB` |
| 外键 | ❌ 缺失 | `FOREIGN KEY` 约束 |
| 索引定义 | ❌ 缺失 | `CREATE INDEX` |

---

## 5. 方言特定限制

### 5.1 PostgreSQL DISTINCT ON

**状态**: 已标记为忽略

```rust
#[test]
#[ignore = "TODO: PostgreSQL DISTINCT ON syntax not yet supported in grammar"]
fn test_parse_postgresql_specific_syntax() {
    let source = "SELECT DISTINCT ON (name) name, id FROM users";
}
```

**缺少规则**:
```javascript
// 需要添加到 grammar.js 或 postgresql 方言文件
distinct_on_clause: $ => seq(
  'DISTINCT',
  'ON',
  '(',
  $.expression,
  repeat(seq(',', $.expression)),
  ')'
),
```

### 5.2 MySQL 隐式表别名

MySQL 支持不带 `AS` 关键字的表别名（如 `FROM users u`），但当前语法要求显式 `AS`。

**当前要求**:
- ❌ `FROM users u` (失败)
- ✅ `FROM users AS u` (成功)

**SQL 标准行为**:
- PostgreSQL 要求显式 `AS`
- MySQL 允许隐式别名
- 当前语法对两种方言都要求 `AS`

### 5.3 方言覆盖矩阵

| 特性 | MySQL 5.7 | MySQL 8.0 | PostgreSQL 12 | PostgreSQL 14 |
|------|-----------|-----------|---------------|---------------|
| 基础 SELECT | ✅ | ✅ | ✅ | ✅ |
| 表别名 (AS) | ⚠️ | ⚠️ | ✅ | ✅ |
| 表别名 (隐式) | ❌ | ❌ | ❌ | ❌ |
| JOIN | ⚠️ | ⚠️ | ⚠️ | ⚠️ |
| 窗口函数 | ❌ | ✅ | ❌ | ❌ |
| DISTINCT ON | ❌ | ❌ | ❌ | ❌ |
| CREATE TABLE | ✅ | ✅ | ✅ | ✅ |
| 多语句 | ❌ | ❌ | ❌ | ❌ |

---

## 6. tree-sitter 解析器特性限制

### 6.1 GLR 解析与冲突

tree-sitter 使用 GLR (Generalized LR) 解析，但存在以下限制：

1. **无回溯到同级的其他选择**: 一旦某个选择部分匹配，解析器不会尝试同级其他选择
2. **ERROR 节点**: 解析失败时会创建 ERROR 节点，但不会自动回退到其他解析路径
3. **conflicts 声明**: 只能声明冲突，不能自动解决

**当前 conflicts** (`grammar.js:64-66`):
```javascript
conflicts: $ => [
  [$.projection, $.expression],
],
```

此冲突表示 `projection` 和 `expression` 存在歧义，但 tree-sitter 会选择第一个匹配的规则。

### 6.2 标识符解析限制

**当前 identifier 规则** (`grammar.js:385-389`):
```javascript
identifier: $ => choice(
  /[a-zA-Z_][a-zA-Z0-9_]*/,
  /`[^`]+`/,        // MySQL style
  /"[^"]+"/,        // PostgreSQL style
  /\[[^\]]+\]/      // SQL Server style
),
```

**限制**:
- 无法区分保留字与标识符
- 无法验证标识符是否为关键字
- SQL 关键字（如 `SELECT`, `FROM`）可能与标识符冲突

---

## 7. 测试覆盖情况

### 当前测试统计

```
running 11 tests
test test_dialect_family_uses_same_grammar ... ok
test test_language_caching ... ok
test test_language_for_dialect_coverage ... ok
test test_language_for_dialect_postgresql_family ... ok
test test_language_for_dialect_mysql_family ... ok
test test_parse_postgresql_specific_syntax ... ignored (TODO)
test test_parse_mysql_specific_syntax ... ok
test test_parse_simple_query_mysql ... ok
test test_parse_with_syntax_error ... ok
test test_parse_complex_query ... ignored (TODO)
test test_parse_multiple_statements ... ignored (TODO)

test result: ok. 8 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out
```

### 测试覆盖矩阵

| 测试用例 | 状态 | 涵盖功能 |
|----------|------|----------|
| test_language_for_dialect | ✅ | 方言加载 |
| test_parse_simple_query_mysql | ✅ | 简单 SELECT |
| test_parse_mysql_specific_syntax | ✅ | LIMIT 子句 |
| test_parse_with_syntax_error | ✅ | 错误检测 |
| test_parse_complex_query | ⚠️ | JOIN + 聚合 (已忽略) |
| test_parse_multiple_statements | ⚠️ | 多语句 (已忽略) |
| test_parse_postgresql_specific_syntax | ⚠️ | DISTINCT ON (已忽略) |

---

## 8. 优先级建议

### 高优先级 (影响核心功能)

1. **修复 table_reference 歧义** - 影响 JOIN 查询
   - 尝试使用 `field()` 标注
   - 研究其他 tree-sitter SQL 语法的实现
   - 可能需要重构整个表引用解析逻辑

2. **实现多语句解析** - 影响 SQL 脚本支持
   - 在 `source_file` 规则中添加分号处理
   - 验证 CREATE TABLE 语句的完整性

### 中优先级 (增强功能)

3. **支持 MySQL 隐式表别名** - 提升用户体验
   - 为 MySQL 方言添加 `seq($.table_name, $.alias)` 选项
   - 保持 PostgreSQL 方言要求显式 `AS`

4. **完善 CREATE TABLE 支持** - 支持更多 DDL
   - 添加列约束 (NOT NULL, PRIMARY KEY)
   - 支持类型参数 (VARCHAR(50))
   - 添加表选项 (ENGINE, CHARSET)

### 低优先级 (锦上添花)

5. **PostgreSQL DISTINCT ON** - 方言特定功能
6. **窗口函数** - MySQL 8.0+ 功能
7. **高级 JOIN 类型** - CROSS JOIN, NATURAL JOIN

---

## 9. 参考资源

### 相关文件

| 文件 | 描述 |
|------|------|
| `crates/grammar/src/grammar/grammar.js` | 主语法文件 |
| `crates/grammar/src/grammar/dialect/*.js` | 方言特定扩展 |
| `crates/grammar/tests/api_tests.rs` | API 测试 |
| `crates/grammar/build.rs` | 构建脚本 |
| `crates/grammar/build.sh` | 语法生成脚本 |

### 有价值的参考

1. **tree-sitter 官方文档**: https://tree-sitter.github.io/tree-sitter/creating-parsers
2. **其他 SQL tree-sitter 语法**:
   - https://github.com/DerekStride/tree-sitter-sql
   - https://github.com/m-novikov/tree-sitter-sql
3. **SQL 标准**: ISO/IEC 9075 (SQL:2016)

---

## 10. 变更历史

| 日期 | 版本 | 变更说明 |
|------|------|----------|
| 2025-01-14 | 1.0 | 初始版本，记录当前限制 |

---

## 附录: 调试命令

### 重新生成语法
```bash
cd crates/grammar && ./build.sh
```

### 运行语法测试
```bash
cargo test -p unified-sql-grammar
```

### 查看解析树
```bash
cargo test -p unified-sql-grammar --test api_tests -- --nocapture
```

### 运行特定测试
```bash
cargo test -p unified-sql-grammar --test api_tests test_parse_complex_query
```
