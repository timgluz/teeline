# Writing Style

Voice and anti-slop rules for the blog and long-form docs, adapted from William Zinsser's
*On Writing Well*. The posts in `teeline-web/src/content/blog/` are the reference.

## Voice

Write like a person who has done the work, not like a model imitating one. First person,
concrete, specific, and willing to say what failed. Prefer numbers, names, and short
sentences.

## Simplicity: strip every sentence

Zinsser's first principle is to strip every sentence to its cleanest components. Remove the
clutter: the unnecessary words, the pompous phrases, the useless qualifiers. If a sentence
can lose a word without losing meaning, lose it.

- Cut "very", "really", "quite", "a bit", "sort of", "perhaps", "I think", "it seems".
- Prefer the short word: "use" not "utilize", "help" not "facilitate", "start" not
  "commence".
- Cut the padding phrases: "in order to" → "to", "due to the fact that" → "because", "at
  this point in time" → "now".
- One idea per paragraph; keep paragraphs short.

## Words and verbs

- Use active verbs: "the solver picks the nearest city", not "the nearest city is picked".
- Do not prop up a weak verb with an adverb: "ran quickly" → "sprinted".
- Be specific: "0.51 s on dsj1000", not "fast"; "8,098 against 7,542", not "close".
- Replace clichés and stock phrases with fresh, concrete images.

## The reader

- Decide who the reader is before you start. Here: Rust developers and TSP enthusiasts who
  are comfortable with code but are not necessarily researchers.
- Trust the reader. Do not over-explain the obvious or repeat the previous paragraph.
- A post should be as long as its idea, and no longer.

## The lead and the ending

- The lead is the most important sentence. Open with the concrete thing, not a definition.
- The ending should land, not summarize. Do not add a `## Conclusion` that restates the
  introduction; end on the last real point or a call to action.

## Rewriting

Zinsser: rewriting is the essence of writing. The first draft is material, not the post.

- Cut a meaningful fraction of the draft on the second pass.
- Read it aloud; if you stumble, the sentence is too long or the rhythm is wrong.
- Delete the throat-clearing first paragraph if the real lead is the second one.
- Verify every number, code snippet, and link against the repo.

## Markers to avoid

- **Em-dashes.** No em-dashes in prose. Use a comma, colon, parentheses, or a full stop. The
  only exceptions are a verbatim quote (never altered to fit style) and the bibliographic
  separator in a References list.
- **The "not X, but Y" tic** ("It isn't a bug. It's a feature.") and its variants.
- **Stock phrases**: "delve", "tapestry", "landscape", "realm", "navigate the complexities",
  "in today's fast-paced world", "it's worth noting", "let's dive in".
- **Mechanical tricolons** and lists of three adjectives for rhythm rather than meaning.
- **Empty intensifiers** ("very", "extremely", "incredibly") and hedges that add nothing.
- **A "Conclusion" section** that only restates the introduction.

If a sentence needs a dash to hold it together, it probably needs to be two sentences.

## Checklist

- Is the lead concrete, with no throat-clearing before it?
- Can any sentence lose a word without losing meaning?
- Is any adverb or adjective doing the verb's job?
- Is the sentence in the active voice?
- Is every number, snippet, and link verified?
- Does the ending land, or does it summarize?
- Read it aloud. Did you stumble?

## Where this applies

- `teeline-web/src/content/blog/`.
- The descriptions and notes in `docs/algorithms/`.
- `docs/benchmarks.md` and long-form docs.

The blog agents (`blog-brainstormer`, `blog-writer`, `blog-editor`) and the
`readability-checker` enforce this. See [`docs/agents.md`](agents.md).
