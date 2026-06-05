# Command Reference

## Global Flags

| Flag | Description |
|------|-------------|
| `--root <path>` | Project root (default: auto-detect via `.git`) |
| `--json` | Output as JSON (all commands) |
| `--no-color` | Disable colored output |
| `--verbose / -v` | Show diagnostic output (timing, file counts) |
| `--workspace` | Enable workspace mode (multi-root indexing) |
| `--daemon` | Route query through running daemon (requires `watch --serve`) |

## Commands

### Indexing

| Command | Description | Token Cost |
|---------|-------------|------------|
| `symlens index [path]` | Index the project (parallel, incremental) | — |
| `symlens index --force` | Force re-index, ignore cache | — |
| `symlens watch` | Auto-update index on file changes | — |
| `symlens watch --serve` | Start daemon — keep index in memory, serve queries via Unix socket | — |
| `symlens init` | Generate default `symlens.toml` config | — |
| `symlens doctor` | Diagnose index health, cache status, languages | — |
| `symlens stats` | Show index statistics | ~50 |

### Symbol Lookup

| Command | Description | Token Cost |
|---------|-------------|------------|
| `symlens search <query>` | BM25 search by name, signature, or docs (fuzzy-tolerant) | ~40/result |
| `symlens search <query> --offset 20 --limit 10` | Paginate search results | ~40/result |
| `symlens symbol <id>` | Get signature + doc comment | ~60 |
| `symlens symbol <id> --source` | Get full source code | ~500-2000 |
| `symlens outline <file>` | File symbol tree | ~50/file |
| `symlens outline --project` | Project-wide structure overview | ~200 |
| `symlens lines <file> <start> <end>` | Get source by line range | varies |

### References & Call Graph

| Command | Description | Token Cost |
|---------|-------------|------------|
| `symlens refs <name>` | Find references (AST-level, import-aware, parallel) | ~30/ref |
| `symlens refs <name> --offset 10 --limit 20` | Paginate reference results | ~30/ref |
| `symlens callers <name>` | Who calls this symbol | ~20/caller |
| `symlens callees <name>` | What this symbol calls | ~20/callee |
| `symlens graph impact <name>` | Blast radius analysis (risk score, cycle detection) | ~200 |
| `symlens graph deps [--fmt mermaid]` | Module dependency graph (with cycle detection) | ~150 |
| `symlens graph path <from> <to>` | Shortest call path between two symbols | ~50 |

### Git Integration

| Command | Description | Token Cost |
|---------|-------------|------------|
| `symlens blame <name>` | Git blame for a symbol's line range | ~100 |
| `symlens diff --from <ref> --to <ref>` | Changed symbols between git refs | ~50/change |

### Agent & Tooling

| Command | Description |
|---------|-------------|
| `symlens setup <agent>` | Install into AI agent (claude-code, cursor, openclaw) |
| `symlens setup --uninstall <agent>` | Remove SymLens integration |
| `symlens setup --all [--global]` | Install into all agents |
| `symlens setup --list` | List supported agents |
| `symlens completions <shell>` | Generate shell completions (bash, zsh, fish) |
| `symlens mcp` | Start MCP server (requires `--features mcp`) |
| `symlens export [--format json]` | Export index as JSON |

## Language Support

9 languages with full symbol extraction, call graph, reference finding, and import tracking:

| Language | Extensions | Symbol Types |
|----------|-----------|-------------|
| **Rust** | `.rs` | fn, struct, enum, trait, impl, const, type, macro |
| **TypeScript** | `.ts` `.tsx` `.js` `.jsx` `.mts` `.cts` | function, class, interface, type, enum, const |
| **Python** | `.py` | function, class, method, variable |
| **Go** | `.go` | func, method, struct, interface, type, const, var |
| **Swift** | `.swift` | func, class, struct, enum, protocol |
| **Dart** | `.dart` | class, mixin, enum, extension, typedef, function, method |
| **C** | `.c` `.h` | function, struct, enum, typedef, macro |
| **C++** | `.cpp` `.cc` `.hpp` | function, class, struct, enum, namespace, method |
| **Kotlin** | `.kt` `.kts` | function, class, interface, enum, object, property |

## MCP Tools

When running as an MCP server (`symlens mcp`), 12 tools are available:

| Tool | Description |
|------|-------------|
| `symlens_index` | Index a project, returns symbol count and timing |
| `symlens_index_workspace` | Index a workspace with multiple roots |
| `symlens_search` | BM25 search with optional kind filter (fuzzy-tolerant) |
| `symlens_symbol` | Get symbol details by ID, optional source code |
| `symlens_outline` | File or project outline |
| `symlens_refs` | Find references to a symbol |
| `symlens_impact` | Blast radius analysis with risk score |
| `symlens_callers` | Direct callers of a symbol |
| `symlens_callees` | Direct callees of a symbol |
| `symlens_lines` | Get source by line range |
| `symlens_diff` | Changed symbols between git refs |
| `symlens_stats` | Index statistics |

## Performance Benchmarks

Measured with [criterion](https://github.com/bheisler/criterion.rs) on the SymLens codebase (58 files, 828 symbols):

| Operation | Time | Notes |
|-----------|------|-------|
| Full project index | 20 ms | Parallel via rayon |
| Incremental index (no changes) | <1 ms | blake3 content hash |
| Search (exact name) | 149 µs | Pre-computed lowercase cache |
| Search (miss) | 191 µs | Scans all symbols |
| Callers query | 96 ns | Cached petgraph DiGraph |
| Callees query | 28 ns | HashMap lookup |
| Transitive callers (depth 3) | 1.08 µs | BFS on cached graph |
| Find call path | 577 µs | Bidirectional BFS |
| Impact analysis | 106 µs | Callers + callees + transitive + cycle |
| Parse single Rust file | 521 µs | tree-sitter |
| Parse single Python file | 45 µs | tree-sitter |
| Parse single Go file | 82 µs | tree-sitter |
| Dep cycle check (100 nodes) | 330 ns | BFS traversal |
| Detect all cycles (100 nodes) | 261 µs | Per-node BFS |
| bincode encode | 182 µs | 828 symbols |
| bincode decode | 728 µs | Rebuild search cache |
| Build call graph | 405 µs | petgraph construction |
| Registry lookup | 16 ns | Extension → parser HashMap |

### Daemon vs CLI (hyperfine, 20 runs)

| Query | CLI | Daemon | Daemon speedup |
|-------|-----|--------|----------------|
| search | 9.6 ms | 6.2 ms | 1.6× |
| refs | 8.6 ms | 6.2 ms | 1.4× |
| callers | 8.8 ms | 6.2 ms | 1.4× |
| impact | 10.8 ms | 7.0 ms | 1.5× |

## Comparison

| | SymLens | SymLens (daemon) | LSP (Serena) | pitlane-mcp | Aider repo-map |
|---|---------|-------------------|-------------|------------|----------------|
| Cold start | 50 ms | 6 ms (warm) | 3-10 s | Fast | Rebuilds each time |
| Dependencies | None (single binary) | None | Python + LSP servers | None | Python |
| Call graph | Yes | Yes | No | Yes | No |
| Impact analysis | Yes | Yes | No | No | No |
| BM25 search | Yes | Yes | No | Yes | No |
| Git blame/diff | Yes | Yes | No | No | No |
| MCP server | Yes | — | Yes | Yes | No |
| Semantic precision | ~90% (syntax) | ~90% (syntax) | ~99% (semantic) | ~70% | N/A |
| Refactoring | No (read-only) | No (read-only) | Yes | No | No |

**When to use SymLens:** You want fast, zero-dependency code intelligence for AI agents.
**When to use an LSP:** You need semantic accuracy (rename, go-to-definition) and don't mind the startup cost.

## CI/CD

- **CI** (`ci.yml`): cargo check, test (Linux + macOS), clippy, rustfmt, WASM check — every push/PR to `master`
- **Release** (`release.yml`): cross-platform builds (Linux x86/ARM, macOS x86/ARM) + GitHub Release — triggered by `v*` tags

## Project Stats

- Rust 2024 edition, minimum rustc 1.92
- 228 tests (15 daemon + 213 other), 0 clippy warnings
- 28 benchmarks across 7 groups
- 21 commands, 9 languages, 12 MCP tools
- WASM build support via `--features wasm`

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | ✅ | CLI, filesystem, BM25 search, watch mode, daemon mode, incremental indexing |
| `mcp` | — | MCP server via rmcp (includes `native`) |
| `wasm` | — | WASM API via wasm-bindgen (parsing, call graphs, queries) |

## WASM API

Build with `cargo build --target wasm32-wasip1 --no-default-features --features wasm`.

| Function | Description |
|----------|-------------|
| `parse_source(filename, source)` | Parse source code → symbols JSON |
| `extract_calls(filename, source)` | Extract call edges from source |
| `extract_imports(filename, source)` | Extract import statements |
| `build_call_graph(edges_json)` | Build call graph from edge pairs |
| `query_callers(graph_json, symbol)` | Query direct callers |
| `query_callees(graph_json, symbol)` | Query direct callees |
| `supported_extensions()` | List all supported file extensions |

**Supported in WASM:** symbol parsing (9 languages), call graph building/querying, import extraction.
**Not supported in WASM:** filesystem indexing, BM25 search, watch mode, MCP server.
