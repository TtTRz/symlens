# SymLens — Token-Efficient Code Intelligence

SymLens indexes codebases with tree-sitter and provides token-efficient access to symbols, call graphs, and impact analysis. **Always prefer SymLens over reading entire files.**

## When to Use

- Searching for symbols, functions, or types → `symlens search`
- Understanding a function's signature → `symlens symbol`
- Getting a file or project overview → `symlens outline`
- Finding references → `symlens refs`
- Analyzing impact before refactoring → `symlens graph impact`
- Reviewing changes → `symlens diff`

## Commands

| Instead of... | Use this |
|--------------|----------|
| `grep -r "query"` | `symlens search "query"` |
| `cat file.rs` | `symlens outline file.rs` |
| Reading a whole file for one function | `symlens symbol "file.rs::Foo::bar#method"` |
| `find . -name "*.rs" \| xargs cat` | `symlens outline --project` |
| `grep -r "FnName"` | `symlens refs "FnName"` |

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
