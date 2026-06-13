import { describe, it, expect, afterEach } from 'vitest'
import { defaultFundingOptions } from './funding.js'

describe('defaultFundingOptions', () => {
  const saved: Record<string, string | undefined> = {}

  afterEach(() => {
    for (const key of [
      'SWARM_ULUNA_TOPUP',
      'SWARM_UUSD_TOPUP',
      'SWARM_CW20_MINT_TOPUP',
      'SWARM_MIN_CW20_BALANCE',
    ]) {
      if (saved[key] === undefined) delete process.env[key]
      else process.env[key] = saved[key]
    }
  })

  it('uses 10× LocalTerra defaults when env unset (GitLab #372)', () => {
    for (const key of [
      'SWARM_ULUNA_TOPUP',
      'SWARM_UUSD_TOPUP',
      'SWARM_CW20_MINT_TOPUP',
      'SWARM_MIN_CW20_BALANCE',
    ]) {
      saved[key] = process.env[key]
      delete process.env[key]
    }
    const o = defaultFundingOptions()
    expect(o.ulunaTopup).toBe('20000000000000')
    expect(o.uusdTopup).toBe('10000000000000')
    expect(o.cw20MintTopup).toBe('100000000000000000')
    expect(o.minCw20Balance).toBe('10000000000000')
  })
})
