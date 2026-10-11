// The problem minimap: where an instance's points come from, and how they are drawn.
//
// Shared because two scripts need different halves of it — `gen-problem-minimaps.mjs` renders the
// image, `gen-problem-pages.mjs` only needs to know whether the layout was derived so the page can
// say so. Rendering in the page generator was what made a regeneration a 223k-line diff; the two
// jobs are now separate.

export const VIEW_W = 340
export const VIEW_H = 200
const PAD = 20

export const DERIVED_NOTE =
  'Layout derived from the distance matrix — this instance publishes no coordinates.'

// Above this many cities the dots cost megabytes without showing more: the viewBox is 340x200, so
// past a few thousand points they are already sharing pixels. `pla85900` was 5.8 MB of `<circle>`
// for a picture whose structure a grid carries in a fraction of that.
const DENSITY_THRESHOLD = 3000
const DENSITY_CELL = 2

export function parseHeader(txt) {
  const fields = {}
  const re = /^([A-Z_ ]+?)\s*:\s*(.+)$/gm
  let m
  while ((m = re.exec(txt)) !== null) fields[m[1].trim()] = m[2].trim()
  return fields
}

// Read a section body as whitespace-split rows. The section ends at the next keyword line or EOF,
// not at a blank line: `swiss42` pads its headers with trailing spaces, and a coordinate-less
// instance's `DISPLAY_DATA_SECTION` is not always followed by `EOF`.
export function readSection(txt, name) {
  const lines = txt.split(/\r?\n/)
  const header = new RegExp(`^${name}\\s*$`, 'i')
  const start = lines.findIndex((l) => header.test(l.trim()))
  if (start < 0) return null

  const rows = []
  for (let i = start + 1; i < lines.length; i++) {
    const line = lines[i].trim()
    if (!line) continue
    if (/^EOF$/i.test(line) || /^[A-Z_]+\s*(:|$)/.test(line)) break
    rows.push(line.split(/\s+/))
  }
  return rows.length > 0 ? rows : null
}

// Points as `index x y` rows. NODE_COORD_SECTION holds the real coordinates; DISPLAY_DATA_SECTION
// holds TSPLIB's own 2-D projection for instances whose distances come from a matrix, which is why
// the five EXPLICIT instances that ship one can be drawn without deriving anything.
export function parsePoints(txt) {
  for (const section of ['NODE_COORD_SECTION', 'DISPLAY_DATA_SECTION']) {
    const rows = readSection(txt, section)
    if (!rows) continue
    const pts = []
    for (const parts of rows) {
      if (parts.length < 3) continue
      const x = parseFloat(parts[1])
      const y = parseFloat(parts[2])
      if (!isNaN(x) && !isNaN(y)) pts.push({ x, y })
    }
    if (pts.length > 0) return pts
  }
  return null
}

// The entries an EDGE_WEIGHT_FORMAT writes, in file order — TSPLIB's nine layouts, matching the
// Rust reader in src/tsp/tsplib.rs. Each picks a triangle, includes the diagonal or not, and walks
// it row- or column-wise.
const LAYOUTS = [
  ['FULL_MATRIX', (n) => rows(n, (i) => range(n))],
  ['UPPER_ROW', (n) => rows(n, (i) => range(n, i + 1))],
  ['LOWER_ROW', (n) => rows(n, (i) => range(i))],
  ['UPPER_DIAG_ROW', (n) => rows(n, (i) => range(n, i))],
  ['LOWER_DIAG_ROW', (n) => rows(n, (i) => range(i + 1))],
  ['UPPER_COL', (n) => columns(n, (j) => range(j))],
  ['LOWER_COL', (n) => columns(n, (j) => range(n, j + 1))],
  ['UPPER_DIAG_COL', (n) => columns(n, (j) => range(j + 1))],
  ['LOWER_DIAG_COL', (n) => columns(n, (j) => range(n, j))],
]

const range = (end, start = 0) => Array.from({ length: end - start }, (_, k) => start + k)
const rows = (n, inner) => Array.from({ length: n }, (_, i) => inner(i).map((j) => [i, j])).flat()
const columns = (n, inner) => Array.from({ length: n }, (_, j) => inner(j).map((i) => [i, j])).flat()

function explicitEntries(format, n) {
  const layout = LAYOUTS.find(([name]) => name === format)
  return layout ? layout[1](n) : null
}

// TSPLIB's matrix instances define the distances and nothing else, so their picture has to be
// derived. Returns null when the section is absent, the format is unknown, or the token count does
// not match the format's shape — a matrix of the wrong length would silently lay out the wrong
// instance.
export function parseDistanceMatrix(txt, dim, format) {
  const rows = readSection(txt, 'EDGE_WEIGHT_SECTION')
  if (!rows) return null
  const values = rows.flat().map(Number)
  if (values.some((v) => !Number.isFinite(v))) return null

  const entries = explicitEntries(format, dim)
  if (!entries || values.length !== entries.length) return null

  const D = Array.from({ length: dim }, () => new Float64Array(dim))
  entries.forEach(([i, j], k) => {
    D[i][j] = values[k]
    D[j][i] = values[k]
  })
  return D
}

// Classical MDS: double-centre the squared distances and keep the top two eigenvectors, which are
// the two dimensions that best preserve the distances in the plane. This is the same operation
// TSPLIB performed for the instances it published a DISPLAY_DATA_SECTION for — scored against
// those five it reproduces them at r = 0.95–1.000.
//
// Power iteration from a fixed starting vector, so the layout is a pure function of the matrix and
// the image stays byte-reproducible (which is what lets the publisher skip unchanged objects).
export function mdsLayout(D, dim) {
  const rowMean = new Float64Array(dim)
  const colMean = new Float64Array(dim)
  let grand = 0
  for (let i = 0; i < dim; i++) {
    for (let j = 0; j < dim; j++) {
      const sq = D[i][j] * D[i][j]
      rowMean[i] += sq
      colMean[j] += sq
      grand += sq
    }
  }
  for (let i = 0; i < dim; i++) rowMean[i] /= dim
  for (let j = 0; j < dim; j++) colMean[j] /= dim
  grand /= dim * dim

  const centred = (i, j) => -0.5 * (D[i][j] * D[i][j] - rowMean[i] - colMean[j] + grand)

  // Power iteration converges on the largest-magnitude eigenvalue and a double-centred matrix is
  // indefinite, so the spectrum is shifted by its largest |diagonal| to make the leading one
  // dominant. Shifting by a multiple of the identity leaves the eigenvectors alone.
  let shift = 0
  for (let i = 0; i < dim; i++) shift = Math.max(shift, Math.abs(centred(i, i)))
  const work = Array.from({ length: dim }, (_, i) => {
    const row = new Float64Array(dim)
    for (let j = 0; j < dim; j++) row[j] = centred(i, j) + (i === j ? shift : 0)
    return row
  })

  const multiply = (M, v) => {
    const out = new Float64Array(dim)
    for (let i = 0; i < dim; i++) {
      let sum = 0
      const row = M[i]
      for (let j = 0; j < dim; j++) sum += row[j] * v[j]
      out[i] = sum
    }
    return out
  }
  const norm = (v) => {
    let sum = 0
    for (let i = 0; i < v.length; i++) sum += v[i] * v[i]
    return Math.sqrt(sum)
  }

  const axes = []
  for (let axis = 0; axis < 2; axis++) {
    let v = new Float64Array(dim)
    for (let i = 0; i < dim; i++) v[i] = Math.sin(i * 1.7 + axis * 3.1) + Math.cos(i * 0.37)
    let len = norm(v) || 1
    for (let i = 0; i < dim; i++) v[i] /= len

    for (let iter = 0; iter < 400; iter++) {
      const next = multiply(work, v)
      len = norm(next) || 1
      for (let i = 0; i < dim; i++) next[i] /= len
      v = next
    }

    const wv = multiply(work, v)
    let rayleigh = 0
    for (let i = 0; i < dim; i++) rayleigh += v[i] * wv[i]
    const eigenvalue = rayleigh - shift
    axes.push({ v, eigenvalue })

    // Deflate the shifted matrix by the eigenpair just found: work = (B + shift·I) - λvvᵀ, so the
    // shift needs no correction here.
    for (let i = 0; i < dim; i++) {
      const row = work[i]
      const vi = v[i]
      if (vi === 0) continue
      for (let j = 0; j < dim; j++) row[j] -= eigenvalue * vi * v[j]
    }
  }

  return Array.from({ length: dim }, (_, i) => ({
    x: axes[0].v[i] * Math.sqrt(Math.max(axes[0].eigenvalue, 0)),
    y: axes[1].v[i] * Math.sqrt(Math.max(axes[1].eigenvalue, 0)),
  }))
}

/**
 * Where this instance's picture comes from, without computing it.
 *
 * `gen-problem-pages.mjs` needs this and nothing else: laying out `si1032` by MDS just to learn
 * that its map is derived would put a second of work in the page generator for a boolean.
 */
export function minimapSource(txt) {
  for (const section of ['NODE_COORD_SECTION', 'DISPLAY_DATA_SECTION']) {
    const rows = readSection(txt, section)
    if (!rows) continue
    if (rows.some((parts) => parts.length >= 3 && !isNaN(parseFloat(parts[1])))) {
      return section === 'NODE_COORD_SECTION' ? 'coords' : 'display'
    }
  }
  return 'matrix'
}

/** Points for the minimap, and whether they had to be derived rather than read off the file. */
export function minimapPoints(txt, dim, format) {
  const published = parsePoints(txt)
  if (published) return { points: published, derived: false }

  const D = parseDistanceMatrix(txt, dim, format)
  if (!D) return { points: null, derived: false }
  return { points: mdsLayout(D, dim), derived: true }
}

// One <rect> per occupied cell. Opacity grows with the count on a log scale, so the few cells
// holding most of the points do not saturate the picture and hide the sparse edges.
function densityRects(projected) {
  const cols = Math.ceil(VIEW_W / DENSITY_CELL)
  const rows = Math.ceil(VIEW_H / DENSITY_CELL)
  const counts = new Map()

  for (const p of projected) {
    const col = Math.min(cols - 1, Math.max(0, Math.floor(p.x / DENSITY_CELL)))
    const row = Math.min(rows - 1, Math.max(0, Math.floor(p.y / DENSITY_CELL)))
    const key = row * cols + col
    counts.set(key, (counts.get(key) || 0) + 1)
  }

  let max = 0
  for (const count of counts.values()) max = Math.max(max, count)

  const rects = []
  for (const [key, count] of counts) {
    const col = key % cols
    const row = (key - col) / cols
    const opacity = (0.45 + 0.55 * (Math.log1p(count) / Math.log1p(max))).toFixed(2)
    rects.push(
      `<rect x="${(col * DENSITY_CELL).toFixed(1)}" y="${(row * DENSITY_CELL).toFixed(1)}" width="${DENSITY_CELL}" height="${DENSITY_CELL}" fill="#0d9488" opacity="${opacity}"/>`
    )
  }
  return rects.join('\n')
}

/**
 * A standalone SVG document.
 *
 * Self-contained because it is fetched through `<img>`, which gives it no page context: the light
 * card and the point colour have to be in the file, and so does the label, since an isolated SVG
 * does not inherit the page's `alt`.
 */
export function renderMinimapSvg(coords, derived = false) {
  const xs = coords.map((p) => p.x)
  const ys = coords.map((p) => p.y)
  const minX = Math.min(...xs),
    maxX = Math.max(...xs)
  const minY = Math.min(...ys),
    maxY = Math.max(...ys)
  const w = maxX - minX || 1
  const h = maxY - minY || 1

  const scale = Math.min((VIEW_W - PAD * 2) / w, (VIEW_H - PAD * 2) / h)
  const offX = (VIEW_W - w * scale) / 2
  const offY = (VIEW_H - h * scale) / 2

  const projected = coords.map((p) => ({
    x: (p.x - minX) * scale + offX,
    y: (p.y - minY) * scale + offY,
  }))

  // For large datasets, use smaller dots
  const r = coords.length > 2000 ? 0.8 : coords.length > 500 ? 1.2 : 2

  const marks =
    projected.length > DENSITY_THRESHOLD
      ? densityRects(projected)
      : projected
          .map(
            (p) =>
              `<circle cx="${p.x.toFixed(1)}" cy="${p.y.toFixed(1)}" r="${r}" fill="#0d9488" opacity="0.7"/>`
          )
          .join('\n')

  const label = derived
    ? 'City layout derived from the distance matrix; this instance publishes no coordinates'
    : 'City map for this problem'

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${VIEW_W} ${VIEW_H}" width="${VIEW_W}" height="${VIEW_H}" style="border-radius:8px;border:1px solid #d0d7de;background:#f6f8fa" role="img" aria-label="${label}">
${marks}
</svg>
`
}
