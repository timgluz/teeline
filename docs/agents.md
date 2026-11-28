# Agents

The project defines agent instructions in `.agents/agents/` and shared skills in
`.agents/skills/`. Use the narrowest agent that matches the job.

After opening a pull request, ask `open-code-review` (`ocr`) for a review.

## Model policy

- **Explore** with Gemini (`google/gemini-4-hydrogen`).
- **Author plans** and **review builds** with Opus (`anthropic/claude-opus-5-5`).
- **Review plans** with GitHub Copilot on the plan PR.
- **Implement** with DeepSeek (`deepseek/deepseek-v4-pro`).

## Blog posts

| Agent | Model | Role |
| --- | --- | --- |
| `blog-brainstormer` | Gemini | Generate 5–7 post angles with outlines, readers, evidence, and risks |
| `blog-writer` | Opus | Draft or rewrite a post, verify claims, validate the build |
| `blog-editor` | Opus | Independent editorial review; fact-checks and checks voice; no edits |
| `readability-checker` | inherit | Quantitative readability metrics only |

Flow: `blog-brainstormer` → pick an angle → `blog-writer` → `blog-editor`. The prose rules
are in [`docs/writing-style.md`](writing-style.md).

## Explainers

| Agent | Model | Role |
| --- | --- | --- |
| `explainer-brainstormer` | Gemini | 2–4 concepts: learning objective, aha, storyboard, fixture, interaction, risks |
| `explainer-planner` | Opus | OpenSpec change (`add-<id>-explainer`) + the explainer checklist |
| `explainer-plan-reviewer` | inherit | Thin: validates, opens or surfaces the plan PR and Copilot's review |
| `explainer-implementor` | DeepSeek | `openspec-apply-change` plus the built explainer |
| `explainer-build-reviewer` | Opus | Delivery review: tests, a11y, reduced motion, responsive, registration, honest stats |

Flow: `explainer-brainstormer` → `explainer-planner` (OpenSpec propose) → plan PR + Copilot
review → approve → `explainer-implementor` → `explainer-build-reviewer`.

The shared checklist is
[`.agents/skills/explainer-workflow/SKILL.md`](../.agents/skills/explainer-workflow/SKILL.md).

## OpenSpec and benchmarks

The OpenSpec workflow (`openspec-propose`, `openspec-apply-change`, `openspec-archive-change`)
and `benchmark-solver` are skills under `.agents/skills/`.
