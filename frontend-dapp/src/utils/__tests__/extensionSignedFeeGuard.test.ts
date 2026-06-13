import { describe, expect, it, vi } from 'vitest'
import {
  EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE,
  extensionSignedFeeUndershootMessage,
  gasFromAminoFee,
  isLocalTerraChainId,
  ulunaFromAminoFee,
} from '../extensionSignedFeeGuard'

describe('extensionSignedFeeGuard (GitLab #127)', () => {
  it('detects LocalTerra chain ids case-insensitively', () => {
    expect(isLocalTerraChainId('localterra')).toBe(true)
    expect(isLocalTerraChainId('LocalTerra')).toBe(true)
    expect(isLocalTerraChainId('columbus-5')).toBe(false)
  })

  it('parses uluna from amino fee', () => {
    expect(ulunaFromAminoFee({ amount: [{ denom: 'uluna', amount: '5665000' }] })).toBe(5665000n)
  })

  it('returns retail copy for Station-style undershoot (3000 vs 5665000)', () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const msg = extensionSignedFeeUndershootMessage(
      { fee: { amount: [{ denom: 'uluna', amount: '3000' }] } },
      { amount: [{ denom: 'uluna', amount: '5665000' }] },
      'localterra'
    )
    expect(msg).toBe(EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE)
    expect(msg).not.toMatch(/GitLab #127/)
    expect(msg).not.toMatch(/uluna/)
    expect(warnSpy.mock.calls[0]?.[1]).toMatch(/5665000/)
    warnSpy.mockRestore()
  })

  it('parses gas from amino fee', () => {
    expect(gasFromAminoFee({ gas: '840000' })).toBe(840000n)
  })

  it('allows signed fee and gas at or above 95% of expected', () => {
    expect(
      extensionSignedFeeUndershootMessage(
        { fee: { amount: [{ denom: 'uluna', amount: '5665000' }], gas: '200000' } },
        { amount: [{ denom: 'uluna', amount: '5665000' }], gas: '200000' },
        'localterra'
      )
    ).toBeNull()
  })

  it('returns retail copy for partial fee rewrite (~23 LUNC vs ~36 LUNC, GitLab #134)', () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const msg = extensionSignedFeeUndershootMessage(
      { fee: { amount: [{ denom: 'uluna', amount: '23000000' }], gas: '600000' } },
      { amount: [{ denom: 'uluna', amount: '36000000' }], gas: '840000' },
      'localterra'
    )
    expect(msg).toBe(EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE)
    expect(warnSpy.mock.calls[0]?.[1]).toMatch(/840000/)
    warnSpy.mockRestore()
  })

  it('returns retail copy when signed fee is missing on LocalTerra', () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const msg = extensionSignedFeeUndershootMessage(
      {},
      { amount: [{ denom: 'uluna', amount: '5665000' }] },
      'localterra'
    )
    expect(msg).toBe(EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE)
    expect(warnSpy.mock.calls[0]?.[1]).toMatch(/returned ~0 uluna/)
    warnSpy.mockRestore()
  })

  it('returns retail copy for zero uluna signed fee on LocalTerra', () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const msg = extensionSignedFeeUndershootMessage(
      { fee: { amount: [{ denom: 'uluna', amount: '0' }] } },
      { amount: [{ denom: 'uluna', amount: '5665000' }] },
      'localterra'
    )
    expect(msg).toBe(EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE)
    warnSpy.mockRestore()
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
