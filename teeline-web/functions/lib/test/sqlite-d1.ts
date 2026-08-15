// Shared test helper: a minimal D1-compatible shim over real SQLite
// (better-sqlite3). D1 *is* SQLite, so the SQL in db.ts / the handlers runs
// unchanged; numbered ?NNN params are normalized to anonymous ? (identical
// bind order — db.ts never reuses a numbered param).
//
// Why not miniflare's standalone D1 emulation: the miniflare library
// build shipped with wrangler 4.119 fails to start D1 wrapped bindings
// (cloudflare-internal:d1-api; workers-sdk#4077 / #10114). Note `wrangler
// pages dev`'s local D1 *does* work — this only affects the library path.
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import Database from 'better-sqlite3'

// Concatenate every migration in order so tests always mirror the production
// schema (a future 0002_*.sql is picked up automatically).
const migrationsDir = join(import.meta.dirname, '../../../migrations')
export const SCHEMA = readdirSync(migrationsDir)
  .filter((f) => f.endsWith('.sql'))
  .sort()
  .map((f) => readFileSync(join(migrationsDir, f), 'utf8'))
  .join('\n')

export type D1Like = {
  exec(sql: string): void
  prepare(sql: string): {
    bind(...params: unknown[]): {
      first<T>(col?: string): T | null
      all<T>(): { results: T[] }
      run(): { meta: { changes: number; last_row_id: number } }
    }
  }
  batch(stmts: { _isSelect?: boolean; all?<T>(): { results: T[] }; run?(): { meta: { changes: number } } }[]): unknown[]
}

export function makeD1(db: Database.Database): D1Like {
  return {
    exec(sql) {
      db.exec(sql)
    },
    prepare(sql) {
      // Fail loudly if a future query reuses a numbered param (?1 twice) or
      // embeds ?NNN in a string literal — the normalization would silently
      // change bind semantics and mask real bugs.
      const numbered = sql.match(/\?(\d+)/g)
      if (numbered && new Set(numbered).size !== numbered.length) {
        throw new Error(`sqlite-d1 shim: reused numbered param in SQL: ${sql}`)
      }
      const stmt = db.prepare(sql.replace(/\?(\d+)/g, '?'))
      // Real D1 batch executes each statement and returns per-statement
      // results; better-sqlite3 needs .all() for SELECTs and .run() for
      // writes, so the bound object carries the hint for batch().
      const isSelect = /^\s*SELECT/i.test(sql)
      return {
        bind(...params) {
          return {
            _isSelect: isSelect,
            first<T>(col?: string): T | null {
              const row = stmt.get(...params) as Record<string, unknown> | undefined
              if (row === undefined) return null
              if (col !== undefined) return (row[col] as T | undefined) ?? null
              return row as T
            },
            all<T>(): { results: T[] } {
              return { results: stmt.all(...params) as T[] }
            },
            run() {
              const info = stmt.run(...params)
              return { meta: { changes: info.changes, last_row_id: info.lastInsertRowid as number } }
            },
          }
        },
      }
    },
    batch(stmts) {
      return db.transaction(() => stmts.map((s) => (s._isSelect ? s.all() : s.run())))()
    },
  }
}
