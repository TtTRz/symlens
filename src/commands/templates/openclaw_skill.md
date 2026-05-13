---
name: symlens
description: Token-efficient code intelligence — search, outline, refs, call graph, impact analysis, git blame/diff
version: 0.1.0
tools:
  - symlens
---

# SymLens — Code Intelligence Skill

SymLens indexes codebases with tree-sitter and provides token-efficient access to symbols, call graphs, and impact analysis.

## Available Commands

| Command | What it does |
|---------|-------------|
| `symlens index` | Index the project (run once) |
| `symlens search "<query>"` | BM25 full-text symbol search |
| `symlens symbol "<id>"` | Get function signature + docs |
| `symlens symbol "<id>" --source` | Get full source code |
| `symlens outline <file>` | File symbol tree |
| `symlens outline --project` | Project structure overview |
| `symlens refs "<name>"` | Find all references (AST-level) |
| `symlens callers "<name>"` | Who calls this symbol |
| `symlens callees "<name>"` | What this symbol calls |
| `symlens graph impact "<name>"` | Blast radius / impact analysis |
| `symlens graph deps [--fmt mermaid]` | Module dependency graph |
| `symlens graph path <A> <B>` | Call path between two symbols |
| `symlens lines <file> <start> <end>` | Get source by line range |
| `symlens blame "<name>"` | Git blame for a symbol |
| `symlens diff --from <ref> --to <ref>` | Changed symbols between commits |
| `symlens stats` | Index statistics |
| `symlens export [--format json]` | Export index as JSON |
| `symlens watch` | Auto-update index on file changes |
| `symlens watch --serve` | Daemon mode: keep index in memory (~6ms/query) |
| `symlens --daemon <command>` | Route any query through the daemon |

## Usage Guidelines

1. **Always prefer `symlens` over reading entire files** — it saves tokens.
2. Run `symlens index` first if the project hasn't been indexed.
3. Use `symlens search` instead of `grep -r` for symbol search.
4. Use `symlens outline --project` instead of `find` + `cat` for project overview.
5. **Before refactoring**: run `symlens graph impact "<symbol>"` to check blast radius.
6. **Before reviewing changes**: run `symlens diff --from main --to HEAD`.
7. Use `symlens symbol "<id>"` to get just the signature (~60 tokens) instead of the whole file (~4000 tokens).
8. For high-frequency queries, use daemon mode: `symlens watch --serve &` then `symlens --daemon <command>` (1.5× faster).

## When to Use `grep`/`cat` Instead

Use grep/cat when the target is **not** a code symbol in a supported language:

- **Non-code files**: `.md`, `.toml`, `.yml`, `.json`, `.env`, logs, configs
- **String literals / comments / magic numbers**: symlens only indexes symbols (functions, types, methods)
- **Unsupported languages**: anything outside the 10 languages listed above
- **Regex pattern matching**: arbitrary text patterns across file contents

> **Decision rule**: Is the target a symbol (function/struct/trait/class/method) in a supported language?
> → Yes: use `symlens` · No: use `grep`/`cat`

## Supported Languages

Rust · TypeScript · Python · Go · Swift · Dart · C · C++ · Kotlin · Vue — full support for symbols, calls, refs, and imports.
