import { describe, it, expect } from 'vitest'
import {
  computeNativeTransferTaxUluna,
  netUlunaAfterTransferTax,
  grossUlunaForTargetNet,
  formatBurnTaxPercentLabel,
  type NativeTransferTaxParams,
} from '@/utils/nativeTransferTax'

describe('nativeTransferTax (Classic burn tax / #512)', () => {
  const params015: NativeTransferTaxParams = { rate: '0.015', capUluna: 1_000_000_000_000_000n }
  const params005: NativeTransferTaxParams = { rate: '0.0005', capUluna: 1_000_000_000_000_000n }

  it('matches columbus-5 InstantWithdraw tax: 9800 × 1.5% = 147 (#512 evidence)', () => {
    const withdraw = 9_800_000_000n
    expect(computeNativeTransferTaxUluna(withdraw, params015)).toBe(147_000_000n)
    expect(netUlunaAfterTransferTax(withdraw, params015)).toBe(9_653_000_000n)
  })

  it('uses multiply formula (not Terraswap inverse)', () => {
    const gross = 10_000_000n
    // floor(10_000_000 * 0.0005) = 5_000
    expect(computeNativeTransferTaxUluna(gross, params005)).toBe(5_000n)
    expect(netUlunaAfterTransferTax(gross, params005)).toBe(9_995_000n)
  })

  it('returns gross when tax rate is zero', () => {
    const gross = 1_000_000n
    expect(netUlunaAfterTransferTax(gross, { rate: '0', capUluna: 0n })).toBe(gross)
  })

  it('respects tax cap', () => {
    const params: NativeTransferTaxParams = { rate: '0.015', capUluna: 100n }
    expect(computeNativeTransferTaxUluna(1_000_000n, params)).toBe(100n)
  })

  it('grossUlunaForTargetNet finds smallest gross with net ≥ target', () => {
    const withdraw = 9_800_000_000n
    const net = netUlunaAfterTransferTax(withdraw, params015)
    const gross = grossUlunaForTargetNet(net, params015)
    expect(netUlunaAfterTransferTax(gross, params015)).toBeGreaterThanOrEqual(net)
    expect(netUlunaAfterTransferTax(gross - 1n, params015)).toBeLessThan(net)
    // Multiply tax can invert to a slightly smaller preimage than the sample withdraw.
    expect(gross).toBeLessThanOrEqual(withdraw)
  })

  it('formatBurnTaxPercentLabel renders percent labels', () => {
    expect(formatBurnTaxPercentLabel('0.015')).toBe('1.5%')
    expect(formatBurnTaxPercentLabel('0.01')).toBe('1%')
    expect(formatBurnTaxPercentLabel('0')).toBeNull()
  })
})
