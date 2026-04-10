# 🌲 CodeLens

English | **[中文](./README_CN.md)**

**Token-efficient code intelligence CLI powered by tree-sitter.**

CodeLens indexes your codebase with tree-sitter and lets you fetch exactly the symbols you need — signatures, outlines, call graphs, and impact analysis — instead of reading entire files. Designed for AI agents (Claude Code) and humans alike.

## Why CodeLens?

AI coding agents (Claude Code, Cursor, etc.) waste tokens reading entire files when they only need a function signature. CodeLens gives them a smarter way:

```
Without CodeLens:  cat src/engine.rs              → ~4000 tokens (entire file)
With CodeLens:     codelens symbol "Engine::run"   → ~60 tokens (just the signature)
                                                     = 66× token savings
```

## Quick Start

```bash
# Install
cargo install --path .

# Index your project
cd /path/to/your/project
codelens index

# Search symbols
codelens search "process audio"

# Get function signature
codelens symbol "src/engine.rs::Engine::run#method"

# Get full source when needed
codelens symbol "src/engine.rs::Engine::run#method" --source

# Project overview
codelens outline --project

# Impact analysis before refactoring
codelens graph impact "Engine::run"

# Use --root to target a different project
codelens --root /path/to/project search "handler"
```

## Commands

| Command | Description | Token Cost |
|---------|-------------|------------|
| `codelens index` | Index the project (parallel, cached) | — |
| `codelens search <query>` | BM25 search with camelCase splitting | ~40/result |
| `codelens symbol <id>` | Signature + doc comment | ~60 |
| `codelens symbol <id> --source` | Full source code | ~500-2000 |
| `codelens outline <file>` | File symbol tree | ~50/file |
| `codelens outline --project` | Project structure overview | ~200 |
| `codelens refs <name>` | Find references (AST-level, import-aware) | ~30/ref |
| `codelens callers <name>` | Who calls this symbol | ~20/caller |
| `codelens callees <name>` | What this symbol calls | ~20/callee |
| `codelens graph impact <name>` | Blast radius analysis | ~200 |
| `codelens graph deps` | Module dependency graph | ~150 |
| `codelens graph path <A> <B>` | Call path between two symbols | ~50 |
| `codelens lines <file> <start> <end>` | Get source by line range | varies |
| `codelens blame <name>` | Git blame for a symbol's line range | ~100 |
| `codelens diff --from <ref> --to <ref>` | Changed symbols between git refs | ~50/change |
| `codelens setup <agent>` | Install CodeLens into AI agent | — |
| `codelens watch` | Auto-update index on file changes | — |
| `codelens stats` | Index statistics | ~50 |

**Global flags:** `--root <path>` to specify project root (default: auto-detect via `.git`).

## Language Support

All 5 languages have full support for symbols, call extraction, reference finding, and import tracking:

| Language | Symbols | Calls | Refs | Imports |
|----------|---------|-------|------|---------|
| **Rust** | ✅ fn, struct, enum, trait, impl, const, type, macro | ✅ | ✅ v3 | ✅ |
| **TypeScript** | ✅ function, class, interface, type, enum, const | ✅ | ✅ | ✅ |
| **Python** | ✅ function, class, method, docstring | ✅ | ✅ | ✅ |
| **Swift** | ✅ func, class, struct, enum, protocol | ✅ | ✅ | ✅ |
| **Go** | ✅ func, method, struct, interface, type, const, var | ✅ | ✅ | ✅ |

## Git Integration

```bash
# Who last modified a symbol?
codelens blame "AudioEngine::process_block"

# What symbols changed between commits?
codelens diff --from HEAD~3 --to HEAD

# What symbols changed (filter by kind)?
codelens diff --from main --to feature-branch --kind function
```

`diff` detects added (+), modified (~), and deleted (-) symbols with per-file breakdown.

## MCP Server

CodeLens can run as an [MCP](https://modelcontextprotocol.io/) server for direct integration with AI editors:

```bash
# Install with MCP support
cargo install --path . --features mcp

# Start MCP server (stdio transport)
codelens mcp
```

**MCP tools:** `codelens_index`, `codelens_search`, `codelens_symbol`, `codelens_outline`, `codelens_refs`, `codelens_impact`

The server registers `tools/list` and `tools/call` JSON-RPC methods following the MCP protocol.

MCP config (for Claude Code / Cursor):

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

## Agent Integration

One command to install CodeLens into your AI agent:

```bash
# Install into Claude Code (writes CLAUDE.md)
codelens setup claude-code

# Install into OpenClaw (writes ~/.openclaw/skills/codelens/SKILL.md)
codelens setup openclaw

# Install into Cursor (writes .cursor/rules/codelens.mdc)
codelens setup cursor

# Install into all agents at once
codelens setup --all

# Overwrite existing config
codelens setup --all --force

# List supported agents
codelens setup --list
```

| Agent | What `setup` writes | Location |
|-------|-------------------|----------|
| **Claude Code** | `CLAUDE.md` (appends if exists) | Project root |
| **OpenClaw** | `SKILL.md` skill package | `~/.openclaw/skills/codelens/` |
| **Cursor** | `.mdc` rule file | `.cursor/rules/codelens.mdc` |

If a `CLAUDE.md` already exists, `setup claude-code` intelligently appends the CodeLens section instead of overwriting.

## Architecture

```
Source Files → tree-sitter AST → Symbol Extraction ─┬→ tantivy BM25 Index
                                                     ├→ petgraph Call Graph
                                                     ├→ Import Tracking (refs v3)
                                                     └→ bincode Persistence
```

| Component | Role |
|-----------|------|
| **tree-sitter** | Parse 5 languages into ASTs, extract symbols |
| **tantivy** | Full-text BM25 search with custom camelCase/snake_case tokenizer |
| **petgraph** | Directed call graph for callers/callees/impact analysis |
| **bincode** | Fast binary serialization for index persistence |
| **rayon** | Parallel file parsing |
| **notify** | File system watching for auto-update |
| **tower-lsp** | MCP server transport (optional, `--features mcp`) |

## Performance

| Operation | Time |
|-----------|------|
| Index 1000 files | < 1s |
| Search (BM25) | < 1ms |
| Symbol lookup | < 0.1ms |
| Index load from disk | < 50ms |
| Release binary size | 12 MB |

## vs Other Tools

| | CodeLens | Serena (LSP) | pitlane-mcp | Aider repo-map |
|---|---------|-------------|------------|----------------|
| Speed | ⚡ 50ms cold start | 🐢 3-10s | ⚡ Fast | 🐢 Rebuilds each time |
| Dependencies | None (single binary) | Python + LSP servers | None | Python |
| Call graph | ✅ | ❌ | ✅ | ❌ |
| Impact analysis | ✅ | ❌ | ❌ | ❌ |
| Import tracking | ✅ | N/A (LSP) | ❌ | ❌ |
| BM25 search | ✅ | ❌ | ✅ | ❌ |
| Git blame/diff | ✅ | ❌ | ❌ | ❌ |
| MCP server | ✅ | ✅ | ✅ | ❌ |
| Refactoring | ❌ (read-only) | ✅ rename/move | ❌ | ❌ |
| Precision | ~90% (syntax) | ~99% (semantic) | ~70% | N/A |

## CI/CD

GitHub Actions workflows included:

- **CI** (`ci.yml`): check, test (Linux + macOS), clippy, rustfmt — runs on every push/PR to `main`
- **Release** (`release.yml`): cross-platform builds (Linux x86/ARM, macOS x86/ARM) + GitHub Release with checksums — triggered by `v*` tags

## Project Stats

- **Rust 2024 edition**, minimum rustc 1.85
- **~6000 lines** of Rust across 41 source files + 680 lines of tests
- **43 tests** (6 unit + 37 integration), zero warnings
- **17 commands** (16 default + 1 MCP feature-gated)
- **5 languages** with full symbol/call/refs/import support

## License

MIT — [TtTRz](mailto:romc1224@gmail.com)

---

**[🇨🇳 中文文档](./README_CN.md)**
