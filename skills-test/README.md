# SymLens Skill Evaluation Suite

Evaluation test suite for the [SymLens](https://github.com/TtTRz/symlens) skill, targeting [TDesign Vue Next](https://github.com/tencent/tdesign-vue-next) as the benchmark codebase.

## How to Run

Use `skill-creator` with a subAgent to run the evals in `evals/evals.json`:

1. **Load skill-creator** — invoke `skill-creator` skill to get evaluation guidelines
2. **Spawn subAgent** — use a fresh subAgent per eval case, passing the `prompt` and `assertions`
3. **Baseline run** — run each `prompt` **without** the symlens skill, record the answer
4. **Skill run** — run the same `prompt` **with** the symlens skill loaded, record the answer
5. **Score** — check each answer against the `assertions` list; an assertion passes if the answer includes the stated fact
6. **Compare** — aggregate pass rates for baseline vs skill to measure improvement
