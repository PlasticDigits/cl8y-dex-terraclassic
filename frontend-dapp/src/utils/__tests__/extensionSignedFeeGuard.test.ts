import { describe, expect, it } from 'vitest'
import { extensionSignedFeeUndershootMessage, isLocalTerraChainId, ulunaFromAminoFee } from '../extensionSignedFeeGuard'

describe('extensionSignedFeeGuard (GitLab #127)', () => {
  it('detects LocalTerra chain ids case-insensitively', () => {
    expect(isLocalTerraChainId('localterra')).toBe(true)
    expect(isLocalTerraChainId('LocalTerra')).toBe(true)
    expect(isLocalTerraChainId('columbus-5')).toBe(false)
  })

  it('parses uluna from amino fee', () => {
    expect(ulunaFromAminoFee({ amount: [{ denom: 'uluna', amount: '5665000' }] })).toBe(5665000n)
  })

  it('flags Station-style undershoot (3000 vs 5665000)', () => {
    const msg = extensionSignedFeeUndershootMessage(
      { fee: { amount: [{ denom: 'uluna', amount: '3000' }] } },
      { amount: [{ denom: 'uluna', amount: '5665000' }] },
      'localterra'
    )
    expect(msg).toMatch(/GitLab #127/)
    expect(msg).toMatch(/5665000/)
    expect(msg).toMatch(/3000/)
  })

  it('allows signed fee within half of expected', () => {
    expect(
      extensionSignedFeeUndershootMessage(
        { fee: { amount: [{ denom: 'uluna', amount: '5665000' }] } },
        { amount: [{ denom: 'uluna', amount: '5665000' }] },
        'localterra'
      )
    ).toBeNull()
  })

  it('skips validation on mainnet chain id', () => {
    expect(
      extensionSignedFeeUndershootMessage(
        { fee: { amount: [{ denom: 'uluna', amount: '3000' }] } },
        { amount: [{ denom: 'uluna', amount: '5665000' }] },
        'columbus-5'
      )
    ).toBeNull()
  })
})
