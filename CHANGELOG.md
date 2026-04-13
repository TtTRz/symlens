# Changelog

All notable changes to SymLens will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-04-13

### Changed

- **Upgraded all dependencies** to latest versions: tree-sitter 0.26, tantivy 0.26, petgraph 0.8, bincode 2, notify 8, toml 1.1
- **Install instructions** updated for crates.io (`cargo install symlens`) and source build

## [0.2.0] - 2026-04-13

### Changed

- **Renamed project from CodeLens to SymLens** to avoid collision with VS Code's CodeLens feature
  - Binary: `codelens` → `symlens`
  - Cache directory: `~/.codelens/` → `~/.symlens/`
  - Config file: `codelens.toml` → `symlens.toml`
  - MCP tool names: `codelens_*` → `symlens_*`

### Added

- **3 new language parsers**: C, C++, Kotlin (total: 9 languages)
- **`symlens doctor`**: diagnose index health, cache size, detected languages, call graph stats
- **`symlens completions <shell>`**: generate shell completions for bash/zsh/fish
- **`symlens init`**: generate default `symlens.toml` configuration file
- **`symlens setup --uninstall`**: remove SymLens integration from AI agents
- **MCP callers/callees tools**: `symlens_callers` and `symlens_callees` (total: 8 MCP tools)
- **Enhanced impact analysis**: transitive callees, affected modules count, cycle detection, risk score (0-100%)
- **`symlens.toml` config**: project-level configuration for max_files, ignore patterns, language filtering
- **Criterion benchmark suite**: `cargo bench` with 7 benchmarks covering index/search/callers/path

### Improved

- **Performance**: cached DiGraph in CallGraph (callers query: 13ns), bidirectional BFS for path finding, pre-cached lowercase for search, parallel refs scanning via rayon
- **Incremental indexing**: two-tier mtime + blake3 content hash — survives git checkout/rebase without false rebuilds
- **Deps resolution**: multi-language module resolution (was Rust-only), now supports C/C++ includes, Python/Kotlin dot-path imports, TS relative imports
- **MCP server**: static index cache (Arc-based, no disk reload per tool call)
- **Watch mode**: adaptive debounce with incremental rebuild reusing previous index
- **Error handling**: all Mutex `.unwrap()` replaced with `.expect()` with context messages
- **Dynamic tantivy heap**: 15-100MB based on symbol count (was hardcoded 50MB)
- **Registry fast path**: static `match` dispatch for 9 languages before HashMap fallback
- **Git diff**: single `--name-status` call replaces 3 separate subprocess calls

### Testing

- **85 tests** (was 48): added Go (11), Swift (10), C (5), C++ (5), Kotlin (7) test suites with fixtures
- **0 clippy warnings** (was 36)

## [0.1.0] - 2026-04-13

### Added

- **6 language parsers**: Rust, TypeScript, Python, Swift, Go, Dart — full support for symbols, calls, refs, and imports
- **18 commands**: index, search, symbol, outline, refs, callers, callees, lines, graph (impact/deps/path), watch, stats, blame, diff, export, setup, mcp
- **BM25 full-text search** via tantivy with custom camelCase/snake_case tokenizer
- **Call graph analysis**: callers, callees, transitive impact (blast radius), call path between symbols
- **Reference finding** (v3): AST-level identifier search with import-aware scope narrowing
- **Git integration**: `blame` (per-symbol) and `diff` (symbol-level changes between refs)
- **Incremental indexing**: skip unchanged files based on mtime
- **MCP server** (`--features mcp`): 6 tools via JSON-RPC, tower-lsp/stdio transport
- **`symlens setup`**: one-command installation into Claude Code, OpenClaw, and Cursor
- **CI/CD**: GitHub Actions for check/test/clippy/fmt + cross-platform release builds

### Architecture

- Rust 2024 edition, minimum rustc 1.92
- lib + bin crate structure
- tree-sitter + tantivy + petgraph + bincode + rayon + notify + blake3
- Zero `unsafe` code, zero external network calls
- MIT license
