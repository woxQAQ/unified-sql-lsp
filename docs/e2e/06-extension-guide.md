# 其他数据库扩展指南

## PostgreSQL 接入步骤

1. 配置文件
   - 复制 `config/mysql-5.7.toml`
   - 创建 `config/postgresql-12.toml`
   - 修改 dialect、version、connection_string
2. 数据准备
   - 创建 `fixtures/schema/postgresql/`
   - 转换表结构（数据类型映射）
   - 创建数据初始化脚本
3. Adapter 实现
   - 复制 `src/db/mysql_adapter.rs`
   - 创建 `src/db/postgresql_adapter.rs`
   - 替换数据库驱动
   - 调整连接逻辑
4. 测试用例
   - 复用 MySQL 测试用例
   - 创建 PostgreSQL 特有测试
   - 调整预期结果

## TiDB 接入步骤

1. 配置文件
   - 创建 `config/tidb-7.0.toml`
   - 使用 MySQL 协议
2. 数据准备
   - 复用 MySQL schema
   - 添加 TiDB 特性表
3. Adapter
   - 复用 MySQL adapter
   - 添加 TiDB 版本检测
4. 测试
   - 基础功能复用 MySQL 测试
   - 添加 TiDB 特有语法测试

## MariaDB 接入步骤

1. 配置文件
   - 创建 `config/mariadb-10.11.toml`
2. 数据准备
   - 复用 MySQL schema
3. Adapter
   - 复用 MySQL adapter
4. 测试
   - 复用 MySQL 测试集

## CockroachDB 接入步骤

1. 配置文件
   - 创建 `config/cockroachdb-23.0.toml`
2. 数据准备
   - 复用 PostgreSQL schema
   - 调整类型定义
3. Adapter
   - 复用 PostgreSQL adapter
4. 测试
   - 复用 PostgreSQL 测试集

## 通用扩展原则

1. 最小差异化
   - 复用已有代码
   - 抽象公共逻辑
2. 配置驱动
   - 数据库特定配置独立
   - 测试用例参数化
3. 渐进式接入
   - 先跑通基础测试
   - 逐步添加特性测试
4. 文档同步
   - 记录版本差异
   - 记录已知问题
