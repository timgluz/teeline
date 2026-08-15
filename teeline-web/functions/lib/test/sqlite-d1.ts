// Shared test helper: a minimal D1-compatible shim over real SQLite
// (better-sqlite3). D1 *is* SQLite, so the SQL in db.ts / the handlers runs
// unchanged; numbered ?NNN params are normalized to anonymous ? (identical
// bind order — db.ts never reuses a numbered param).
//
// Why not miniflare's standalone D1 emulation: the miniflare library
// build shipped with wrangler 4.119 fails to start D1 wrapped bindings
// (cloudflare-internal:d1-api; workers-sdk#4077 / #10114). Note `wrangler
// pages dev`'s local D1 *does* work — this only affects the library path.
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import Database from 'better-sqlite3'

export const SCHEMA = readFileSync(join(import.meta.dirname, '../../../migrations/0001_init.sql'), 'utf8')

export type D1Like = {
  exec(sql: string): void
  prepare(sql: string): {
    bind(...params: unknown[]): {
      first<T>(col?: string): T | null
      all<T>(): { results: T[] }
      run(): { meta: { changes: number } }
    }
  }
  batch(stmts: { run(): { meta: { changes: number } } }[]): { meta: { changes: number } }[]
}

export function makeD1(db: Database.Database): D1Like {
  return {
    exec(sql) {
      db.exec(sql)
    },
    prepare(sql) {
      const stmt = db.prepare(sql.replace(/\?(\d+)/g, '?'))
      return {
        bind(...params) {
          return {
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
              return { meta: { changes: info.changes } }
            },
          }
        },
      }
    },
    batch(stmts) {
      return db.transaction(() => stmts.map((s) => s.run()))()
    },
  }
}
