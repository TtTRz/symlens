# 🌲 CodeLens

**[English](./README.md)** | 中文

**基于 tree-sitter 的高效代码智能 CLI 工具。**

CodeLens 通过 tree-sitter 索引代码库，让你精准获取所需符号——函数签名、文件大纲、调用图谱、影响分析——而非阅读整个文件。专为 AI 代理（Claude Code）和开发者设计。

## 为什么选 CodeLens？

AI 编程代理（Claude Code、Cursor 等）读取整个文件浪费大量 token，而实际只需要一个函数签名。CodeLens 提供更高效的方式：

```
不用 CodeLens:  cat src/engine.rs              → ~4000 tokens（整个文件）
使用 CodeLens:  codelens symbol "Engine::run"   → ~60 tokens（仅签名）
                                                  = 节省 66 倍 token
```

## 快速开始

```bash
# 安装
cargo install --path .

# 索引项目
cd /path/to/your/project
codelens index

# 搜索符号
codelens search "process audio"

# 获取函数签名
codelens symbol "src/engine.rs::Engine::run#method"

# 需要时获取完整源码
codelens symbol "src/engine.rs::Engine::run#method" --source

# 项目概览
codelens outline --project

# 重构前做影响分析
codelens graph impact "Engine::run"

# 用 --root 指定不同项目
codelens --root /path/to/project search "handler"
```

## 命令一览

| 命令 | 说明 | Token 开销 |
|------|------|-----------|
| `codelens index` | 索引项目（并行、增量缓存） | — |
| `codelens search <query>` | BM25 搜索（支持 camelCase 拆分） | ~40/条 |
| `codelens symbol <id>` | 函数签名 + 文档注释 | ~60 |
| `codelens symbol <id> --source` | 完整源码 | ~500-2000 |
| `codelens outline <file>` | 文件符号树 | ~50/文件 |
| `codelens outline --project` | 项目结构概览 | ~200 |
| `codelens refs <name>` | 查找引用（AST 级别，感知 import） | ~30/条 |
| `codelens callers <name>` | 谁调用了此符号 | ~20/条 |
| `codelens callees <name>` | 此符号调用了谁 | ~20/条 |
| `codelens graph impact <name>` | 影响范围分析（爆炸半径） | ~200 |
| `codelens graph deps` | 模块依赖图 | ~150 |
| `codelens graph path <A> <B>` | 两个符号间的调用路径 | ~50 |
| `codelens lines <file> <start> <end>` | 按行范围获取源码 | 视长度 |
| `codelens blame <name>` | 符号行范围的 git blame | ~100 |
| `codelens diff --from <ref> --to <ref>` | 两个 git ref 间变更的符号 | ~50/条 |
| `codelens setup <agent>` | 安装 CodeLens 到 AI Agent | — |
| `codelens watch` | 监听文件变更自动更新索引 | — |
| `codelens stats` | 索引统计信息 | ~50 |

**全局参数：** `--root <path>` 指定项目根目录（默认通过 `.git` 自动检测）。

## 语言支持

5 种语言全部支持符号提取、调用分析、引用查找和 import 追踪：

| 语言 | 符号 | 调用 | 引用 | Import |
|------|------|------|------|--------|
| **Rust** | ✅ fn, struct, enum, trait, impl, const, type, macro | ✅ | ✅ v3 | ✅ |
| **TypeScript** | ✅ function, class, interface, type, enum, const | ✅ | ✅ | ✅ |
| **Python** | ✅ function, class, method, docstring | ✅ | ✅ | ✅ |
| **Swift** | ✅ func, class, struct, enum, protocol | ✅ | ✅ | ✅ |
| **Go** | ✅ func, method, struct, interface, type, const, var | ✅ | ✅ | ✅ |

## Git 集成

```bash
# 谁最后修改了这个符号？
codelens blame "AudioEngine::process_block"

# 两次提交间有哪些符号发生了变化？
codelens diff --from HEAD~3 --to HEAD

# 按类型过滤变更的符号
codelens diff --from main --to feature-branch --kind function
```

`diff` 可检测新增（+）、修改（~）和删除（-）的符号，按文件分组显示。

## MCP 服务器

CodeLens 可作为 [MCP](https://modelcontextprotocol.io/) 服务器运行，直接集成到 AI 编辑器中：

```bash
# 安装（包含 MCP 支持）
cargo install --path . --features mcp

# 启动 MCP 服务器（stdio 传输）
codelens mcp
```

**MCP 工具：** `codelens_index`、`codelens_search`、`codelens_symbol`、`codelens_outline`、`codelens_refs`、`codelens_impact`

服务器注册 `tools/list` 和 `tools/call` JSON-RPC 方法，遵循 MCP 协议。

MCP 配置（用于 Claude Code / Cursor）：

```json
{
  "mcpServers": {
    "codelens": {
      "command": "codelens",
      "args": ["mcp"]
    }
  }
}
```

## AI Agent 集成

一条命令把 CodeLens 安装到你的 AI 代理中：

```bash
# 安装到 Claude Code（写入 CLAUDE.md）
codelens setup claude-code

# 安装到 OpenClaw（写入 ~/.openclaw/skills/codelens/SKILL.md）
codelens setup openclaw

# 安装到 Cursor（写入 .cursor/rules/codelens.mdc）
codelens setup cursor

# 一键安装到所有 Agent
codelens setup --all

# 覆盖已有配置
codelens setup --all --force

# 查看支持的 Agent 列表
codelens setup --list
```

| Agent | `setup` 写入内容 | 安装位置 |
|-------|----------------|----------|
| **Claude Code** | `CLAUDE.md`（已存在则追加） | 项目根目录 |
| **OpenClaw** | `SKILL.md` 技能包 | `~/.openclaw/skills/codelens/` |
| **Cursor** | `.mdc` 规则文件 | `.cursor/rules/codelens.mdc` |

如果 `CLAUDE.md` 已存在，`setup claude-code` 会智能追加 CodeLens 段落而非覆盖。

## 架构

```
源文件 → tree-sitter AST → 符号提取 ─┬→ tantivy BM25 索引
                                       ├→ petgraph 调用图谱
                                       ├→ Import 追踪（refs v3）
                                       └→ bincode 持久化
```

| 组件 | 作用 |
|------|------|
| **tree-sitter** | 解析 5 种语言 AST，提取符号 |
| **tantivy** | 全文 BM25 搜索，自定义 camelCase/snake_case 分词器 |
| **petgraph** | 有向调用图谱，支持 callers/callees/影响分析 |
| **bincode** | 快速二进制序列化，索引持久化 |
| **rayon** | 并行文件解析 |
| **notify** | 文件系统监听，自动更新索引 |
| **tower-lsp** | MCP 服务器传输层（可选，`--features mcp`） |

## 性能

| 操作 | 耗时 |
|------|------|
| 索引 1000 个文件 | < 1 秒 |
| 搜索（BM25） | < 1 毫秒 |
| 符号查找 | < 0.1 毫秒 |
| 从磁盘加载索引 | < 50 毫秒 |
| Release 二进制大小 | 12 MB |

## 竞品对比

| | CodeLens | Serena (LSP) | pitlane-mcp | Aider repo-map |
|---|---------|-------------|------------|----------------|
| 速度 | ⚡ 50ms 冷启动 | 🐢 3-10 秒 | ⚡ 快 | 🐢 每次重建 |
| 依赖 | 无（单二进制） | Python + LSP 服务器 | 无 | Python |
| 调用图谱 | ✅ | ❌ | ✅ | ❌ |
| 影响分析 | ✅ | ❌ | ❌ | ❌ |
| Import 追踪 | ✅ | N/A (LSP) | ❌ | ❌ |
| BM25 搜索 | ✅ | ❌ | ✅ | ❌ |
| Git blame/diff | ✅ | ❌ | ❌ | ❌ |
| MCP 服务器 | ✅ | ✅ | ✅ | ❌ |
| 重构 | ❌（只读） | ✅ 重命名/移动 | ❌ | ❌ |
| 精度 | ~90%（语法级） | ~99%（语义级） | ~70% | N/A |

## CI/CD

已包含 GitHub Actions 工作流：

- **CI**（`ci.yml`）：check、test（Linux + macOS）、clippy、rustfmt — 每次推送/PR 到 `main` 触发
- **Release**（`release.yml`）：跨平台构建（Linux x86/ARM、macOS x86/ARM）+ GitHub Release 附校验和 — 推送 `v*` 标签触发

## 项目数据

- **Rust 2024 edition**，最低 rustc 1.85
- **~6000 行** Rust 代码，41 个源文件 + 680 行测试
- **43 个测试**（6 单元 + 37 集成），零 warning
- **17 个命令**（16 默认 + 1 MCP feature-gated）
- **5 种语言**完整支持符号/调用/引用/import

## 许可证

MIT — [TtTRz](mailto:romc1224@gmail.com)
