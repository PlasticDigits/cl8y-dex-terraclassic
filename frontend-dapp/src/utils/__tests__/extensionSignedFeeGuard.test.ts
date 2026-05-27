import { describe, expect, it } from 'vitest'
import {
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

  it('flags Station-style partial fee rewrite (~23 LUNC vs ~36 LUNC, GitLab #134)', () => {
    const msg = extensionSignedFeeUndershootMessage(
      { fee: { amount: [{ denom: 'uluna', amount: '23000000' }], gas: '600000' } },
      { amount: [{ denom: 'uluna', amount: '36000000' }], gas: '840000' },
      'localterra'
    )
    expect(msg).toMatch(/GitLab #127/)
    expect(msg).toMatch(/840000/)
    expect(msg).toMatch(/600000/)
  })

  it('flags missing signed fee on LocalTerra', () => {
    const msg = extensionSignedFeeUndershootMessage(
      {},
      { amount: [{ denom: 'uluna', amount: '5665000' }] },
      'localterra'
    )
    expect(msg).toMatch(/GitLab #127/)
    expect(msg).toMatch(/returned ~0 uluna/)
  })

  it('flags zero uluna signed fee on LocalTerra', () => {
    const msg = extensionSignedFeeUndershootMessage(
      { fee: { amount: [{ denom: 'uluna', amount: '0' }] } },
      { amount: [{ denom: 'uluna', amount: '5665000' }] },
      'localterra'
    )
    expect(msg).toMatch(/GitLab #127/)
    expect(msg).toMatch(/returned ~0 uluna/)
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
