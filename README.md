# SymLens

[![Crates.io](https://img.shields.io/crates/v/symlens)](https://crates.io/crates/symlens)
[![CI](https://github.com/TtTRz/symlens/actions/workflows/ci.yml/badge.svg)](https://github.com/TtTRz/symlens/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/crates/l/symlens)](https://github.com/TtTRz/symlens/blob/main/LICENSE)
[![Downloads](https://img.shields.io/crates/d/symlens)](https://crates.io/crates/symlens)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange)](https://www.rust-lang.org)
[![Languages](https://img.shields.io/badge/languages-9-blue)](#what-can-it-do)

[中文](./README_CN.md) | English

**Give your AI agent a code search engine instead of `cat` or `grep`.**

```
cat src/engine.rs              → 4000 tokens
symlens symbol "Engine::run"  →   60 tokens
```

SymLens parses your codebase with [tree-sitter](https://tree-sitter.github.io/) and builds an index of every symbol — functions, classes, call graphs, imports. Your AI agent (or you) queries exactly what it needs instead of reading entire files.

Supports **Rust, TypeScript, Python, Go, Swift, Dart, C, C++, Kotlin**.

## Why Not Just `cat` and `grep`?

| | `cat` / `grep` | `symlens` |
|---|---|---|
| **Granularity** | Lines / files | Symbols (functions, classes, methods) |
| **Search** | Regex string matching | BM25 semantic search (understands camelCase/snake_case) |
| **Call graph** | No idea | Knows who calls whom (`callers` / `callees` / `graph path`) |
| **Impact analysis** | Impossible | `graph impact` tells you how many places break if you change a function |
| **Token cost** | Entire file (~4000 tokens) | Just the signature (~60 tokens) — **66x savings** |
| **Reference finding** | `grep "foo"` matches comments, strings, everything | AST-level — only real code references |

## Install

From crates.io:
```bash
cargo install symlens
```

From source:
```bash
git clone https://github.com/TtTRz/symlens.git
cd symlens
cargo install --path .
```

## 3-Step Start

```bash
symlens index                                  # 1. Index your project
symlens search "AudioEngine"                   # 2. Find symbols
symlens symbol "src/engine.rs::Engine::run#method"  # 3. Get just the signature
```

That's it. The index is cached — subsequent runs are instant.

## What Can It Do?

**Search & Navigate**
```bash
symlens search "process audio"          # BM25 full-text search
symlens symbol "<id>" --source          # Full source when you need it
symlens outline --project               # Project-wide symbol tree
symlens refs "Engine"                   # Find all references (AST-level)
```

**Understand Call Flow**
```bash
symlens callers "process_block"         # Who calls this?
symlens callees "process_block"         # What does this call?
symlens graph impact "Engine::run"      # Blast radius before refactoring
symlens graph path "main" "cleanup"     # Call path between two symbols
```

**Git-Aware**
```bash
symlens diff --from main --to HEAD      # Changed symbols between refs
symlens blame "Engine::process_block"   # Git blame at symbol level
```

**Tooling**
```bash
symlens doctor                          # Check index health
symlens watch                           # Auto-rebuild on file changes
symlens completions zsh                 # Shell completions
symlens init                            # Generate symlens.toml config
```

## Performance

Benchmarked with [criterion](https://github.com/bheisler/criterion.rs) on the SymLens codebase itself (55 files, 660 symbols):

| Operation | Time |
|-----------|------|
| Full index | 17 ms |
| BM25 search | 89 us |
| Callers query | 13 ns |
| Find call path | 20 us |
| Parse single file | 437 us |

The callers query runs in **13 nanoseconds** because the call graph is cached as a petgraph DiGraph — no reconstruction per query.

## MCP Server

Run as an [MCP](https://modelcontextprotocol.io/) server for Claude Code, Cursor, or any MCP-compatible editor:

```bash
cargo install symlens --features mcp
symlens mcp
```

```json
{
  "mcpServers": {
    "symlens": { "command": "symlens", "args": ["mcp"] }
  }
}
```

8 tools: `index`, `search`, `symbol`, `outline`, `refs`, `impact`, `callers`, `callees`.

## Agent Setup

One command to teach your AI agent to use SymLens:

```bash
symlens setup claude-code     # Writes CLAUDE.md
symlens setup cursor          # Writes .cursor/rules/symlens.mdc
symlens setup openclaw        # Writes ~/.openclaw/skills/symlens/SKILL.md
symlens setup --all           # All agents at once
symlens setup --uninstall claude-code   # Remove
```

## Architecture

```
Source Code → tree-sitter AST → Symbol Extraction ─┬→ tantivy BM25 Search
                                                    ├→ petgraph Call Graph
                                                    ├→ Import Tracking
                                                    └→ bincode Cache
```

Single binary, no runtime dependencies. Index persists across sessions.

## Limitations

- **Syntax-level analysis** (~90% precision). No type inference or semantic resolution — if you need rename-refactoring or go-to-definition with 99% accuracy, use an LSP.
- **Read-only**. SymLens doesn't modify code.
- C++ templates and Kotlin extension functions have limited call graph coverage.

## License

MIT

---

[Full command reference](./docs/commands.md) | [Changelog](./CHANGELOG.md)
