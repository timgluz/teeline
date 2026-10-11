import { defineCollection, z } from 'astro:content'
import { glob } from 'astro/loaders'

const docs = defineCollection({
  loader: glob({ pattern: '*.md', base: '../docs/algorithms' }),
  schema: z.object({
    id: z.string(),
    name: z.string(),
    typeBadge: z.string(),
    description: z.string(),
    hasExplainer: z.boolean().default(false),
  }),
})

const problems = defineCollection({
  loader: glob({ pattern: '*.md', base: '../docs/problems' }),
  schema: z.object({
    id: z.string(),
    name: z.string(),
    description: z.string(),
    cities: z.number(),
    sizeGroup: z.enum(['small', 'medium', 'large', 'xl']),
    edgeWeightType: z.string(),
    optimalCost: z.number().nullable(),
    dataUrl: z.string(),
    // Published to R2 by scripts/publish-minimaps.sh, not committed: the maps totalled 19.6 MB of
    // inline SVG in this collection before they moved out. Optional so that this schema and the
    // regeneration of 111 pages that sets it are separable — the deletion of the old inline maps
    // is a large mechanical diff and does not belong in the same review as the code.
    minimapUrl: z.url().optional(),
    // True where the instance publishes no coordinates and the layout was derived from its
    // distance matrix — the page has to say so rather than present a projection as a map.
    minimapDerived: z.boolean().default(false),
    // The raster twin of minimapUrl, for og:image and for the Dataset JSON-LD: neither social
    // clients nor structured-data consumers accept SVG.
    ogImageUrl: z.url().optional(),
    source: z.string(),
    sourceUrl: z.string(),
    creator: z.string(),
    license: z.string(),
  }),
})

const blog = defineCollection({
  loader: glob({ pattern: '*.md', base: './src/content/blog' }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    pubDate: z.coerce.date(),
    updatedDate: z.coerce.date().optional(),
    tags: z.array(z.string()).default([]),
    draft: z.boolean().default(false),
  }),
})

export const collections = { docs, blog, problems }
