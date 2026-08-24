import { describe, it, expect, afterEach } from 'vitest'
import { defaultFundingOptions, fundingExecuteMsg } from './funding.js'
import { classifyCw20FundingKind, fundingEnvFromVite } from './fundingKind.js'

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

  it('uses 10× LocalTerra genesis defaults (GitLab #372)', () => {
    for (const key of [
      'SWARM_ULUNA_TOPUP',
      'SWARM_UUSD_TOPUP',
      'SWARM_CW20_MINT_TOPUP',
      'SWARM_MIN_CW20_BALANCE',
    ]) {
      saved[key] = process.env[key]
      delete process.env[key]
    }
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

describe('classifyCw20FundingKind (GitLab #620)', () => {
  const wrap = 'terra1wrap'
  const tax = 'terra1tax'
  const gem = 'terra1gem'
  const env = fundingEnvFromVite({
    VITE_LUNC_C_TOKEN_ADDRESS: wrap,
    VITE_TOKEN_COMMUNITY_TAX_ADDRESS: tax,
  })

  it('skips wrap CW20s (no Mint)', () => {
    expect(classifyCw20FundingKind(wrap, env)).toBe('skip')
  })

  it('Transfers the pinned QA tax token (never Mint)', () => {
    expect(classifyCw20FundingKind(tax, env)).toBe('transfer')
  })

  it('Transfers when GetLauncherOrigin.launcher is set', () => {
    expect(classifyCw20FundingKind(gem, { wrapAddresses: [] }, 'terra1launcher')).toBe(
      'transfer'
    )
  })

  it('Mints gems and leaves TCL8Y to the caller Mint path', () => {
    expect(classifyCw20FundingKind(gem, env)).toBe('mint')
  })

  it('does not treat an empty origin as tax', () => {
    expect(classifyCw20FundingKind(gem, env, null)).toBe('mint')
    expect(classifyCw20FundingKind(gem, env, '')).toBe('mint')
  })
})

describe('fundingExecuteMsg (GitLab #624 leftover #5)', () => {
  it('Transfers the tax token and never Mints it', () => {
    expect(fundingExecuteMsg('transfer', 'terra1bot', '1')).toEqual({
      transfer: { recipient: 'terra1bot', amount: '1' },
    })
    expect(JSON.stringify(fundingExecuteMsg('transfer', 'terra1bot', '1'))).not.toMatch(
      /"mint"/
    )
  })

  it('Mints gems and skips wraps', () => {
    expect(fundingExecuteMsg('mint', 'terra1bot', '1')).toEqual({
      mint: { recipient: 'terra1bot', amount: '1' },
    })
    expect(fundingExecuteMsg('skip', 'terra1bot', '1')).toBeNull()
  })
})
