# 测试用例数据格式规范

## 概述

测试用例使用简单的文本格式 (`.txt`)，每个文件包含多个测试用例，用 `---` 分隔。这种格式易于人类编写和机器解析。

## 格式定义

### 基本结构

```
---
description: <测试描述>
dialect: <方言: mysql | postgresql | all>
context: <可选：上下文信息，如 schema 要求>
input: |
  <带光标标记的 SQL>
  使用 | 表示光标位置
expected: |
  <预期候选项，每行一个>
  可以包含完整标签或仅名称
options: |
  <可选：额外测试选项>
  - require_schema: <是否需要特定 schema>
  - min_items: <最少候选项数量>
  - contains: <必须包含的项>

---
<下一个测试用例>
```

### 字段说明

| 字段 | 必需 | 说明 |
|------|------|------|
| `description` | ✅ | 人类可读的测试描述 |
| `dialect` | ✅ | 适用的方言：`mysql`、`postgresql` 或 `all` |
| `context` | ❌ | 需要的数据库上下文（如表结构、外键关系等） |
| `input` | ✅ | 带光标标记的 SQL（支持多行） |
| `expected` | ✅ | 预期候选项列表（支持多行） |
| `options` | ❌ | 额外验证选项（YAML 列表格式） |

### 基本规则

1. **光标位置标记**
   - 使用 `|` 表示光标位置
   - `|` 必须在 SQL 中的精确位置
   - 多个测试点创建多个测试用例

2. **预期候选项格式**
   - 每行一个候选项
   - 完整格式：`label [kind] detail`
   - 简化格式：仅名称（测试时检查标签包含即可）

3. **选项格式**
   - 使用 YAML 列表语法
   - 常用选项：
     - `min_items`: 最少候选项数量
     - `contains`: 必须包含的项（逗号分隔）
     - `exact_match`: 是否精确匹配（默认 false）
     - `require_schema`: 是否需要特定 schema

## 示例

### 示例 1: 简单列名补全

```
---
description: SELECT 子句中的简单列名补全
dialect: all
context: users 表包含列 id, name, email, created_at
input: |
  SELECT | FROM users
expected: |
  id [Field] users.id
  name [Field] users.name
  email [Field] users.email
  created_at [Field] users.created_at
  * [Snippet]
```

### 示例 2: 表名补全

```
---
description: 表名补全
dialect: mysql
input: |
  SELECT * FROM |
expected: |
  users [Table]
  orders [Table]
  products [Table]
```

### 示例 3: 使用选项

```
---
description: JOIN 条件中的外键列优先
dialect: all
context: orders.customer_id 是外键引用 users.id
input: |
  SELECT * FROM orders o JOIN users u ON o.| = u.id
expected: |
  customer_id [Field] orders.customer_id
  id [Field] orders.id
options: |
  - contains: customer_id
  - min_items: 2
```

### 示例 4: 多行输入

```
---
description: 复杂查询中的列名补全
dialect: postgresql
input: |
  SELECT
    u.id,
    u.name,
    |
  FROM users u
  JOIN orders o ON u.id = o.user_id
  WHERE o.created_at > '2024-01-01'
expected: |
  o.user_id [Field] o.user_id
  o.total [Field] o.total
  u.email [Field] u.email
```

## 解析器实现

### 数据结构

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct TestCase {
    pub description: String,
    pub dialect: Dialect,
    pub context: Option<String>,
    pub input: String,
    pub expected: Vec<ExpectedItem>,
    pub options: Option<TestOptions>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ExpectedItem {
    Full { label: String, kind: String, detail: String },
    Simple(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TestOptions {
    pub min_items: Option<usize>,
    pub contains: Option<Vec<String>>,
    pub exact_match: Option<bool>,
    pub require_schema: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dialect {
    MySQL,
    PostgreSQL,
    All,
}
```

### 解析逻辑

```rust
use std::fs;
use std::path::Path;

pub fn parse_test_file(path: &Path) -> Result<Vec<TestCase>, ParseError> {
    let content = fs::read_to_string(path)?;
    parse_test_content(&content)
}

pub fn parse_test_content(content: &str) -> Result<Vec<TestCase>, ParseError> {
    let mut cases = Vec::new();
    let mut current_case: Option<TestCaseBuilder> = None;
    let mut current_field: Option<String> = None;
    let mut current_value = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // 分隔符
        if trimmed == "---" {
            if let Some(builder) = current_case.take() {
                cases.push(builder.build()?);
            }
            current_field = None;
            current_value.clear();
            continue;
        }

        // 字段声明
        if let Some(key) = trimmed.strip_suffix(':') {
            if !current_value.is_empty() {
                // 保存上一个字段的值
                if let Some(field) = &current_field {
                    current_case.get_or_insert_with(TestCaseBuilder::default)
                        .set_field(field, &current_value);
                }
            }
            current_field = Some(key.to_string());
            current_value.clear();
            continue;
        }

        // 多行值内容
        if let Some(field) = &current_field {
            current_value.push(line.to_string());
        }
    }

    // 最后一个测试用例
    if let Some(builder) = current_case {
        cases.push(builder.build()?);
    }

    Ok(cases)
}

struct TestCaseBuilder {
    description: Option<String>,
    dialect: Option<Dialect>,
    context: Option<String>,
    input: Option<String>,
    expected: Option<Vec<String>>,
    options: Option<Vec<String>>,
}

impl TestCaseBuilder {
    fn set_field(&mut self, field: &str, value: &[String]) {
        match field {
            "description" => self.description = Some(value.join("\n").trim().to_string()),
            "dialect" => self.dialect = Some(serde_yaml::from_str(value.join("\n").trim()).unwrap()),
            "context" => self.context = Some(value.join("\n").trim().to_string()),
            "input" => self.input = Some(dedent(value)),
            "expected" => self.expected = Some(value.iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()),
            "options" => self.options = Some(value.iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()),
            _ => {}
        }
    }

    fn build(self) -> Result<TestCase, ParseError> {
        Ok(TestCase {
            description: self.description.ok_or(ParseError::MissingField("description"))?,
            dialect: self.dialect.unwrap_or(Dialect::All),
            context: self.context,
            input: self.input.ok_or(ParseError::MissingField("input"))?,
            expected: self.expected.unwrap_or_default().into_iter().map(|s| {
                if s.contains('[') && s.contains(']') {
                    ExpectedItem::Full { /* 解析 */ }
                } else {
                    ExpectedItem::Simple(s)
                }
            }).collect(),
            options: self.options.and_then(|opts| parse_options(&opts).ok()),
        })
    }
}

fn dedent(lines: &[String]) -> String {
    // 去除多行文本的公共缩进
    lines.join("\n").trim().to_string()
}
```

### 验证逻辑

```rust
use lsp_types::CompletionItem;

pub fn validate_completion(
    actual: &[CompletionItem],
    expected_case: &TestCase,
) -> Result<(), ValidationError> {
    let options = expected_case.options.as_ref()
        .map(|o| o as &dyn TestOptions)
        .unwrap_or(&DefaultOptions);

    // 检查最少数量
    if let Some(min) = options.min_items() {
        if actual.len() < min {
            return Err(ValidationError::TooFewItems {
                expected: min,
                actual: actual.len(),
            });
        }
    }

    // 检查必须包含的项
    if let Some(contains) = options.contains() {
        for item in contains {
            if !actual.iter().any(|i| i.label.contains(item)) {
                return Err(ValidationError::MissingItem(item.clone()));
            }
        }
    }

    // 精确匹配（如果需要）
    if options.exact_match().unwrap_or(false) {
        for exp in &expected_case.expected {
            match exp {
                ExpectedItem::Full { label, kind, detail } => {
                    let found = actual.iter().any(|item| {
                        item.label == *label
                            && item.kind
                                .map(|k| k.as_str() == kind)
                                .unwrap_or(false)
                            && item.detail.as_deref() == Some(detail.as_str())
                    });
                    if !found {
                        return Err(ValidationError::ItemNotFound {
                            label: label.clone(),
                            kind: kind.clone(),
                            detail: detail.clone(),
                        });
                    }
                }
                ExpectedItem::Simple(name) => {
                    if !actual.iter().any(|i| i.label == *name) {
                        return Err(ValidationError::ItemNotFoundSimple(name.clone()));
                    }
                }
            }
        }
    }

    Ok(())
}
```

## 文件命名规范

测试用例文件使用 `.txt` 扩展名，按功能模块命名：

```
tests/e2e/fixtures/cases/
├── 01_basic_select.txt
├── 02_from_clause.txt
├── 03_join.txt
├── 04_where_clause.txt
├── 05_functions.txt
├── 06_advanced.txt
└── 07_edge_cases.txt
```

命名规则：
- 两位数字前缀表示执行顺序
- 下划线后跟描述性名称
- 小写字母，单词间用下划线分隔

## 错误处理

### 解析错误

- `ParseError::MissingField`: 缺少必需字段
- `ParseError::InvalidDialect`: 无效的方言值
- `ParseError::InvalidSyntax`: 语法错误

### 验证错误

- `ValidationError::TooFewItems`: 候选项数量不足
- `ValidationError::MissingItem`: 缺少必须包含的项
- `ValidationError::ItemNotFound`: 未找到指定的项
- `ValidationError::ItemNotFoundSimple`: 未找到简单匹配的项
