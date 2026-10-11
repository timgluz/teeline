// Display name for a solver id.
//
// The shards carry ids (`2opt`, `bhk`, `shuffle`), and the names live in nav-data with the rest of
// the navigation. A solver with no doc page — `shuffle`, the random baseline — falls back to its
// id rather than disappearing from a table.

import { SOLVER_META } from '../nav-data'

export function solverLabel(id: string): string {
  return SOLVER_META[id]?.name ?? id
}

/**
 * The solver's own page, or null when it has none.
 *
 * `shuffle` publishes a shard but no doc page, so a leaderboard row for it must render as text
 * rather than as a link to a 404.
 */
export function solverHref(id: string): string | null {
  return SOLVER_META[id] ? `/algorithms/${id}/` : null
}
