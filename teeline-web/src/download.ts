import type { City } from 'teeline-wasm'
import type { RunRecord } from './results'

export function buildTourText(name: string, route: number[]): string {
  return `NAME: ${name}\nTOUR_SECTION\n${route.join('\n')}\n-1\nEOF`
}

export function buildCsvText(route: number[], cities: City[]): string {
  const byId = new Map(cities.map((c) => [c.id, c]))
  const rows = route.map((id) => {
    const c = byId.get(id)!
    return `${c.id},${c.x},${c.y}`
  })
  return ['id,x,y', ...rows].join('\n')
}

export function buildJsonText(
  name: string,
  record: RunRecord,
  cities: City[],
  timestamp: number,
): string {
  return JSON.stringify({
    name,
    solver: record.solver,
    options: {},
    cities,
    route: record.route,
    total: record.total,
    runtime: record.runtime,
    timestamp,
  })
}

export function serializeSvg(svgEl: SVGSVGElement): Blob {
  const svg = new XMLSerializer().serializeToString(svgEl)
  return new Blob([svg], { type: 'image/svg+xml' })
}

export function triggerDownload(content: string | Blob, filename: string, mime: string): void {
  const blob = typeof content === 'string' ? new Blob([content], { type: mime }) : content
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}
