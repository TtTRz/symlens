---
description: SymLens code intelligence — use symlens CLI for token-efficient code search, symbol lookup, refs, and call graph analysis
globs:
alwaysApply: true
---

# SymLens Code Intelligence

This project has `symlens` installed for token-efficient code navigation.

## Rules

1. **Prefer `symlens` over reading entire files** to save tokens:
   - `symlens search "<query>"` instead of `grep -r`
   - `symlens outline <file>` instead of `cat <file>`
   - `symlens symbol "<id>"` instead of reading the whole file
   - `symlens symbol "<id>" --source` only when you need full implementation
   - `symlens outline --project` instead of `find` + `cat`
   - `symlens refs "<name>"` instead of `grep -r "<name>"`

2. **Before refactoring**: ALWAYS run `symlens graph impact "<symbol>"` first to check blast radius.

3. **Before reviewing a PR**: run `symlens diff --from main --to HEAD` to see changed symbols.

4. **Git history**: use `symlens blame "<name>"` to check who last modified a symbol.

5. Run `symlens index` if you get "index not found" errors.

## Quick Reference

| Task | Command |
|------|---------|
| Search symbols | `symlens search "<query>"` |
| Function signature | `symlens symbol "<id>"` |
| Full source | `symlens symbol "<id>" --source` |
| File outline | `symlens outline <file>` |
| Project overview | `symlens outline --project` |
| Find references | `symlens refs "<name>"` |
| Callers | `symlens callers "<name>"` |
| Callees | `symlens callees "<name>"` |
| Impact analysis | `symlens graph impact "<name>"` |
| Dependency graph | `symlens graph deps [--fmt mermaid]` |
| Call path | `symlens graph path <A> <B>` |
| Source by lines | `symlens lines <file> <start> <end>` |
| Git blame | `symlens blame "<name>"` |
| Changed symbols | `symlens diff --from <ref> --to <ref>` |
| Index stats | `symlens stats` |
| Export index | `symlens export [--format json]` |

## Supported Languages

Rust · TypeScript · Python · Go · Swift · Dart · C · C++ · Kotlin
