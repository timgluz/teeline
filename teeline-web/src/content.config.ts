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
    source: z.string(),
    sourceUrl: z.string(),
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
