export interface SolverMeta {
  id: string
  name: string
}

export interface SolverGroup {
  label: string
  ids: string[]
}

export declare const SOLVER_META: Record<string, SolverMeta>
export declare const SOLVER_GROUPS: SolverGroup[]
export declare const PAGED_SOLVERS: Set<string>

export declare function renderSidebarHtml(currentId: string | null): string
export declare function renderTopbarHtml(): string
