# Changelog

All notable changes to SymLens will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-04-15

### Added

- **`export --format sqlite`**: SQLite export with 4 tables (`symbols`, `call_edges`, `files`, `metadata`) + indexes for common queries. Default output to `~/.symlens/indexes/symlens-<hash>.db`, supports `--output` for custom path
- **`graph deps --json`**: JSON output for module dependency graph with `modules` and `edges` fields
- **`graph deps --module` / `--reverse`**: query a specific module's dependencies (`--module`) or who depends on it (`--module --reverse`). Exposes `DepsGraph::dependents()` and `DepsGraph::dependencies()` to CLI
- **NO_COLOR / CLICOLOR_FORCE support**: respects [NO_COLOR](https://no-color.org/) env var (disables color), `CLICOLOR_FORCE` (enables color even in pipes). Precedence: `--no_color` > `NO_COLOR` > `CLICOLOR_FORCE` > isatty auto-detection

### Changed

- **`rusqlite` dependency**: added as native-only optional dependency (bundled SQLite)
- **`ProjectIndex::remove_file` doc**: clarified that current incremental re-index naturally excludes deleted files; method retained for future fine-grained updates

### Testing

- **132 tests** (was 118): added 4 DepsGraph query tests, 2 SQLite export tests, 8 color resolution tests

## [0.6.1] - 2026-04-15

### Changed

- **Python `extract_imports` migrated to tree-sitter**: replaced regex-based line scanning with AST traversal — now handles dotted imports (`import os.path`), aliased imports (`import numpy as np`), relative imports (`from ..pkg import Mod`), and `from x import y as z`
- **Removed unused `child_by_field_name` wrapper** in TypeScript parser

### Added

- 8 Python AST import tests covering: simple, dotted, from-import, typing, relative, wildcard, aliased, from-aliased

## [0.6.0] - 2026-04-14

### Added

- **MCP tools**: 3 new tools — `symlens_lines` (read source lines), `symlens_diff` (changed symbols between git refs), `symlens_stats` (index statistics). MCP tools now 7→10
- **`LanguageParser::language()`**: all 9 parsers return their `tree_sitter::Language`, enabling single-parse optimization
- **`extract_all()`**: new trait method that parses once and extracts symbols + calls + imports simultaneously — indexing now does **1 parse per file instead of 3**
- **`ParsedOutput`**: new struct holding all extraction results from a single parse pass
- **TypeScript AST imports**: `extract_imports` migrated from regex to tree-sitter AST traversal — now handles `import type`, `import * as`, default imports, and `export {} from` re-exports

### Changed

- **Diff command refactor**: core logic extracted into `collect_changes()` public function, shared by CLI and MCP
- **tantivy Reader/QueryParser caching**: `SearchEngine` now caches `IndexReader` and `QueryParser` across searches instead of recreating per query — `index_symbols()` auto-reloads both after commit
- **JSON output unification**: `format_search_results()` and `format_symbol_value()` shared between CLI and MCP — consistent field names (`qualified_name`, `visibility`, `parent`) across both interfaces

## [0.5.0] - 2026-04-14

### Fixed

- **BFS/DFS confusion**: `transitive_callers` and `compute_transitive_callees` used `Vec::pop()` (LIFO/DFS) instead of `VecDeque::pop_front()` (FIFO/BFS) — depth-limited queries now correctly explore shallower nodes first
- **Impact score double-counting**: `direct_callers.len() + transitive_callers.len()` counted direct callers twice (they are depth-1 in transitive results), inflating risk scores
- **`extract_module` wrong segment**: returned `"crate"` for paths like `crate::audio::engine::AudioEngine` — now skips `crate`/`self`/`super` prefixes
- **`Box::leak` memory leak**: all 9 `collect_*_calls` functions leaked one `&'static str` per function definition per parse — replaced with `Option<String>` ownership pattern
- **Non-atomic index write**: `storage::save()` wrote directly to `index.bin` — crash mid-write corrupted the index. Now writes to temp file then `fs::rename()` for atomic replacement
- **Tokenizer offset bug**: `text.find(word)` always found the first occurrence of duplicate words, corrupting tantivy position data — now tracks cumulative search offset
- **`callers_partial` O(E*T)**: `Vec::contains()` per edge was O(T) — changed to `HashSet` for O(1) lookup
- **CallGraph edge dedup missing**: `build()` did not deduplicate edges, inflating graph size and skewing traversal
- **`remove_file` incomplete cleanup**: did not clean `file_call_edges`, `file_imports`, or `import_names` — stale data accumulated during incremental updates
- **JS/JSX extension mismatch**: `TypeScriptParser::extensions()` returned `["ts", "tsx"]` but `parser_for()` also matched `"js"` / `"jsx"` — `is_supported()` rejected JS files during file walk. Now `extensions()` includes all 4
- **Config parse error silently swallowed**: malformed `symlens.toml` fell through to default config with no warning — now prints `eprintln!` diagnostic
- **`detect_cycle` used DFS**: same `Vec::pop()` issue as transitive callers — fixed to `VecDeque` BFS

### Changed

- **Parser common module**: extracted `parse_source()`, `node_text()`, `node_span()`, `node_text_first_line()` into `src/parser/helpers.rs` — eliminates ~300 lines of duplicated code across 9 language parsers
- **Indexer parallelism**: replaced `Mutex<ProjectIndex>` + `par_iter().for_each()` with lock-free `par_iter().map().collect()` + sequential merge — removes 5 Mutexes (including 2 counter Mutexes), enables true multi-core parallel parsing
- **README/README_CN**: added real-world benchmark comparison tables (symlens vs grep vs cat) showing token efficiency (6x-100x savings) and information quality differences

## [0.4.1] - 2026-04-14

### Fixed

- **README/README_CN**: MCP tool names now show correct `symlens_` prefix (e.g. `symlens_index` instead of `index`)
- **docs/commands.md**: feature flag description updated from tower-lsp to rmcp

## [0.4.0] - 2026-04-14

### Changed

- **MCP server**: migrated from tower-lsp to official `rmcp` 1.4 SDK — proper MCP protocol handshake, macro-driven tool routing (`#[tool_router]` + `#[tool_handler]`), schemars auto-generated JSON schemas, standard MCP error handling

## [0.3.4] - 2026-04-14

### Added

- **`setup claude-code --global`**: now also registers symlens in `~/.claude/CLAUDE.md` with usage decision guide (when to use symlens vs grep/cat), so the agent knows when and how to trigger the skill
- **`setup claude-code --global --uninstall`**: now also removes the symlens registration from `~/.claude/CLAUDE.md`
- **New template**: `claude_code_register.md` for the CLAUDE.md registration block

## [0.3.3] - 2026-04-14

### Changed

- **Skill templates**: added "When to use grep/cat instead" guidance with decision rule to all 4 agent templates (claude-code section, claude-code skill, openclaw, cursor)

## [0.3.2] - 2026-04-14

### Changed

- **README/README_CN**: added missing commands (export, lines, stats, graph deps), expanded Agent Setup with --global usage
- **Skill templates**: extracted from hardcoded strings to standalone `.md` files under `src/commands/templates/`, loaded via `include_str!()`
- **Skill content updated**: all 4 agent templates now list all 9 languages and full command set (was missing C/C++/Kotlin/Dart, export, lines, stats)

## [0.3.1] - 2026-04-14

### Changed

- **README/README_CN**: added WASM Support section with collapsible API table
- **docs/commands.md**: updated test count (104), added Feature Flags table, WASM API reference, CI description

## [0.3.0] - 2026-04-14

### Added

- **Incremental call graph updates**: watch mode now carries forward call edges and imports for unchanged files, eliminating full graph rebuilds when only a few files change
- **WASM compilation support**: new `wasm` feature flag with `wasm-bindgen` API — `parse_source()`, `extract_calls()`, `extract_imports()`, `build_call_graph()`, `query_callers()`, `query_callees()`, `supported_extensions()`
- **Feature restructuring**: `native` feature (default) for CLI/filesystem deps, `wasm` feature for browser-compatible builds
- **CI WASM check**: GitHub Actions job verifies WASM feature compiles

### Changed

- **Dependencies restructured**: `rayon`, `tantivy`, `ignore`, `notify`, `blake3`, `clap`, `toml` now optional behind `native` feature
- **Watch mode**: now monitors all 9 language extensions (added C, C++, Kotlin, Dart file types)
- **ImportInfo**: now serializable (`Serialize`/`Deserialize`) for incremental import caching

### Testing

- **104 tests** (was 85): added 6 incremental call graph tests + 8 WASM API tests + 5 serialization roundtrip tests

## [0.2.2] - 2026-04-13

### Changed

- **README redesign**: centered header layout, merged install + quick start into 4-line code block, 2x2 command grid, mermaid architecture diagram, collapsible MCP config

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
