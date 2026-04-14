## Code Navigation (SymLens)

This project has `symlens` installed for token-efficient code intelligence.

### Use `symlens` when (saves 60-90% tokens):

1. `symlens search "<query>"` instead of `grep -r` — for finding functions, types, methods
2. `symlens outline <file>` instead of `cat <file>` — for understanding file structure
3. `symlens symbol "<id>"` instead of reading the whole file — for function signatures
4. `symlens symbol "<id>" --source` only when you need the implementation
5. `symlens outline --project` instead of `find` + `cat` — for project overview
6. `symlens refs "<name>"` instead of `grep -r "<name>"` — for finding symbol references
7. `symlens callers/callees "<name>"` — for call relationship analysis
8. `symlens blame "<name>"` — for git blame on a specific symbol
9. **Before refactoring**: ALWAYS run `symlens graph impact "<symbol>"` first
10. **Before reviewing a PR**: run `symlens diff --from main --to HEAD`

### Use `grep`/`cat` when:

- Searching **non-code files** (`.md`, `.toml`, `.yml`, `.json`, `.env`, logs)
- Searching **string literals, comments, or magic numbers** inside code
- Working with **unsupported languages** (symlens supports: Rust, TypeScript, Python, Go, Swift, Dart, C, C++, Kotlin)
- Need **regex pattern matching** across file contents (not symbol names)
- Reading **config/data files** that have no symbols to parse

### Decision rule

> **Is the target a symbol (function/struct/trait/class/method) in a supported language?**
> → Yes: use `symlens` · No: use `grep`/`cat`

Run `symlens index` if you get "index not found" errors.
