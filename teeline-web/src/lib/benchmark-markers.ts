// Which index rows carry the "has published results" marker.
//
// The manifest is the only thing that says published: the publisher retires an object by omitting
// it from the manifest while the shard keeps answering 200, and bench/matrix/tier-a.toml is the
// campaign's intent rather than a statement about what exists. So both index pages decide at
// runtime, from the manifest, and never at build time.

import type { BenchmarkIndex } from './benchmark'

export type MarkerKind = 'dataset' | 'solver'

/** The tooltip, and the accessible name, of every marker. */
export const MARKER_TITLE = 'It has benchmark results'

/**
 * The row attribute naming each subject, which is also the manifest field to match it against.
 *
 * The pages write these attributes by hand, so a rename here would silently stop the markers
 * appearing; the e2e asserts a marker on a known row rather than counting them for that reason.
 */
export function markerAttribute(kind: MarkerKind): string {
  return kind === 'dataset' ? 'data-dataset' : 'data-solver'
}

/**
 * The ids the manifest publishes, or null when the manifest could not be read.
 *
 * Null and an empty set are different facts: an unreadable manifest has to leave every marker
 * hidden, rather than let the page imply that nothing has been measured.
 */
export function publishedIds(
  index: BenchmarkIndex | null,
  kind: MarkerKind,
): Set<string> | null {
  if (index === null) return null
  return kind === 'dataset'
    ? new Set(index.problems.map((problem) => problem.id))
    : new Set(index.solvers.map((solver) => solver.id))
}

/**
 * The count to state above the tables, or null when it would say nothing.
 *
 * When every row has results — which is the case for the solvers today — the line is noise, and
 * when the manifest could not be read there is no count to state at all.
 */
export function markerCount(
  marked: number,
  total: number,
  kind: MarkerKind,
): string | null {
  if (total === 0 || marked >= total) return null
  const noun = kind === 'dataset' ? 'instance' : 'solver'
  const subject = total === 1 ? `${noun} has` : `${noun}s have`
  return `${marked} of ${total} ${subject} published benchmark results`
}
