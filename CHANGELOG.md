# Changelog

All notable changes to CodeLens will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-13

### Added

- **6 language parsers**: Rust, TypeScript, Python, Swift, Go, Dart — full support for symbols, calls, refs, and imports
- **18 commands**: index, search, symbol, outline, refs, callers, callees, lines, graph (impact/deps/path), watch, stats, blame, diff, export, setup, mcp
- **BM25 full-text search** via tantivy with custom camelCase/snake_case tokenizer
- **Call graph analysis**: callers, callees, transitive impact (blast radius), call path between symbols
- **Reference finding** (v3): AST-level identifier search with import-aware scope narrowing
- **Git integration**: `blame` (per-symbol) and `diff` (symbol-level changes between refs, +added/~modified/-deleted)
- **Incremental indexing**: skip unchanged files based on mtime, ~10x faster on re-index
- **Global `--json` flag**: all commands support JSON output for scripting and MCP
- **Colored output**: ANSI color highlighting with auto-detection (disable with `--no-color`)
- **`codelens export`**: dump full index as JSON (symbols + call edges + file metadata)
- **`codelens setup`**: one-command installation into Claude Code, OpenClaw, and Cursor (project-level or global with `-g`)
- **MCP server** (`--features mcp`): 6 tools via `tools/list` + `tools/call` JSON-RPC, tower-lsp/stdio transport
- **CI/CD**: GitHub Actions for check/test/clippy/fmt (CI) and cross-platform release builds (Release)
- **Global `--root`** flag to target any project directory
- **`codelens watch`**: auto-update index on file changes

### Architecture

- Rust 2024 edition, minimum rustc 1.85
- lib + bin crate structure
- tree-sitter (AST) + tantivy (BM25) + petgraph (call graph) + bincode (persistence) + rayon (parallel) + notify (file watch) + blake3 (hash)
- Zero `unsafe` code, zero external network calls
- MIT license
