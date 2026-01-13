# 测试用例生成指南

## 概述

本文档描述如何生成和组织测试用例，确保覆盖所有已实现的功能和边界情况。

## 测试用例分类

### 文件组织

```
tests/e2e/fixtures/cases/
├── 01_basic_select.txt    # SELECT 子句
├── 02_from_clause.txt     # FROM 子句
├── 03_join.txt            # JOIN 操作
├── 04_where_clause.txt    # WHERE 条件
├── 05_functions.txt       # 函数补全
├── 06_advanced.txt        # 高级特性
└── 07_edge_cases.txt      # 边界情况
```

## 各类别详细说明

### 1. 基础补全测试 (01_basic_select.txt)

**目标**: 验证 SELECT 子句中的列名补全

**覆盖场景**:
- 简单列名补全: `SELECT | FROM users`
- 多列选择中的补全: `SELECT id, name, | FROM users`
- 表名限定列补全: `SELECT users.| FROM users`
- 表别名限定: `SELECT u. FROM users u`
- 通配符扩展: `SELECT * |` (考虑是否支持部分通配符)
- 嵌套查询中的 SELECT: `SELECT (SELECT | FROM orders) FROM users`

**示例测试点**:
```
---
description: 简单列名补全
dialect: all
input: |
  SELECT | FROM users
expected: |
  id [Field] users.id
  name [Field] users.name
  email [Field] users.email

---
description: 表名限定的列名补全
dialect: all
input: |
  SELECT users.| FROM users
expected: |
  id [Field] users.id
  name [Field] users.name
```

### 2. FROM 子句 (02_from_clause.txt)

**目标**: 验证表名补全和表别名

**覆盖场景**:
- 基础表名补全: `SELECT * FROM |`
- Schema 限定表名 (PostgreSQL): `SELECT * FROM public.|`
- 数据库名限定表名 (MySQL): `SELECT * FROM app_db.|`
- 表别名定义: `SELECT * FROM users |`
- 多表 FROM: `SELECT * FROM users, |`
- 子查询别名: `SELECT * FROM (SELECT id FROM users) |`

**Schema 上下文要求**:
- 需要预先定义的表列表
- Schema 层级结构（如 public, app_db）

**示例测试点**:
```
---
description: 表名补全
dialect: all
context: 数据库包含 users, orders, products 表
input: |
  SELECT * FROM |
expected: |
  users [Table]
  orders [Table]
  products [Table]

---
description: PostgreSQL schema 限定
dialect: postgresql
context: public schema 包含 users, orders
input: |
  SELECT * FROM public.|
expected: |
  users [Table] public.users
  orders [Table] public.orders
```

### 3. JOIN 测试 (03_join.txt)

**目标**: 验证 JOIN 相关的补全功能

**覆盖场景**:
- JOIN 类型关键字: `SELECT * FROM users |`
- JOIN 后的表名: `SELECT * FROM users JOIN |`
- ON 条件中的列名: `SELECT * FROM orders o JOIN users u ON o.| = u.id`
- 多表 JOIN: `SELECT * FROM users u JOIN orders o ON u.id = o.| JOIN products p ON o.product_id = p.id`
- 外键列优先级
- 歧义列处理（两个表都有同名列）

**Schema 上下文要求**:
- 外键关系定义
- 主键列标记
- 同名列冲突信息

**示例测试点**:
```
---
description: INNER JOIN ON 条件中的列名补全
dialect: all
context: orders.customer_id 是外键引用 users.id
input: |
  SELECT * FROM orders o INNER JOIN users u ON o.| = u.id
expected: |
  customer_id [Field] o.customer_id
  id [Field] o.id
options: |
  - contains: customer_id
  - min_items: 2
```

### 4. WHERE 子句 (04_where_clause.txt)

**目标**: 验证 WHERE 条件中的列名和运算符补全

**覆盖场景**:
- 列名补全: `SELECT * FROM users WHERE |`
- 运算符上下文: `SELECT * FROM users WHERE id |`
- 函数嵌套: `SELECT * FROM users WHERE |(id) > 0`
- 逻辑运算符: `SELECT * FROM users WHERE name = 'test' |`
- BETWEEN: `SELECT * FROM users WHERE id BETWEEN |`
- IN: `SELECT * FROM users WHERE id IN (|`

**示例测试点**:
```
---
description: WHERE 子句中的列名补全
dialect: all
input: |
  SELECT * FROM users WHERE |
expected: |
  id [Field] users.id
  name [Field] users.name
  email [Field] users.email

---
description: 运算符后的值上下文
dialect: all
input: |
  SELECT * FROM users WHERE id =
expected: |
  1 [Snippet]
  NULL [Keyword]
```

### 5. 函数补全 (05_functions.txt)

**目标**: 验证函数补全功能

**覆盖场景**:
- 聚合函数 (COUNT, SUM, AVG, MIN, MAX)
- 标量函数 (UPPER, LOWER, SUBSTRING, TRIM, etc.)
- 窗口函数 (ROW_NUMBER, RANK, DENSE_RANK, etc.)
- 日期时间函数
- 函数参数中的列名: `SELECT COUNT(|) FROM users`
- 嵌套函数: `SELECT ROUND(SUM(|), 2) FROM orders`

**函数元数据要求**:
- 函数签名
- 参数类型
- 返回类型
- 函数描述

**示例测试点**:
```
---
description: 聚合函数补全
dialect: all
input: |
  SELECT C| FROM users
expected: |
  COUNT [Function] COUNT(*) - count rows
  COALESCE [Function] COALESCE(val1, val2, ...) - return first non-null value
  CAST [Function] CAST(expr AS type) - convert type
options: |
  - contains: COUNT
  - min_items: 3
```

### 6. 高级场景 (06_advanced.txt)

**目标**: 验证高级 SQL 特性的补全

**覆盖场景**:
- **子查询**:
  - FROM 子查询: `SELECT * FROM (SELECT | FROM users) AS t`
  - WHERE 子查询: `SELECT * FROM users WHERE id IN (SELECT | FROM orders)`
  - SELECT 列表子查询: `SELECT id, (SELECT | FROM orders) FROM users`

- **CTE (WITH)**:
  - CTE 定义: `WITH | AS (SELECT * FROM users) SELECT * FROM t`
  - 多个 CTE: `WITH a AS (SELECT 1), b AS (|) SELECT * FROM b`

- **UNION**:
  - UNION 后的 SELECT: `SELECT id FROM users UNION SELECT | FROM orders`
  - UNION ALL: `SELECT id FROM users UNION ALL SELECT | FROM orders`

- **GROUP BY / HAVING**:
  - GROUP BY 列: `SELECT name, COUNT(*) FROM users GROUP BY |`
  - HAVING 条件: `SELECT name, COUNT(*) FROM users GROUP BY name HAVING |`

- **ORDER BY**:
  - ORDER BY 列: `SELECT * FROM users ORDER BY |`
  - 排序方向: `SELECT * FROM users ORDER BY name |`

**示例测试点**:
```
---
description: CTE 定义中的列名补全
dialect: all
input: |
  WITH user_stats AS (
    SELECT | FROM users
  )
  SELECT * FROM user_stats
expected: |
  id [Field] users.id
  name [Field] users.name
  COUNT [Function] COUNT(*)
options: |
  - contains: id, COUNT
```

### 7. 边界情况 (07_edge_cases.txt)

**目标**: 验证边界情况和错误处理

**覆盖场景**:
- **空白输入**: `|`
- **语法错误**: `SELET | FROM users` (注意 SELET 拼写错误)
- **不完整的关键字**: `S|` (应该补全为 SELECT)
- **超长标识符**: 测试对长表名/列名的处理
- **特殊字符**: 带下划线、数字等的列名
- **深层嵌套**: 5 层以上的子查询嵌套
- **同名列冲突**: 表名和列名同名的情况
- **保留字作为标识符**: 使用引号的保留字

**示例测试点**:
```
---
description: 空白输入
dialect: all
input: |
  |
expected: |
  SELECT [Keyword]
  INSERT [Keyword]
  UPDATE [Keyword]
  DELETE [Keyword]
  WITH [Keyword]
options: |
  - min_items: 3

---
description: 语法错误的 SQL
dialect: all
input: |
  SELET | FROM users
expected: |
options: |
  - min_items: 0

---
description: 深层嵌套子查询
dialect: all
input: |
  SELECT * FROM (SELECT * FROM (SELECT * FROM (SELECT * FROM users WHERE | ) AS t1 ) AS t2 ) AS t3
expected: |
  id [Field] users.id
  name [Field] users.name
options: |
  - min_items: 1
```

## 覆盖率要求

### 已实现功能

根据 `FEATURE_LIST.yaml` 中的定义：

- **COMPLETION-001**: SELECT 子句列名补全 ✅
- **COMPLETION-002**: FROM 子句表名补全 ✅
- **COMPLETION-003**: JOIN 列名补全 ✅
- **COMPLETION-004**: WHERE 子句补全 ✅
- **COMPLETION-005**: 函数补全 ✅
- **COMPLETION-006**: 关键字补全 ✅
- **COMPLETION-007**: 运算符补全 ✅

### 边界情况覆盖

- [ ] 空白输入返回关键字
- [ ] 语法错误不影响补全（或返回空列表）
- [ ] 超长标识符（>64 字符）
- [ ] 特殊字符处理（下划线、数字、Unicode）
- [ ] 保留字冲突处理

### 方言差异覆盖

- [ ] MySQL 特定语法
- [ ] PostgreSQL 特定语法
- [ ] 共享语法（all 标记）

### 性能场景

- [ ] 大量表（>100）
- [ ] 大量列（>50）
- [ ] 深层嵌套（>5 层）
- [ ] 复杂 JOIN（>3 表）

## 测试用例生成策略

### 1. 基于功能列表

从 `FEATURE_LIST.yaml` 中提取每个功能点，为每个功能点创建对应测试用例。

### 2. 基于上下文类型

每个补全上下文（SELECT, FROM, WHERE, JOIN 等）至少需要：
- 1 个基础用例
- 1 个方言特定用例（如有）
- 1 个边界用例

### 3. 基于优先级

**高优先级**（核心功能）:
- 基础列名补全
- 表名补全
- JOIN 列名补全

**中优先级**（常用功能）:
- 函数补全
- WHERE 条件补全

**低优先级**（高级功能）:
- CTE
- 复杂子查询

### 4. 增量生成

1. 从最简单的用例开始
2. 逐步增加复杂度
3. 每个新功能先添加正向用例
4. 再添加边界和错误用例

## Schema 数据准备

### 静态 Schema 文件

测试用例可能需要预定义的 schema 信息：

```yaml
# tests/e2e/fixtures/schema/test_schema.yaml
tables:
  - name: users
    columns:
      - name: id
        type: INTEGER
        primary_key: true
      - name: name
        type: VARCHAR(100)
      - name: email
        type: VARCHAR(255)

  - name: orders
    columns:
      - name: id
        type: INTEGER
        primary_key: true
      - name: user_id
        type: INTEGER
        foreign_key:
          table: users
          column: id
      - name: total
        type: DECIMAL(10,2)

foreign_keys:
  - from_table: orders
    from_column: user_id
    to_table: users
    to_column: id
```

### 动态 Schema 生成

对于需要大量表的性能测试，可以动态生成：
- 随机表名
- 随机列名
- 随机外键关系

## 测试用例优先级矩阵

| 功能 | 基础 | 进阶 | 边界 | 性能 | 优先级 |
|------|------|------|------|------|--------|
| SELECT 列名 | ✅ | ✅ | ✅ | ❌ | 高 |
| FROM 表名 | ✅ | ✅ | ✅ | ✅ | 高 |
| JOIN 列名 | ✅ | ✅ | ✅ | ❌ | 高 |
| WHERE 条件 | ✅ | ✅ | ✅ | ❌ | 中 |
| 函数补全 | ✅ | ✅ | ✅ | ❌ | 中 |
| CTE | ✅ | ❌ | ❌ | ❌ | 低 |
| 子查询 | ✅ | ✅ | ✅ | ❌ | 中 |

## 自动化生成建议

对于重复性高的测试用例，可以考虑使用脚本生成：

1. **组合生成**: 为常见模式组合生成测试用例
   - 所有 JOIN 类型 (INNER, LEFT, RIGHT, CROSS) × 所有表组合

2. **模板填充**: 使用模板填充不同的表名、列名

3. **规则驱动**: 基于 SQL 语法规则生成测试用例
   - 为每个产生式规则创建测试用例

注意：自动化生成的测试用例仍需人工审查和调整。
