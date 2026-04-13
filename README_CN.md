# SymLens

**给你的 AI 代理一个代码搜索引擎，别再用 `cat` 了。**

```
cat src/engine.rs              → 4000 tokens
symlens symbol "Engine::run"  →   60 tokens
```

SymLens 用 [tree-sitter](https://tree-sitter.github.io/) 解析代码库，建立全量符号索引——函数、类、调用图、import 关系。AI 代理（或你自己）按需精准查询，不再读整个文件。

支持 **Rust, TypeScript, Python, Go, Swift, Dart, C, C++, Kotlin**。

## 安装

```bash
cargo install --path .
```

## 三步上手

```bash
symlens index                                  # 1. 索引项目
symlens search "AudioEngine"                   # 2. 搜索符号
symlens symbol "src/engine.rs::Engine::run#method"  # 3. 获取签名
```

索引会缓存，后续运行几乎是即时的。

## 能做什么？

**搜索与导航**
```bash
symlens search "process audio"          # BM25 全文搜索
symlens symbol "<id>" --source          # 需要时获取完整源码
symlens outline --project               # 项目级符号树
symlens refs "Engine"                   # 查找所有引用（AST 级别）
```

**理解调用流**
```bash
symlens callers "process_block"         # 谁调用了这个函数？
symlens callees "process_block"         # 这个函数调用了谁？
symlens graph impact "Engine::run"      # 重构前的爆炸半径分析
symlens graph path "main" "cleanup"     # 两个符号间的调用路径
```

**Git 感知**
```bash
symlens diff --from main --to HEAD      # 两个 ref 间变更的符号
symlens blame "Engine::process_block"   # 符号级别的 git blame
```

**工具链**
```bash
symlens doctor                          # 检查索引健康状态
symlens watch                           # 文件变更自动重建
symlens completions zsh                 # Shell 补全
symlens init                            # 生成 symlens.toml 配置
```

## 性能

使用 [criterion](https://github.com/bheisler/criterion.rs) 在 SymLens 自身代码库上实测（55 文件，660 符号）：

| 操作 | 耗时 |
|------|------|
| 完整索引 | 17 ms |
| BM25 搜索 | 89 us |
| callers 查询 | 13 ns |
| 调用路径查找 | 20 us |
| 解析单个文件 | 437 us |

callers 查询只需 **13 纳秒**，因为调用图以 petgraph DiGraph 缓存——不需要每次重建。

## MCP 服务器

作为 [MCP](https://modelcontextprotocol.io/) 服务器运行，直接集成到 Claude Code、Cursor 或任何 MCP 兼容编辑器：

```bash
cargo install --path . --features mcp
symlens mcp
```

```json
{
  "mcpServers": {
    "symlens": { "command": "symlens", "args": ["mcp"] }
  }
}
```

8 个工具：`index`、`search`、`symbol`、`outline`、`refs`、`impact`、`callers`、`callees`。

## Agent 集成

一条命令让你的 AI 代理学会使用 SymLens：

```bash
symlens setup claude-code     # 写入 CLAUDE.md
symlens setup cursor          # 写入 .cursor/rules/symlens.mdc
symlens setup openclaw        # 写入 ~/.openclaw/skills/symlens/SKILL.md
symlens setup --all           # 一键全部安装
symlens setup --uninstall claude-code   # 卸载
```

## 架构

```
源代码 → tree-sitter AST → 符号提取 ─┬→ tantivy BM25 搜索
                                       ├→ petgraph 调用图
                                       ├→ Import 追踪
                                       └→ bincode 缓存
```

单一二进制，无运行时依赖。索引跨会话持久化。

## 局限

- **语法级分析**（~90% 精度）。没有类型推断和语义解析——如果你需要重命名重构或 99% 精度的跳转定义，请用 LSP。
- **只读**。SymLens 不修改代码。
- C++ 模板和 Kotlin 扩展函数的调用图覆盖有限。

## 许可证

MIT

---

[English](./README.md) | [完整命令参考](./docs/commands.md)
