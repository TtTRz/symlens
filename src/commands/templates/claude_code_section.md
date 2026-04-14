## Code Navigation (SymLens)

This project has `symlens` installed for token-efficient code intelligence.

**IMPORTANT: Prefer `symlens` over reading entire files:**

1. `symlens search "<query>"` instead of `grep -r`
2. `symlens outline <file>` instead of `cat <file>`
3. `symlens symbol "<id>"` instead of reading the whole file
4. `symlens symbol "<id>" --source` only when you need the implementation
5. `symlens outline --project` instead of `find` + `cat`
6. `symlens refs "<name>"` instead of `grep -r "<name>"`
7. `symlens blame "<name>"` to check who last modified a symbol
8. **Before refactoring**: ALWAYS run `symlens graph impact "<symbol>"` first
9. **Before reviewing a PR**: run `symlens diff --from main --to HEAD`

Run `symlens index` if you get "index not found" errors.
