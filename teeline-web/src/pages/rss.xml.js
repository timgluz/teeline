import rss from '@astrojs/rss'
import { getCollection } from 'astro:content'

export async function GET(context) {
  // Standard Astro draft handling: `astro dev` shows drafts (for preview),
  // production builds (`astro build`/preview) exclude them.
  const posts = await getCollection('blog', ({ data }) => !data.draft || !import.meta.env.PROD)
  return rss({
    title: 'Teeline Blog',
    description: 'Notes on TSP algorithms, teeline development, and the occasional deep dive.',
    site: context.site,
    items: posts
      .sort((a, b) => b.data.pubDate.valueOf() - a.data.pubDate.valueOf())
      .map((post) => ({
        title: post.data.title,
        description: post.data.description,
        pubDate: post.data.pubDate,
        link: `/blog/${post.id}/`,
      })),
  })
}
