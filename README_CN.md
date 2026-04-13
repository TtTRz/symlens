<div align="center">

# SymLens

**给你的 AI 代理一个代码搜索引擎，别再用 `cat` 或 `grep` 了。**

[![Crates.io](https://img.shields.io/crates/v/symlens)](https://crates.io/crates/symlens)
[![CI](https://github.com/TtTRz/symlens/actions/workflows/ci.yml/badge.svg)](https://github.com/TtTRz/symlens/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/crates/l/symlens)](https://github.com/TtTRz/symlens/blob/main/LICENSE)
[![Downloads](https://img.shields.io/crates/d/symlens)](https://crates.io/crates/symlens)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange)](https://www.rust-lang.org)
[![Languages](https://img.shields.io/badge/languages-9-blue)](#-能做什么)

中文 | [English](./README.md)

</div>

---

```bash
cargo install symlens           # 安装
symlens index                   # 索引项目
symlens search "AudioEngine"    # 搜索符号
symlens symbol "Engine::run"    # 获取签名 → 60 tokens，不再读 4000 tokens 的整个文件
```

SymLens 用 [tree-sitter](https://tree-sitter.github.io/) 解析代码库，建立全量符号索引——函数、类、调用图、import 关系。AI 代理（或你自己）按需精准查询，不再读整个文件。

> **9 种语言：** Rust · TypeScript · Python · Go · Swift · Dart · C · C++ · Kotlin

---

## 为什么不直接用 `cat` 和 `grep`？

| | `cat` / `grep` | SymLens |
|:--|:--|:--|
| **粒度** | 行 / 文件 | 符号（函数、类、方法） |
| **搜索** | 正则匹配字符串 | BM25 语义搜索（理解 camelCase / snake_case） |
| **调用关系** | — | 谁调用谁 · `callers` · `callees` · `graph path` |
| **影响分析** | — | `graph impact` — 重构前的爆炸半径 |
| **Token 开销** | ~4000 tokens（整个文件） | ~60 tokens（仅签名）— **便宜 66 倍** |
| **引用查找** | 匹配注释、字符串、所有东西 | AST 级别 — 只匹配真正的代码引用 |

---

## 🔍 能做什么？

<table>
<tr><td width="50%">

**搜索与导航**
```bash
symlens search "process audio"
symlens symbol "<id>" --source
symlens outline --project
symlens refs "Engine"
```

</td><td width="50%">

**理解调用流**
```bash
symlens callers "process_block"
symlens callees "process_block"
symlens graph impact "Engine::run"
symlens graph path "main" "cleanup"
```

</td></tr>
<tr><td>

**Git 感知**
```bash
symlens diff --from main --to HEAD
symlens blame "Engine::process_block"
```

</td><td>

**工具链**
```bash
symlens doctor
symlens watch
symlens completions zsh
symlens init
```

</td></tr>
</table>

---

## ⚡ 性能

使用 [criterion](https://github.com/bheisler/criterion.rs) 在 SymLens 自身代码库上实测（55 文件，660 符号）：

```
完整索引 ·········· 17 ms
BM25 搜索 ········· 89 µs
callers 查询 ······ 13 ns   ← 缓存 DiGraph，无需每次重建
调用路径查找 ······ 20 µs   ← 双向 BFS
解析单个文件 ······ 437 µs
```

---

## 🤖 MCP 服务器

作为 [MCP](https://modelcontextprotocol.io/) 服务器运行，集成到 Claude Code、Cursor 或任何 MCP 兼容编辑器：

```bash
cargo install symlens --features mcp
symlens mcp
```

<details>
<summary>MCP 配置（点击展开）</summary>

```json
{
  "mcpServers": {
    "symlens": { "command": "symlens", "args": ["mcp"] }
  }
}
```

**8 个工具：** `index` · `search` · `symbol` · `outline` · `refs` · `impact` · `callers` · `callees`

</details>

---

## 🔌 Agent 集成

一条命令让你的 AI 代理学会使用 SymLens：

```bash
symlens setup claude-code                    # → CLAUDE.md
symlens setup cursor                         # → .cursor/rules/symlens.mdc
symlens setup openclaw                       # → ~/.openclaw/skills/symlens/SKILL.md
symlens setup --all                          # 一键全部安装
symlens setup --uninstall claude-code        # 卸载
```

---

## 🏗️ 架构

```mermaid
graph LR
    A[源代码] --> B[tree-sitter AST]
    B --> C[符号提取]
    C --> D[tantivy BM25 搜索]
    C --> E[petgraph 调用图]
    C --> F[Import 追踪]
    C --> G[bincode 缓存]
```

单一二进制 · 无运行时依赖 · 索引跨会话持久化

---

## 局限

- **语法级分析**（~90% 精度）。没有类型推断——如果需要重命名重构或 99% 精度，请用 LSP。
- **只读。** SymLens 不修改代码。
- C++ 模板和 Kotlin 扩展函数的调用图覆盖有限。

## 许可证

MIT

---

<sub>[English](./README.md) · [完整命令参考](./docs/commands.md) · [Changelog](./CHANGELOG.md)</sub>
