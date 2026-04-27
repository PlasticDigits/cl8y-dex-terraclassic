import { describe, it, expect } from 'vitest'
import { loadProfiles, pickActionKind } from './profiles.js'

describe('profiles', () => {
  it('loadProfiles validates five profiles', () => {
    const p = loadProfiles()
    expect(p.profiles).toHaveLength(5)
  })

  it('pickActionKind picks first bucket for small roll', () => {
    const p = loadProfiles().profiles[0]!
    expect(pickActionKind(p, 0)).toBe('router_multihop')
  })
})
