import { describe, it, expect } from 'vitest'
import {
  loadProfiles,
  pickActionKind,
  resolveSwarmProfiles,
  TAX_LISTED_PROFILE_ID,
  taxListedProfile,
} from './profiles.js'

describe('profiles', () => {
  it('loadProfiles validates five profiles', () => {
    const p = loadProfiles()
    expect(p.profiles).toHaveLength(5)
  })

  it('pickActionKind picks first bucket for small roll', () => {
    const p = loadProfiles().profiles[0]!
    expect(pickActionKind(p, 0)).toBe('router_multihop')
  })

  it('resolveSwarmProfiles swaps wallet 4 for tax_listed when tax workers are on', () => {
    const on = resolveSwarmProfiles(true)
    expect(on).toHaveLength(5)
    expect(on[4]!.id).toBe(TAX_LISTED_PROFILE_ID)
    expect(on[0]!.id).toBe('router_bias')
    const off = resolveSwarmProfiles(false)
    expect(off[4]!.id).toBe('balanced')
    expect(off.some((p) => p.id === TAX_LISTED_PROFILE_ID)).toBe(false)
  })

  it('tax_listed weights sum to 1 and stay trader-like', () => {
    const w = taxListedProfile().weights
    const sum = Object.values(w).reduce((a, b) => a + b, 0)
    expect(Math.abs(sum - 1)).toBeLessThan(1e-9)
    expect(w.hybrid_swap).toBeGreaterThan(0)
    expect(w.router_multihop + w.pair_swap + w.limit_order).toBeGreaterThan(0.25)
  })
})
