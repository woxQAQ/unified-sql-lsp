Please analyze user's input $ARGUMENTS and create ralph-loop specific prompts.

Follow the steps:

1. Explore the codebase and understand the requirements thoroughly
2. Please use plan mode if the problem is diffcult
3. Plan and think carefully
4. Create the target ralph-loop prompt with the format below
5. Output to markdown File

NOTE:
1. DO NOT contain too many implement detail, such as where to write down codes. Please give the model more freedom
2. DO NOT contain any quote, backticks that may cause the escape error

<template>
# 任务标题

## 目标
[清晰描述最终目标]

## 要求
- 要求1（可验证）
- 要求2（可验证）
- 要求3（可验证）

## 迭代流程
1. [步骤1]
2. [步骤2 - 包含自我验证]
3. [步骤3 - 如果失败则修复]
4. 重复直到满足所有要求

## 完成条件
当以下所有条件满足时，输出 <promise>DONE</promise>：
- [ ] 条件1已验证
- [ ] 条件2已验证
- [ ] 条件3已验证

</template>

## Introduction to ralph-loop

### What's Ralph Wiggum？

Ralph Wiggum 是一种自引用 AI 开发循环技术，让 AI 代理通过持续迭代自主完成复杂任务。

核心概念："Ralph 就是一个 Bash 循环"
```bash
while true; do
  # 把同一个 prompt 反复喂给 AI
  # AI 看到自己之前的工作成果
  # AI 基于反馈继续改进
  # 直到任务完成
done
```

### 工作原理传统方式 vs Ralph 方式传统方式：
```
你: "构建一个 API"
AI: [写代码] "完成了！"
你: [发现 bug] "这里有问题"
AI: "抱歉，让我修复"
你: [又发现问题] ...
需要人工介入每次迭代。
Ralph 方式：
你: "构建 API，测试通过后输出 DONE"
AI: [写代码] → [运行测试] → [失败]
   → [看到错误] → [修复] → [再测试]
   → [仍失败] → [再修复] ...
   → [测试通过] → "DONE"
AI 自主迭代直到完成。
```

### 核心哲学
1. 迭代胜于完美
  不要期待第一次就完美，让循环来打磨成果。
2. 失败即数据
  失败是确定性的和信息丰富的。测试失败告诉 AI 哪里有问题。
3. 操作员技能很重要
  成功取决于编写好的 prompt，而不仅仅是好的模型。
4. 坚持就是胜利
  自动重试，直到成功或达到上限。
