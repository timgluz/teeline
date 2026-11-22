import { describe, expect, it } from 'vitest'

import {
  FRAME_DWELL_MS,
  HOLD_AT_OPTIMUM_MS,
  HOLD_AT_SEED_MS,
  frameDwell,
} from './hero-canvas'

describe('frameDwell', () => {
  it('settles longest on the converged tour', () => {
    expect(frameDwell(12, 12)).toBe(HOLD_AT_OPTIMUM_MS)
  })

  it('pauses on the seed before replaying', () => {
    expect(frameDwell(0, 12)).toBe(HOLD_AT_SEED_MS)
  })

  it('steps steadily between the endpoints', () => {
    expect(frameDwell(4, 12)).toBe(FRAME_DWELL_MS)
  })

  it('keeps the beats ordered slowest to fastest', () => {
    expect(HOLD_AT_OPTIMUM_MS).toBeGreaterThan(HOLD_AT_SEED_MS)
    expect(HOLD_AT_SEED_MS).toBeGreaterThan(FRAME_DWELL_MS)
  })
})
