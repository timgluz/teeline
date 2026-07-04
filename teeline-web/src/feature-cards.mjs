// Plain JS (no TS syntax) — imported by vite.config.ts's transformIndexHtml plugin
// to server-render the homepage's "Features" card grid at build time.

const FEATURES = [
  {
    slug: 'webmcp',
    title: 'WebMCP',
    description: 'Solve TSP problems directly from an AI agent in the browser — no server, no API key, just a page an agent can call.',
    icon: '<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><circle cx="10" cy="5" r="2"/><circle cx="4.5" cy="15.5" r="2"/><circle cx="15.5" cy="15.5" r="2"/><path d="M8.6 6.8 L5.9 13.7"/><path d="M11.4 6.8 L14.1 13.7"/></svg>',
    links: [{ href: '/webmcp/', text: 'Learn more' }],
  },
  {
    slug: 'api',
    title: 'API',
    description: 'A REST API for solving TSP problems programmatically, with OpenAPI docs and rate limiting built in.',
    icon: '<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M7 5 L2.5 10 L7 15"/><path d="M13 5 L17.5 10 L13 15"/></svg>',
    links: [
      { href: 'https://api.tspsolver.com/docs', text: 'View API docs ↗', external: true },
      { href: 'https://accounts.tspsolver.com/sign-in', text: 'Get your API key ↗', external: true },
    ],
  },
  {
    slug: 'tools',
    title: 'Tools',
    description: 'A command-line binary for scripts and pipelines, plus a graphical desktop app built with Qt.',
    icon: '<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M12.3 3.8 a3 3 0 1 0 -3.9 3.9 L3.6 13.4 a1.6 1.6 0 0 0 2.3 2.3 L11.6 10 a3 3 0 0 0 3.9 -3.9 l-2 2 -1.6-1.6 z"/></svg>',
    links: [{ href: 'https://github.com/timgluz/teeline', text: 'View on GitHub ↗', external: true }],
  },
  {
    slug: 'wasm',
    title: 'WebAssembly',
    description: 'Every solver is compiled to WebAssembly and runs client-side at near-native speed — no install, nothing sent to a server.',
    icon: '<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"><polygon points="10,2 17,6 17,14 10,18 3,14 3,6"/></svg>',
    links: [{ href: 'https://github.com/timgluz/teeline#webassembly', text: 'View on GitHub ↗', external: true }],
  },
]

export function renderFeatureCardsHtml() {
  const cards = FEATURES.map(f => {
    const links = f.links.length
      ? `<ul class="card-links">${f.links
          .map(l => `<li><a href="${l.href}"${l.external ? ' target="_blank" rel="noopener noreferrer"' : ''}>${l.text}</a></li>`)
          .join('')}</ul>`
      : ''
    return `
      <article class="card card--${f.slug}">
        <div class="card-icon" aria-hidden="true">${f.icon}</div>
        <h3 class="card-title">${f.title}</h3>
        <p class="card-desc">${f.description}</p>
        ${links}
      </article>`
  }).join('')
  return `<div class="card-grid card-grid--2col">${cards}</div>`
}
