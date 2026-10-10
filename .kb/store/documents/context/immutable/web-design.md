---
id: 019fe738-4d62-73c3-9cdd-795b94276505
slug: context/immutable/web-design
title: "Web UI Design Guidelines — dark theme, no AI slop"
type: context
status: active
priority: high
in:
  context/extensible/tech: "a0"
  context/immutable/architecture: "a0"
---

## Web UI Design Guidelines (teeline-web)

Long-lived visual constraints for the teeline-web frontend. Applies to the landing page
(already redesigned, PR #442) and to all future page revamps (`/solve/`, `/problems/`,
`/algorithms/`, `/blog/`). If a redesign deviates from these, it needs a strong reason.

## 1. Core identity

The **TSP route/edge-line motif** IS the brand — a tour on a map, an edge being swapped,
a scatter plot of algorithms. Every page should lean on that visual instead of generic
decorative shapes. The product is a tool with a point of view, not a marketing shell.

## 2. Palette (exact values — do not drift)

| Token | Hex | Use |
|---|---|---|
| canvas | `#0A0A0B` | page background |
| surface | `#141416` | cards, panels |
| surface-2 | `#1C1C20` | hover, nested |
| border | `#232327` | 1px hairlines |
| border-hover | `#3A3A40` | hover border brighten |
| text | `#F5F5F6` | primary text |
| text-dim | `#8A8A92` | secondary text, labels |
| accent | `#6366F1` | indigo — reserved for active route edge / primary CTA |
| accent-dim | `rgba(99,102,241,0.12)` | route fills, glows |
| positive | `#22D3A5` | "improving" tour length, good gap |
| negative | `#F87171` | bad gap, errors |

These are defined once in `tailwind.config.mjs` as `--color-*` tokens and used via
Tailwind utility classes. **Never** invent new hex values for a page; reuse the tokens.

## 3. Typography

- UI/body: **Inter** (400/500/600). Base 15px, line-height 1.55.
- Display headings: Inter weight **600 max — never 700**. Letter-spacing `-0.02em` at large sizes.
- Monospace: **JetBrains Mono** for every number, algorithm token, file extension
  (`.tsp`, `.opt.tour`), metrics (tour length, gap, runtime), and code blocks.
- Scale: 13 / 15 / 18 / 24 / 32 / 48. Never beyond 56px.

## 4. Layout

- **Bento grid** for feature/content sections (one large tile + smaller supporting tiles),
  not equal-weight card grids.
- Section rhythm (landing page): hero → algorithms → benchmarks → access → footer.
- The solver wizard (Load → Configure → Results) lives on `/solve/` as the tool's own UI,
  never as the landing hero.
- Algorithm navigation: grouped by family (Exact / Constructive / Local search /
  Metaheuristic) with chips, never a flat 22-item list.
- No FAQ, testimonials, or "trusted by" logo strips — reads as template filler.

## 5. Anti-patterns — the "AI slop" tells (do not reintroduce)

- ❌ Purple→pink gradients, glassmorphism blur cards, aurora background blobs
- ❌ Drop shadows deeper than `0 1px 2px rgba(0,0,0,0.3)`
- ❌ Inline `style=` attributes — everything goes through classes (Tailwind tokens)
- ❌ Decorative motion: fade-in-up on scroll, stagger children, count-ups, parallax
- ❌ Equal-weight icon+title+sentence feature card grids
- ❌ A bare version string floating alone in the footer
- ❌ Wizard/stepper as the hero, product name buried below the fold
- ❌ Template-y copy that could describe any solver ("one of computing's oldest hard problems")

## 6. Motion — functional only

One animated thing per page, and only when it communicates state:
- The hero route solving itself on a loop (already implemented in `hero-canvas.ts`).
- Status dots pulsing (live indicator).
- Hover: border brightens `#232327` → `#3A3A40`, background lifts one surface step.
  No scale transforms, no glow blooms.

## 7. Implementation constraints

- The landing page uses `LandingLayout.astro` (no PicoCSS) so Tailwind utilities can't
  conflict with PicoCSS resets. Docs pages keep PicoCSS + light theme for now.
- The hero canvas is a **TS-only 2-opt replay** (no WASM on the landing page) — keeps load
  fast; the real WASM solver is on `/solve/`.
- Benchmark visuals must use real measured data from `docs/benchmarks.md`, never invented.
- Icons: a consistent monoline set (e.g. Lucide/Phosphor) at 18px, single accent color.
  No emoji icons, no stock illustrations, no isometric characters.

## Validation

When reviewing a new page or redesign, check it against sections 2–6. If it fails any
anti-pattern in section 5, it's a regression. Reference visual floor: tspvis.com
(canvas-first interactivity), Linear/Vercel (craft: near-black canvas, restrained accent,
mono numerics, no decorative chrome).
