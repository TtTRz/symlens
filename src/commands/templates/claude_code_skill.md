# SymLens — Token-Efficient Code Intelligence

SymLens indexes codebases with tree-sitter and provides token-efficient access to symbols, call graphs, and impact analysis. **Always prefer SymLens over reading entire files.**

## When to Use `symlens` (saves 60-90% tokens)

| Instead of... | Use this | Why |
|--------------|----------|-----|
| `grep -r "query"` | `symlens search "query"` | BM25 symbol search, no noise |
| `cat file.rs` | `symlens outline file.rs` | Structure only, ~50 tokens vs ~4000 |
| Reading a whole file for one function | `symlens symbol "file.rs::Foo::bar#method"` | Signature ~60 tokens |
| `find . -name "*.rs" \| xargs cat` | `symlens outline --project` | Project overview ~200 tokens |
| `grep -r "FnName"` | `symlens refs "FnName"` | AST-level, import-aware |
| (no grep equivalent) | `symlens callers/callees "name"` | Call relationship analysis |
| (no grep equivalent) | `symlens graph impact "name"` | Blast radius before refactoring |

## When to Use `grep`/`cat` Instead

- **Non-code files**: `.md`, `.toml`, `.yml`, `.json`, `.env`, logs, configs
- **String literals / comments / magic numbers**: symlens only indexes symbols, not string content
- **Unsupported languages**: anything outside Rust, TypeScript, Python, Go, Swift, Dart, C, C++, Kotlin
- **Regex pattern matching**: when you need arbitrary text patterns, not symbol names
- **Config/data files**: no AST structure to parse

### Decision Rule

> **Is the target a symbol (function/struct/trait/class/method) in a supported language?**
> → **Yes**: use `symlens` · **No**: use `grep`/`cat`

## Full Command Reference

```
symlens index                              # Index project (run first)
symlens search "<query>"                   # BM25 symbol search (~40 tokens/result)
symlens symbol "<id>"                      # Signature + docs (~60 tokens)
symlens symbol "<id>" --source             # Full source (~500-2000 tokens)
symlens outline <file>                     # File symbol tree (~50 tokens/file)
symlens outline --project                  # Project overview (~200 tokens)
symlens refs "<name>"                      # Find references (~30 tokens/ref)
symlens callers "<name>"                   # Who calls this
symlens callees "<name>"                   # What this calls
symlens graph impact "<name>"              # Blast radius analysis
symlens graph deps [--fmt mermaid]         # Module dependency graph
symlens graph path <A> <B>                 # Call path between symbols
symlens lines <file> <start> <end>         # Source by line range
symlens blame "<name>"                     # Git blame for a symbol
symlens diff --from <ref> --to <ref>       # Changed symbols between refs
symlens stats                              # Index statistics
symlens export [--format json]             # Export index as JSON
symlens watch                              # Auto-update index
```

## Critical Rules

1. **ALWAYS** run `symlens graph impact "<symbol>"` before refactoring
2. **ALWAYS** run `symlens diff --from main --to HEAD` before reviewing a PR
3. Run `symlens index` if you get "index not found" errors
4. Use `--source` flag only when you actually need the implementation, not just the signature

## Supported Languages

Rust · TypeScript · Python · Go · Swift · Dart · C · C++ · Kotlin — full support for symbols, calls, refs, and imports.
