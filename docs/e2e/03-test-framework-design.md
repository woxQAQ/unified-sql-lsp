# E2E 测试框架设计

## Crate 结构

1. 创建 `tests/e2e/Cargo.toml`
2. 依赖声明
   - unified-sql-lsp-lsp
   - unified-sql-lsp-catalog
   - tokio
   - tower-lsp
   - serde
   - anyhow
3. 目录结构
   - `tests/e2e/src/` - 测试代码
   - `tests/e2e/fixtures/` - 测试数据
   - `tests/e2e/scripts/` - 脚本
   - `tests/e2e/config/` - 配置

## 核心组件设计

### 测试运行器

1. 创建 `src/runner.rs`
   - 启动 LSP 服务器
   - 建立客户端连接
   - 文档生命周期管理
   - 清理资源

### LSP 客户端模拟

1. 创建 `src/client.rs`
   - 实现 tower-lsp Client trait
   - 捕获 diagnostics
   - 记录 completion 响应
   - 模拟光标移动

### 测试断言工具

1. 创建 `src/assertions.rs`
   - `assert_completion_contains()`
   - `assert_completion_not_contains()`
   - `assert_completion_count()`
   - `assert_completion_order()`
   - `assert_diagnostics()`

### 测试用例宏

1. 创建 `src/macros.rs`
   - `e2e_test!` 宏
   - 参数化测试
   - 自动生成测试函数

## 数据库适配器接口

1. 创建 `src/db/adapter.rs`
   - `DatabaseAdapter` trait
   - MySQL 实现
   - 预留 PostgreSQL 实现位置
2. 方法定义
   - `setup()`
   - `teardown()`
   - `get_connection_string()`

## 测试辅助工具

1. 创建 `src/utils.rs`
   - SQL 片段解析（提取光标位置）
   - LSP Position 计算
   - CompletionItem 过滤
   - 结果格式化输出
