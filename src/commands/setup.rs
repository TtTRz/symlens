use crate::cli::SetupArgs;
use std::fs;
use std::path::{Path, PathBuf};

/// All supported agent targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentTarget {
    ClaudeCode,
    OpenClaw,
    Cursor,
}

impl AgentTarget {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "claude" | "claudecode" | "claude-code" => Some(Self::ClaudeCode),
            "openclaw" | "claw" => Some(Self::OpenClaw),
            "cursor" => Some(Self::Cursor),
            _ => None,
        }
    }

    fn all() -> &'static [AgentTarget] {
        &[Self::ClaudeCode, Self::OpenClaw, Self::Cursor]
    }
}

pub fn run(args: SetupArgs, root_override: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;

    if args.list {
        println!("Supported agents:");
        println!();
        println!("  Agent        Project (default)                   Global (-g/--global)");
        println!(
            "  ───────────  ─────────────────────────────────   ─────────────────────────────────────────"
        );
        println!(
            "  claude-code  ./CLAUDE.md                         ~/.claude/skills/symlens/SKILL.md"
        );
        println!("  openclaw     ~/.openclaw/skills/symlens/         (same — always user-level)");
        println!("  cursor       ./.cursor/rules/symlens.mdc        ~/.cursor/rules/symlens.mdc");
        println!();
        println!("Usage:");
        println!("  symlens setup claude-code           # append to project CLAUDE.md");
        println!("  symlens setup claude-code --global   # install as skill (/symlens)");
        println!("  symlens setup --all --global         # all agents, user-level");
        return Ok(());
    }

    let targets = if args.all {
        AgentTarget::all().to_vec()
    } else if let Some(ref name) = args.agent {
        match AgentTarget::from_str(name) {
            Some(t) => vec![t],
            None => {
                anyhow::bail!(
                    "Unknown agent: '{}'. Use `symlens setup --list` to see supported agents.",
                    name
                );
            }
        }
    } else {
        anyhow::bail!("Specify an agent name or use --all. Run `symlens setup --list` for help.");
    };

    let global = args.global;
    let force = args.force;

    for target in &targets {
        if args.uninstall {
            match target {
                AgentTarget::ClaudeCode => uninstall_claude_code(&root, global)?,
                AgentTarget::OpenClaw => uninstall_openclaw()?,
                AgentTarget::Cursor => uninstall_cursor(&root, global)?,
            }
        } else {
            match target {
                AgentTarget::ClaudeCode => setup_claude_code(&root, global, force)?,
                AgentTarget::OpenClaw => setup_openclaw(force)?,
                AgentTarget::Cursor => setup_cursor(&root, global, force)?,
            }
        }
    }

    Ok(())
}

// ─── Claude Code ─────────────────────────────────────────────────────
//   project: ./CLAUDE.md
//   global:  ~/.claude/CLAUDE.md

fn setup_claude_code(root: &Path, global: bool, force: bool) -> anyhow::Result<()> {
    if global {
        // Global: write as a skill → ~/.claude/skills/symlens/SKILL.md
        let home = home_dir()?;
        let skill_dir = home.join(".claude").join("skills").join("symlens");
        fs::create_dir_all(&skill_dir)?;

        let target = skill_dir.join("SKILL.md");
        let content = claude_code_skill();
        write_file(&target, &content, force)?;
        println!("  ✓ Claude Code (global skill): wrote {}", target.display());
        println!("    Use /symlens in Claude Code to activate");
    } else {
        // Project: write/append CLAUDE.md
        let target = root.join("CLAUDE.md");

        if target.exists() && !force {
            let content = fs::read_to_string(&target)?;
            if content.contains("symlens") {
                println!(
                    "  ✓ Claude Code (project): already contains symlens instructions (use --force to overwrite)"
                );
                return Ok(());
            }
            // Append to existing CLAUDE.md
            let section = claude_code_section();
            let mut new_content = content;
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push('\n');
            new_content.push_str(&section);
            fs::write(&target, new_content)?;
            println!(
                "  ✓ Claude Code (project): appended symlens section to {}",
                target.display()
            );
            return Ok(());
        }

        let content = claude_code_full();
        write_file(&target, &content, force)?;
        println!("  ✓ Claude Code (project): wrote {}", target.display());
    }
    Ok(())
}

fn claude_code_section() -> String {
    r#"## Code Navigation (SymLens)

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
"#
    .to_string()
}

fn claude_code_full() -> String {
    format!("# Project\n\n{}\n", claude_code_section().trim())
}

fn claude_code_skill() -> String {
    r#"# SymLens — Token-Efficient Code Intelligence

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
symlens graph deps                         # Module dependency graph
symlens graph path <A> <B>                 # Call path between symbols
symlens lines <file> <start> <end>         # Source by line range
symlens blame "<name>"                     # Git blame for a symbol
symlens diff --from <ref> --to <ref>       # Changed symbols between refs
symlens watch                              # Auto-update index
symlens stats                              # Index statistics
```

## Critical Rules

1. **ALWAYS** run `symlens graph impact "<symbol>"` before refactoring
2. **ALWAYS** run `symlens diff --from main --to HEAD` before reviewing a PR
3. Run `symlens index` if you get "index not found" errors
4. Use `--source` flag only when you actually need the implementation, not just the signature

## Supported Languages

Rust, TypeScript, Python, Swift, Go — full support for symbols, calls, refs, and imports.
"#
    .to_string()
}

// ─── OpenClaw ────────────────────────────────────────────────────────
//   always user-level: ~/.openclaw/skills/symlens/SKILL.md

fn setup_openclaw(force: bool) -> anyhow::Result<()> {
    let home = home_dir()?;
    let skill_dir = home.join(".openclaw").join("skills").join("symlens");
    fs::create_dir_all(&skill_dir)?;

    let target = skill_dir.join("SKILL.md");
    let content = openclaw_skill();
    write_file(&target, &content, force)?;
    println!("  ✓ OpenClaw (global): wrote {}", target.display());
    Ok(())
}

fn openclaw_skill() -> String {
    r#"---
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
| `symlens graph deps` | Module dependency graph |
| `symlens graph path <A> <B>` | Call path between two symbols |
| `symlens lines <file> <start> <end>` | Get source by line range |
| `symlens blame "<name>"` | Git blame for a symbol |
| `symlens diff --from <ref> --to <ref>` | Changed symbols between commits |
| `symlens watch` | Auto-update index on file changes |
| `symlens stats` | Index statistics |

## Usage Guidelines

1. **Always prefer `symlens` over reading entire files** — it saves tokens.
2. Run `symlens index` first if the project hasn't been indexed.
3. Use `symlens search` instead of `grep -r` for symbol search.
4. Use `symlens outline --project` instead of `find` + `cat` for project overview.
5. **Before refactoring**: run `symlens graph impact "<symbol>"` to check blast radius.
6. **Before reviewing changes**: run `symlens diff --from main --to HEAD`.
7. Use `symlens symbol "<id>"` to get just the signature (~60 tokens) instead of the whole file (~4000 tokens).

## Supported Languages

Rust, TypeScript, Python, Swift, Go — full support for symbols, calls, refs, and imports.
"#
    .to_string()
}

// ─── Cursor ──────────────────────────────────────────────────────────
//   project: ./.cursor/rules/symlens.mdc
//   global:  ~/.cursor/rules/symlens.mdc

fn setup_cursor(root: &Path, global: bool, force: bool) -> anyhow::Result<()> {
    let rules_dir = if global {
        let home = home_dir()?;
        home.join(".cursor").join("rules")
    } else {
        root.join(".cursor").join("rules")
    };
    fs::create_dir_all(&rules_dir)?;

    let scope = if global { "global" } else { "project" };
    let target = rules_dir.join("symlens.mdc");
    let content = cursor_rule();
    write_file(&target, &content, force)?;
    println!("  ✓ Cursor ({scope}): wrote {}", target.display());
    Ok(())
}

fn cursor_rule() -> String {
    r#"---
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
| Impact analysis | `symlens graph impact "<name>"` |
| Dependency graph | `symlens graph deps` |
| Git blame | `symlens blame "<name>"` |
| Changed symbols | `symlens diff --from <ref> --to <ref>` |
"#
    .to_string()
}

// ─── Helpers ─────────────────────────────────────────────────────────

fn home_dir() -> anyhow::Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))
}

fn write_file(path: &Path, content: &str, force: bool) -> anyhow::Result<()> {
    if path.exists() && !force {
        anyhow::bail!(
            "File already exists: {} (use --force to overwrite)",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

// ─── Uninstall ──────────────────────────────────────────────────────

fn uninstall_claude_code(root: &Path, global: bool) -> anyhow::Result<()> {
    if global {
        let home = home_dir()?;
        let skill_dir = home.join(".claude").join("skills").join("symlens");
        if skill_dir.exists() {
            fs::remove_dir_all(&skill_dir)?;
            println!(
                "  ✓ Claude Code (global skill): removed {}",
                skill_dir.display()
            );
        } else {
            println!("  - Claude Code (global skill): not installed");
        }
    } else {
        let target = root.join("CLAUDE.md");
        if target.exists() {
            let content = fs::read_to_string(&target)?;
            if content.contains("## Code Navigation (SymLens)") {
                // Remove the symlens section
                let section_start = "## Code Navigation (SymLens)";
                if let Some(start_idx) = content.find(section_start) {
                    // Find the next ## heading or end of file
                    let after_section = &content[start_idx + section_start.len()..];
                    let end_offset = after_section
                        .find("\n## ")
                        .map(|i| start_idx + section_start.len() + i)
                        .unwrap_or(content.len());
                    let mut new_content = String::new();
                    new_content.push_str(content[..start_idx].trim_end());
                    let remainder = &content[end_offset..];
                    if !remainder.is_empty() {
                        new_content.push_str("\n\n");
                        new_content.push_str(remainder.trim_start());
                    }
                    new_content.push('\n');
                    // If only whitespace left, remove the file
                    if new_content.trim().is_empty() {
                        fs::remove_file(&target)?;
                        println!(
                            "  ✓ Claude Code (project): removed empty {}",
                            target.display()
                        );
                    } else {
                        fs::write(&target, new_content)?;
                        println!(
                            "  ✓ Claude Code (project): removed symlens section from {}",
                            target.display()
                        );
                    }
                }
            } else {
                println!("  - Claude Code (project): no symlens section found in CLAUDE.md");
            }
        } else {
            println!("  - Claude Code (project): CLAUDE.md not found");
        }
    }
    Ok(())
}

fn uninstall_openclaw() -> anyhow::Result<()> {
    let home = home_dir()?;
    let skill_dir = home.join(".openclaw").join("skills").join("symlens");
    if skill_dir.exists() {
        fs::remove_dir_all(&skill_dir)?;
        println!("  ✓ OpenClaw: removed {}", skill_dir.display());
    } else {
        println!("  - OpenClaw: not installed");
    }
    Ok(())
}

fn uninstall_cursor(root: &Path, global: bool) -> anyhow::Result<()> {
    let target = if global {
        let home = home_dir()?;
        home.join(".cursor").join("rules").join("symlens.mdc")
    } else {
        root.join(".cursor").join("rules").join("symlens.mdc")
    };
    let scope = if global { "global" } else { "project" };
    if target.exists() {
        fs::remove_file(&target)?;
        println!("  ✓ Cursor ({scope}): removed {}", target.display());
    } else {
        println!("  - Cursor ({scope}): not installed");
    }
    Ok(())
}
