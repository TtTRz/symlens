# 🌲 CodeLens

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
| `codelens watch` | Auto-update index on file changes | — |
| `codelens stats` | Index statistics | ~50 |

## Language Support

| Language | Symbols | Calls | Refs | Imports |
|----------|---------|-------|------|---------|
| **Rust** | ✅ fn, struct, enum, trait, impl, const, type, macro | ✅ | ✅ v3 | ✅ |
| **TypeScript** | ✅ function, class, interface, type, enum, const | — | — | — |
| **Python** | ✅ function, class, method, docstring | — | ✅ | — |
| **Swift** | ✅ func, class, struct, enum, protocol | — | ✅ | — |
| **Go** | ✅ func, method, struct, interface, type, const, var | ✅ | ✅ | ✅ |

## Claude Code Integration

Add this to your project's `CLAUDE.md`:

```markdown
## Code Navigation

This project has `codelens` installed for token-efficient code search.

**IMPORTANT: Prefer `codelens` over reading entire files:**

1. `codelens search "<query>"` instead of `grep -r`
2. `codelens outline <file>` instead of `cat <file>`
3. `codelens symbol "<id>"` instead of reading the whole file
4. `codelens symbol "<id>" --source` only when you need the implementation
5. `codelens outline --project` instead of `find` + `cat`
6. `codelens refs "<name>"` instead of `grep -r "<name>"`
7. **Before refactoring**: ALWAYS run `codelens graph impact "<symbol>"` first

Run `codelens index` if you get "index not found" errors.
```

## Architecture

```
tree-sitter AST → symbol extraction → tantivy BM25 index
                                    → petgraph call graph
                                    → bincode persistence
                                    → import tracking (refs v3)
```

- **tree-sitter**: Parse 5 languages into ASTs, extract symbols
- **tantivy**: Full-text BM25 search with custom camelCase tokenizer
- **petgraph**: Directed call graph for callers/callees/impact analysis
- **bincode**: Fast binary serialization for index persistence
- **rayon**: Parallel file parsing
- **notify**: File system watching for auto-update

## Performance

| Operation | Time |
|-----------|------|
| Index 1000 files | < 1s |
| Search (BM25) | < 1ms |
| Symbol lookup | < 0.1ms |
| Index load from disk | < 50ms |

## vs Other Tools

| | CodeLens | Serena (LSP) | pitlane-mcp | Aider repo-map |
|---|---------|-------------|------------|----------------|
| Speed | ⚡ 50ms cold start | 🐢 3-10s | ⚡ Fast | 🐢 Rebuilds each time |
| Dependencies | None (single binary) | Python + LSP servers | None | Python |
| Call graph | ✅ | ❌ | ✅ | ❌ |
| Import tracking | ✅ | N/A (LSP) | ❌ | ❌ |
| BM25 search | ✅ | ❌ | ✅ | ❌ |
| Impact analysis | ✅ | ❌ | ❌ | ❌ |
| Refactoring | ❌ (read-only) | ✅ rename/move | ❌ | ❌ |
| Precision | ~90% (syntax) | ~99% (semantic) | ~70% | N/A |

## License

MIT
