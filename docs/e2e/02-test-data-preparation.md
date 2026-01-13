# 测试数据集构建步骤

## 数据库 Schema 设计

## 表结构设计

1. 创建 `tests/e2e/fixtures/schema/`
2. 定义基础表（覆盖常见场景）
   - users 表（用户信息）
   - orders 表（订单）
   - products 表（商品）
   - order_items 表（订单明细）
3. 定义高级场景表
   - 自关联表（employees 组织结构）
   - 多对多关系表（tags, post_tags）
   - 分区表（按时间分区）
4. 添加索引定义
   - 主键
   - 外键
   - 唯一索引
   - 普通索引

## 数据准备脚本

1. 创建 `tests/e2e/fixtures/data/`
2. SQL 初始化脚本
   - `01_create_tables.sql`
   - `02_insert_basic_data.sql`
   - `03_insert_edge_case_data.sql`
3. 数据量控制
   - 小表：10-50 行（易于验证）
   - 中表：100-500 行（性能测试）
4. 数据多样性
   - NULL 值分布
   - 边界值
   - 特殊字符

## 元数据导出

1. 创建 `tests/e2e/fixtures/metadata/`
2. 导出表结构 JSON
   - 表名、列名、类型
   - 约束信息
   - 索引信息
3. 用于断言参考

## 版本差异处理

1. MySQL 特有语法
   - ENUM 类型
   - SET 类型
   - 自定义函数
2. 预留 PostgreSQL 特有结构位置
   - ARRAY 类型
   - JSONB 类型
   - ENUM 定义
