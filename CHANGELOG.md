# Changelog

All notable changes to SymLens will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.12.8] - 2026-07-06

### Performance

- **`index_project_incremental` reads each file at most once.** Previously, when a file's mtime changed AND its content changed (actual edit), `std::fs::read` was called twice: once for the hash check (slow path) and again for parsing (full path). Now the source is read once and reused by both paths. Large-file indexing speeds up; small-file behavior unchanged.

### Changed

- Read-failure semantics tightened: if `fs::read` fails on a file with a previous index, the file is immediately marked failed (previously, the slow path silently skipped and the full path retried, with ms-level TOCTOU race potential).

## [0.12.7] - 2026-07-06

### Changed

- **Tantivy writer heap cap raised from 100MB to 500MB** in `SearchEngine::index_symbols`. Projects with 200K+ symbols (large monorepos) no longer fail to commit the search index due to insufficient heap. Floor remains 15MB (tantivy minimum); projects under ~30K symbols are unaffected.

## [0.12.6] - 2026-07-06

### Changed

- **`std::sync::RwLock` replaced with `parking_lot::RwLock`** in `INDEX_CACHE` (MCP server), `SearchEngine::query_parser`, and daemon `SharedIndex`. Eliminates poison cascade: a thread panic no longer kills the daemon/MCP service for all subsequent requests. Adds `parking_lot = "0.12"` dependency.
- Removed `.read().unwrap()` / `.write().unwrap()` / `.read().expect("...")` from all production lock sites (`parking_lot::RwLock` returns guards directly, no `Result`).
- `daemon::rpc::handle_request` simplified: the `match index.read() { Ok/Err(_) }` poison-error arm is gone (no longer reachable).

### Fixed

- Pre-existing `clippy::field_reassign_with_default` and `clippy::collapsible_if` warnings in `src/index/indexer.rs` resolved.
- Pre-existing `cargo fmt` drift in `src/commands/{callers,index,watch}.rs`, `src/main.rs`, `src/index/indexer.rs` resolved.

## [0.12.5] - 2026-06-18

### Changed

- **Index format bump (v3 → v4).** Older caches are ignored on load, triggering a one-time full re-index.
- `file_mtimes` now stores nanosecond precision (`u128`) instead of seconds (`u64`). `index.bin` grows by ~8 bytes per indexed file.

### Fixed

- Same-second repeated saves (e.g. IDE auto-save jitter) are now correctly detected as file changes by the incremental index fast path. Previously, second-level truncation could cause stale symbols to be reused when a file was saved multiple times within the same second.

## [0.12.4] - 2026-06-18

### Added

- **`--no-ignore` flag** on `index` and `watch` subcommands (and `symlens.toml` `no_ignore` config key). When set, `.gitignore`, `.git/info/exclude`, and global gitignore are not respected, allowing generated or vendored files to be indexed. Default behavior is unchanged (gitignore honored).
- Daemon mode (`watch --serve`) accepts `--no-ignore`; the flag is pinned at daemon startup, restart to change.
- Workspace defaults in `symlens.workspace.toml` accept `no_ignore` (applies to all roots).

### Changed

- `index_project_incremental` and `index_workspace` now take a `&WalkOptions` argument. Existing call sites (`commands/index.rs`, `commands/watch.rs`, `commands/mcp.rs`, `daemon/socket.rs`) are updated to forward CLI flag or default.
- `watch::run` and `daemon::socket::serve_daemon` accept `no_ignore: bool` parameter.

## [0.12.3] - 2026-06-17

### Added

- `IndexResult` and `WorkspaceIndexResult` expose `files_truncated`, `files_failed`, and `failed_paths` fields.
- `stats` command reports `Files truncated` and `Files failed`; when `files_failed > 0`, lists up to 50 failing paths.
- `index` command prints failure count and a truncation warning when applicable (both human and JSON output).

### Changed

- **Index format bump (v2 → v3).** Older `index.bin` caches are ignored on load, triggering a one-time full re-index. Adds `files_truncated`, `files_failed`, `failed_paths` to persisted `ProjectIndex`.
- Walk stage now fully collects supported files before applying `max_files` cap, so truncation count is known. Peak memory increases by ~8MB at 100K files.

### Fixed

- `extract_all` failures no longer silently fall back to symbols-only without surfacing the data loss; the affected file is flagged via internal `degraded` marker and counted in `files_failed`.
- File read errors (permission denied, broken symlink) are now counted and surfaced via `files_failed`/`failed_paths` instead of silently skipped.

## [0.12.2] - 2026-06-17

### Fixed

- **Daemon/watch paths silently ignore `.js/.jsx/.mts/.cts/.vue` files**: `is_source_file()` used a stale `SUPPORTED_EXTENSIONS` const missing these extensions while the registry fast path accepted them — unified by deleting the const and delegating to `GLOBAL_REGISTRY.is_supported()`. Editing a `.js` file under `symlens watch` / daemon mode now correctly triggers incremental re-indexing.

### Changed

- `is_source_file()` no longer references `SUPPORTED_EXTENSIONS` const; the const is removed. The registry is now the single source of truth for supported extensions.

## [0.12.1] - 2026-06-07

### Fixed

- **MCP `symlens_search` BM25 fails in workspace mode**: `storage::open_search` used single-root hash path, never finding workspace tantivy index — added `IndexProvider::open_search()` that resolves correct path per mode
- **MCP `symlens_symbol` source silently ignored in workspace mode**: `single_root()` returns `None` for workspace — now resolves absolute path via `SymbolId` label→hash mapping and `resolve_absolute()`
- **MCP `symlens_outline` returns wrong root's symbols**: `file_keys().find()` matched only relative path, returning first root's symbols for same-named files — now collects symbols from all matching roots and supports `[label]path` display format
- **MCP `symlens_index_workspace` left stale cache**: only invalidated workspace hash key — now also clears each root's input key cache entry
- **MCP `symlens_lines` redundant existence check**: removed dead `exists()` check after `canonicalize()` which already verifies file existence
- **MCP cache double write lock**: merged two sequential `write()` acquisitions into single scope, eliminating cache inconsistency window

## [0.12.0] - 2026-06-06

### Changed

- **Index format split**: `file_identifiers` and `identifier_index` stored in separate `idents.bin` file, loaded lazily only by `refs` — main `index.bin` shrinks from ~2.4MB to ~680KB (72% reduction)
- **All query commands faster**: search/callers/outline no longer deserialize identifier data — restored to v0.9.3 performance levels

### Performance

- **search**: 6.6ms (was 14.8ms, v0.9.3 was 7.1ms)
- **refs**: 9.8ms (was 11.7ms, v0.9.3 was 20.8ms — 2.1× faster than v0.9.3)
- **callers**: 5.8ms (was 11.5ms, v0.9.3 was 6.2ms)
- **outline**: 6.4ms (was 11.9ms, v0.9.3 was 6.5ms)

### Breaking

- **Index format incompatible** with v0.11.x — re-run `symlens index` after upgrade

## [0.11.1] - 2026-06-05

### Fixed

- **MCP `symlens_refs` used runtime parsing**: replaced `rayon` + `LanguageRegistry` + `tree-sitter` runtime parsing with pre-computed `identifier_index` lookup — same optimization as CLI/daemon refs
- **MCP `symlens_callers`/`symlens_callees` lacked enrichment**: added `find_symbol()` enrichment returning `{name, file, line, kind, signature}` per item (was bare name strings)
- **MCP query tools now use `IndexProvider`**: unified cache and query path, enabling workspace mode support across all tools

### Changed

- **MCP server**: replaced dual `SINGLE_CACHE`/`WORKSPACE_CACHE` with unified `INDEX_CACHE<HashMap<String, Arc<IndexProvider>>>`, simplifying cache management and invalidation

### Testing

- **237 tests** (was 228): added 4 MCP unit tests (cache key, invalidation, error format) + 5 integration tests (refs empty index, callers fallback, stats, search, outline through IndexProvider)

## [0.11.0] - 2026-06-05

### Added

- **Daemon mode** (`symlens watch --serve`): long-running process that keeps the index in memory and serves queries via Unix socket — eliminates per-query index deserialization overhead
- **`--daemon` global CLI flag**: routes query commands (search, refs, callers, callees, outline, symbol, impact, status) through the running daemon instead of loading from disk
- **JSON-RPC protocol**: line-delimited JSON over Unix socket (`~/.symlens/daemon/{hash}.sock`) with 8 methods matching CLI commands
- **`SharedIndex`** (`Arc<RwLock<IndexProvider>>`): watcher thread takes write lock for incremental reindex, socket threads take read lock for concurrent queries
- **`IndexProvider::from_single`/`from_workspace` constructors**: create providers from pre-built indexes without disk I/O
- **`IndexProvider::socket_hash()`**: returns hash for socket path naming

### Changed

- **CLI flags**: added `--daemon` global flag and `--serve` flag on `watch` command
- **main.rs dispatch**: daemon routing check before normal command dispatch; `--serve` delegates to daemon server
- **`refs` RPC handler**: supports `kind` parameter filtering (call/type/import/field/constructor), matching CLI `--kind` behavior
- **`outline` RPC handler**: workspace mode file lookup fixed — resolves correct `FileKey` across all roots instead of using empty `root_id`
- **Daemon watcher**: passes original `--root` to workspace re-index instead of `None`; uses `Arc<AtomicBool>` shutdown flag instead of global static; extracts `prev_index` from write lock guard (no disk reload)
- **Daemon client**: computes socket hash directly via blake3 instead of loading full index (~7ms saved)
- **Daemon connections**: supports multiple requests per connection (keep-alive)
- **`SUPPORTED_EXTENSIONS` extracted to `parser/traits.rs`**: shared constant + `is_source_file()` helper used by both `watch.rs` and daemon watcher (eliminates duplicated extension list)

### Fixed

- **Large response truncation (>8KB)**: accepted connections inherited nonblocking mode from the listener — `write_all` returned `WouldBlock` instead of writing the full response. Fixed by setting accepted streams back to blocking mode, and using `write_all` + explicit newline instead of `writeln!`
- **100ms accept latency**: accept loop polled every 100ms, adding up to 100ms fixed overhead to every daemon query. Reduced to 5ms poll interval — daemon queries now ~6ms (was ~100ms, **1.4-1.5× faster than CLI**)
- **`ProjectIndex` / `CallGraph`**: added `Clone` derive (needed for daemon watcher `prev_index` extraction without disk I/O)
- **MCP `symlens_refs` used runtime parsing**: replaced `rayon` + `LanguageRegistry` + `tree-sitter` runtime parsing with pre-computed `identifier_index` lookup — same optimization as CLI/daemon refs
- **MCP `symlens_callers`/`symlens_callees` lacked enrichment**: added `find_symbol()` enrichment returning `{name, file, line, kind, signature}` per item (was bare name strings)
- **MCP query tools now use `IndexProvider`**: unified cache and query path, enabling workspace mode support across all tools

### Changed

- **MCP server**: replaced dual `SINGLE_CACHE`/`WORKSPACE_CACHE` with unified `INDEX_CACHE<HashMap<String, Arc<IndexProvider>>>`, simplifying cache management and invalidation

### Architecture

- `src/daemon/mod.rs` — module root, `SharedIndex` type, socket path helpers
- `src/daemon/rpc.rs` — JSON-RPC protocol, 8 method handlers building JSON from `IndexProvider`
- `src/daemon/socket.rs` — server lifecycle (load → bind → watch → accept → shutdown), watcher thread
- `src/daemon/client.rs` — client socket connection, CLI-to-RPC command mapping
- Pure `std::thread`, no tokio; no new crate dependencies
- No changes to existing `watch.rs` — daemon has independent watch loop

### Testing

- **237 tests** (was 213): added 15 daemon tests + 9 MCP parity tests covering cache key determinism, invalidation, refs on empty index, callers enrichment fallback, stats/search/outline through IndexProvider

## [0.10.0] - 2026-06-05

### Added

- **Pre-computed identifier positions in index**: all 9 parsers now extract identifier references during the single-parse `extract_all` pass — `file_identifiers` (per-file identifier list) and `identifier_index` (name → files) are serialized into the index
- **`IdentifierRef` enhanced**: added `name` field, `Clone`, `Serialize`, `Deserialize` derives — each identifier carries its name for index-time name→file mapping
- **Index v2 format**: caches (`search_cache`, `name_to_idx`, `short_name_idx`) now serialized — v1 indexes auto-upgrade via `rebuild_index()`, v2+ only rebuilds petgraph DiGraph lazily

### Changed

- **`refs` command rewritten to pure index lookup**: eliminated runtime tree-sitter parsing, rayon threads, and `LanguageRegistry` — now uses `identifier_index` → `file_identifiers` HashMap chain. **3.5× faster** (38ms → 11ms measured with hyperfine)
- **Serialized search cache**: `search_cache` in `ProjectIndex`/`WorkspaceIndex` no longer `#[serde(skip)]` — zero rebuild time on index load
- **Serialized call graph name index**: `name_to_idx` and `short_name_idx` in `CallGraph` no longer `#[serde(skip)]` — only petgraph DiGraph is rebuilt lazily via `rebuild_digraph()`
- **Index size trade-off**: index grew ~4× (530KB → 2.2MB on the symlens codebase) due to serialized caches and identifier data; acceptable given refs speedup and single-index design
- **Deduplication**: `identifier_index` uses `HashSet` to ensure one file path per unique identifier name, preventing N×N result inflation

### Testing

- **213 tests** (was 207): added 6 tests — `parsed_output_includes_identifiers`, `typescript_extract_all_identifiers`, `index_contains_file_identifiers`, `workspace_contains_identifiers`, `index_serde_roundtrip_preserves_caches`, `refs_uses_precomputed_identifiers`

## [0.9.3] - 2026-06-04

### Changed

- **Workspace display labels**: SymbolId prefixes now use directory names (e.g., `[audio]src/main.rs`) instead of blake3 hashes (e.g., `[a1b2c3d4]`) for human-readable workspace output — call graph nodes, search results, callers/callees, outline, diff, and watch all show readable names
- **Clippy clean**: fixed pre-existing `map_or` → `is_ok_and` and collapsible `if` warnings in `helpers.rs` and `deps.rs`
- **WASM feature gate**: `blake3` and `Config` gated behind `native` feature — `wasm` feature now includes `blake3` dep and compiles cleanly without native-only modules

### Fixed

- **`remove_root` mismatch**: SymbolId filtering in `WorkspaceIndex::remove_root` compared hash-based `root_id` against label-based SymbolId prefix — now correctly resolves label before matching
- **`resolve_absolute` label fallback**: `WorkspaceIndex::resolve_absolute` now matches by both hash `id` and directory `label`, fixing path resolution for commands that receive label-prefixed SymbolIds
- **`outline` hash display**: outline JSON output showed `[hash]` file prefixes — now uses `IndexProvider::file_display()` to resolve labels
- **`watch` hash display**: workspace watch logs showed `[hash]` prefixes — now shows directory names
- **CI clippy failures**: 10 clippy lint errors in test file (`cloned_ref_to_slice_refs`, `len_zero`, `unused_variables`)
- **CI WASM check failure**: `blake3` and `Config` unresolved in `wasm` feature — added feature gates

### Testing

- **207 tests** (was 201): added 6 workspace label tests covering SymbolId prefix, resolve by label, remove by label, call edge prefix, RootInfo label edge cases, and children remap

## [0.9.1] - 2026-06-04

### Fixed

- **BM25 adaptive fuzzy**: exact query tried first (fast path), fuzzy search only activates when exact match returns no results — eliminates Levenshtein automata overhead for precise queries
- **UTF-8 truncation threshold**: `callers`/`callees` signature truncation now uses `chars().count()` instead of byte `.len()` to match `truncate_str` semantics
- **Search pagination order**: `--offset` now applied before `--limit` truncation, preventing data loss on paginated results
- **JSON deps determinism**: `graph deps --json` modules list now uses `BTreeSet` for consistent output order
- **Benchmark accuracy**: `OnceLock` shared index across benchmark groups (was re-indexing per group); deterministic `hub_node()` selection instead of random `first()`
- **Documentation**: updated `docs/commands.md` with current benchmark data (828 symbols, 28 benchmarks), MCP tools (12), new CLI flags, `.mts`/`.cts` extensions

## [0.9.0] - 2026-06-03

### Added

- **Fuzzy BM25 search**: `query_parser.set_field_fuzzy()` enabled for `name`, `qualified_name`, and `signature` fields — handles typos and partial matches via tantivy fuzzy queries
- **Dependency cycle detection**: `DepsGraph::has_cycle_from()` and `DepsGraph::detect_cycles()` — BFS-based cycle detection with `graph deps` output showing detected cycles (text + JSON)
- **Pagination offset**: `--offset` flag on `search` and `refs` commands for paginating through large result sets
- **`--verbose/-v` global flag**: enables `[verbose]` diagnostic output (index timing, file counts) across commands
- **Shared parser helpers**: `extract_signature`, `extract_doc_comment`, `find_child_by_kind`, `last_child_by_kind`, `find_child_text_by_kind`, `node_text_eq`, `node_text_first_line` extracted into `parser/helpers.rs` — used by all 9 language parsers, eliminating ~207 lines of duplication

### Changed

- **Callers/Callees enhanced output**: shows file path, line number, and signature for each caller/callee (previously only showed symbol name); `color_on` parameter threaded through command chain
- **JSON search output**: wrapped in `{ "query": ..., "results": [...], "count": N }` envelope for programmatic consumption
- **`IndexProvider` trait**: private `trait Index` with 8 methods, `as_index()` helper — eliminates repeated `match` blocks in all 14 methods
- **Refs parallel scanning**: `thread_local!` for per-thread `LanguageRegistry` in rayon parallel closures; verbose timing output
- **Search verbose timing**: `search` command shows elapsed time when `SYMLENS_VERBOSE` is set

### Testing

- **201 tests** (was 156): added 45 new tests covering cycle detection (6), UTF-8 truncation (7), `Cow<str>` color output (7), language detection including `.mts`/`.cts` (11), and parser helpers (14)

## [0.8.1] - 2026-06-02

### Fixed

- **UTF-8 panic in diff/outline**: `&sig[..N]` byte-level slicing could panic on multi-byte characters — replaced with `char_indices()`-based safe truncation (`truncate_str`)
- **TypeScript grammar for `.jsx`**: `.jsx` files now use TSX grammar instead of plain TypeScript grammar, producing correct AST nodes for JSX syntax
- **`.mts`/`.cts` support**: TypeScript parser and language registry now recognize `.mts` and `.cts` extensions; `detect_language()` maps them to `"typescript"`
- **`outline --file` in workspace mode**: hardcoded empty `root_id` caused silent failure in workspace mode — now resolves the correct `root_id` by searching the workspace index

### Changed

- **Zero-allocation identifier comparison**: added `node_text_eq()` to avoid heap allocation on every identifier match in `collect_*_ids` across all 9 parsers
- **Incremental index clone reduction**: eliminated double-clone of `call_edges` and `imports` in `copy_prev_data` and the full-parse path; removed redundant `FileResult` fields (`call_edges`, `imports`)
- **Tantivy index reuse**: `save()`/`save_workspace()` now opens the existing tantivy index instead of destroying and recreating the filesystem directory; `index_symbols()` already clears and re-adds documents
- **Tokenizer optimizations**: eliminated `chars().collect::<Vec<char>>()` allocation and double lowercase conversion (`build lowercase → .to_lowercase()` → single pass); added single-char token filter to reduce index noise
- **CallGraph partial-match acceleration**: added `short_name_idx` HashMap (short name → node indices) for O(1) short-name lookups instead of O(N) linear scan; `callers_partial`/`callees_partial` use fast path with fallback
- **Color output zero-alloc**: `color.rs` functions return `Cow<'_, str>` instead of `String` — `Cow::Borrowed` when color is off, zero heap allocation
- **Workspace hash**: `compute_workspace_hash` uses `blake3::Hasher::update()` loop instead of intermediate `String` concatenation
- **Model dedup**: `kind_priority()` and `detect_language()` extracted from `project.rs`/`workspace.rs` into shared `model/mod.rs`

### Testing

- **156 tests** passing (unchanged count; all existing tests pass with the refactored code)

## [0.8.0] - 2026-04-17

### Added

- **Workspace mode (`--workspace`)**: unified indexing across multiple project roots for cross-project symbol search, call graph traversal, and impact analysis. All commands support `--workspace` flag; roots declared in `symlens.workspace.toml`
- **`WorkspaceIndex`**: unified in-memory index merging multiple `ProjectIndex` instances with `[root_id]` prefixes for symbol disambiguation
- **`FileKey` + `RootInfo`**: workspace-scoped file keys and root metadata (blake3-derived stable ids)
- **`IndexProvider`**: abstraction layer — `Single(ProjectIndex)` for backward compat, `Workspace(WorkspaceIndex)` for multi-root mode
- **`symlens_index_workspace` MCP tool**: index a workspace with multiple roots via MCP protocol; includes caching (`WORKSPACE_CACHE`)
- **Per-root incremental indexing**: each root in a workspace has its own cache; only changed roots are re-indexed on `symlens index --workspace`
- **`symlens.workspace.toml` config**: declare workspace roots with optional path aliases; auto-detected by `--workspace` flag

### Changed

- **`SymbolId` format**: extended to support `[root_id]path::Name#kind` prefix; empty `root_id` falls back to original format (backward compatible)
- **`Config`**: added `Clone` derive; added `WorkspaceConfig` / `WorkspaceSection` structs
- **All 12 commands**: adapted to use `IndexProvider` for transparent single-root / workspace dispatch
- **`graph::impact`**: `extract_module()` skips `[root_id]` prefix before splitting on `::`

### Testing

- **156 tests** (was 138): added 18 workspace mode tests covering SymbolId prefixes, FileKey, RootInfo, WorkspaceIndex insert/search/remove, cross-root call graph, serialization roundtrip, and parent-child remap

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
