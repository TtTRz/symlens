# symlens
- **symlens** — token-efficient code intelligence (search, outline, refs, call graph, impact analysis)
- **When to use**: searching symbols/functions/types, getting file outlines, finding references, call relationship analysis, impact analysis before refactoring, symbol-level git blame/diff — in Rust, TypeScript, Python, Go, Swift, Dart, C, C++, Kotlin projects
- **When NOT to use** (fall back to grep/cat): non-code files (.md/.toml/.yml/.json/.env/logs), string literals/comments/magic numbers, unsupported languages, regex pattern matching
- **Decision rule**: Is the target a symbol (function/struct/trait/class/method) in a supported language? → Yes: `symlens` · No: `grep`/`cat`
- Trigger: `/symlens`
When the user types `/symlens`, invoke the Skill tool with `skill: "symlens"` before doing anything else.
