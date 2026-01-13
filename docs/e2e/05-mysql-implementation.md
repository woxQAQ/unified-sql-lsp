# MySQL 5.7 E2E 测试实现步骤

## 阶段一：基础设施

1. 实现 Database Adapter
   - 创建 `src/db/mysql_adapter.rs`
   - 实现 DatabaseAdapter trait
   - 使用 mysql 驱动连接
   - 实现 setup/teardown
2. 配置加载
   - 读取 `config/mysql-5.7.toml`
   - 解析连接参数
   - 初始化 LiveCatalog
3. 测试运行器实现
   - 实现服务器启动逻辑
   - 实现 client 通信
   - 实现文档管理

## 阶段二：核心测试

1. SELECT 子句补全测试
   - 实现 `tests/select_completion.rs`
   - 测试简单列名
   - 测试限定列名
   - 测试通配符
2. FROM 子句补全测试
   - 实现 `tests/from_completion.rs`
   - 测试表名补全
   - 测试过滤规则
3. WHERE 子句补全测试
   - 实现 `tests/where_completion.rs`
   - 测试列名上下文
   - 测试表别名

## 阶段三：高级功能测试

1. JOIN 补全测试
   - 实现 `tests/join_completion.rs`
   - 测试 PK/FK 优先级
   - 测试多表 JOIN
2. 函数补全测试
   - 实现 `tests/function_completion.rs`
   - 测试函数签名显示
   - 测试上下文过滤
3. 关键字补全测试
   - 实现 `tests/keyword_completion.rs`
   - 测试上下文感知

## 阶段四：集成测试

1. 端到端流程测试
   - 文档打开
   - 增量编辑
   - 补全触发
   - 诊断验证
2. 并发测试
   - 多文档同时打开
   - 并发补全请求
3. 性能验证
   - 补全响应时间
   - 内存占用

## 阶段五：断言增强

1. 候选项验证
   - label 验证
   - kind 验证
   - detail 验证
2. 排序验证
   - 相关性排序
   - 字母排序
3. 文本编辑验证
   - insertText 格式
   - textEdit 范围

## 执行与验证

1. 运行测试
   - `cargo test -p e2e-tests`
2. 查看覆盖率
   - 生成覆盖率报告
3. 性能基准
   - 记录关键指标
4. 失败分析
   - 记录失败用例
   - 分析原因
