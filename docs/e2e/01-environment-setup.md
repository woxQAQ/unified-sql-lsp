# E2E 测试环境搭建步骤

## Docker Compose 配置

1. 创建 `tests/e2e/docker-compose.yml`
2. 配置 MySQL 5.7 服务
   - 端口映射：3307:3306
   - 环境变量：ROOT 权限、测试数据库
   - 数据卷持久化
3. 预留 PostgreSQL 12+ 服务配置（注释状态）
   - 端口映射：5433:5432
   - 环境变量配置
4. 预留其他数据库服务配置块

## 环境管理脚本

1. 创建 `tests/e2e/scripts/setup.sh`
   - 启动数据库服务
   - 等待服务就绪检测
   - 健康检查
2. 创建 `tests/e2e/scripts/teardown.sh`
   - 停止并清理容器
   - 清理数据卷（可选）
3. 创建 `tests/e2e/scripts/healthcheck.sh`
   - 数据库连接测试
   - 返回 0/1 状态码

## 测试配置文件

1. 创建 `tests/e2e/config/mysql-5.7.toml`
   - dialect = "MySQL"
   - version = "5.7"
   - connection_string
   - schema 过滤配置
   - 连接池参数
2. 预留 PostgreSQL 配置文件模板
