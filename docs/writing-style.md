# Writing Style

Voice and anti-slop rules for the blog and long-form docs. The posts in
`teeline-web/src/content/blog/` are the reference.

## Voice

Write like a person who has done the work, not like a model imitating one. First person,
concrete, specific, and willing to say what failed. Prefer numbers, names, and short
sentences.

## Markers to avoid

- **Em-dashes.** Avoid them in prose. Use a comma, colon, parentheses, or a full stop. The
  only exceptions are a verbatim quote (never altered to fit style) and the bibliographic
  separator in a References list.
- **The "not X, but Y" tic** ("It isn't a bug. It's a feature.") and its variants.
- **Stock phrases**: "delve", "tapestry", "landscape", "realm", "navigate the complexities",
  "in today's fast-paced world", "it's worth noting", "let's dive in".
- **Mechanical tricolons** and lists of three adjectives for rhythm rather than meaning.
- **Empty intensifiers** ("very", "extremely", "incredibly") and hedges that add nothing.
- **A "Conclusion" section** that only restates the introduction.

If a sentence needs a dash to hold it together, it probably needs to be two sentences.

## Where this applies

- `teeline-web/src/content/blog/`.
- The descriptions and notes in `docs/algorithms/`.
- `docs/benchmarks.md` and long-form docs.

The blog agents (`blog-brainstormer`, `blog-writer`, `blog-editor`) and the
`readability-checker` enforce this. See [`docs/agents.md`](agents.md).
