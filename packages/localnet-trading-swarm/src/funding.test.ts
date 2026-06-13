import { describe, it, expect, afterEach } from 'vitest'
import { defaultFundingOptions } from './funding.js'

describe('defaultFundingOptions', () => {
  const keys = [
    'SWARM_ULUNA_TOPUP',
    'SWARM_UUSD_TOPUP',
    'SWARM_CW20_MINT_TOPUP',
    'SWARM_MIN_CW20_BALANCE',
  ] as const

  afterEach(() => {
    for (const k of keys) delete process.env[k]
  })

  it('uses 10× LocalTerra genesis defaults (GitLab #372)', () => {
    const f = defaultFundingOptions()
    expect(f.ulunaTopup).toBe('20000000000000')
    expect(f.uusdTopup).toBe('10000000000000')
    expect(f.cw20MintTopup).toBe('100000000000000000')
    expect(f.minCw20Balance).toBe('10000000000000')
  })

  it('allows SWARM_* env overrides', () => {
    process.env.SWARM_ULUNA_TOPUP = '1'
    process.env.SWARM_UUSD_TOPUP = '2'
    process.env.SWARM_CW20_MINT_TOPUP = '3'
    process.env.SWARM_MIN_CW20_BALANCE = '4'
    const f = defaultFundingOptions()
    expect(f.ulunaTopup).toBe('1')
    expect(f.uusdTopup).toBe('2')
    expect(f.cw20MintTopup).toBe('3')
    expect(f.minCw20Balance).toBe('4')
  })
})
