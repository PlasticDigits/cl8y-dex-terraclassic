import { afterEach, describe, expect, it } from 'vitest'
import {
  DEFAULT_SELL_BPS,
  isTaxToken,
  normalizeTaxTokens,
  taxTokenFromEnv,
  taxWorkersEnabled,
} from './taxDetect.js'

describe('taxWorkersEnabled (GitLab #621)', () => {
  const saved = process.env.SWARM_TAX_WORKERS

  afterEach(() => {
    if (saved === undefined) delete process.env.SWARM_TAX_WORKERS
    else process.env.SWARM_TAX_WORKERS = saved
  })

  it('defaults on when unset', () => {
    expect(taxWorkersEnabled({})).toBe(true)
  })

  it('escape hatch SWARM_TAX_WORKERS=0', () => {
    expect(taxWorkersEnabled({ SWARM_TAX_WORKERS: '0' })).toBe(false)
    expect(taxWorkersEnabled({ SWARM_TAX_WORKERS: 'false' })).toBe(false)
  })

  it('treats 1 / true as on', () => {
    expect(taxWorkersEnabled({ SWARM_TAX_WORKERS: '1' })).toBe(true)
    expect(taxWorkersEnabled({ SWARM_TAX_WORKERS: 'true' })).toBe(true)
  })
})

describe('taxTokenFromEnv', () => {
  it('reads VITE_TOKEN_COMMUNITY_TAX_ADDRESS', () => {
    expect(
      taxTokenFromEnv({ VITE_TOKEN_COMMUNITY_TAX_ADDRESS: 'terra1taxqa' })
    ).toBe('terra1taxqa')
  })

  it('ignores empty / non-terra', () => {
    expect(taxTokenFromEnv({})).toBeUndefined()
    expect(taxTokenFromEnv({ VITE_TOKEN_COMMUNITY_TAX_ADDRESS: 'uluna' })).toBeUndefined()
  })
})

describe('normalizeTaxTokens', () => {
  it('dedupes terra1 addresses', () => {
    const s = normalizeTaxTokens(['terra1aaa', ' terra1aaa ', null, 'uluna', 'terra1bbb'])
    expect([...s].sort()).toEqual(['terra1aaa', 'terra1bbb'])
  })

  it('isTaxToken matches the set', () => {
    const s = normalizeTaxTokens(['terra1tax'])
    expect(isTaxToken('terra1tax', s)).toBe(true)
    expect(isTaxToken('terra1gem', s)).toBe(false)
    expect(DEFAULT_SELL_BPS).toBe(500)
  })
})
