import { describe, it, expect } from 'vitest'
import {
  computeNativeTransferTaxUluna,
  netUlunaAfterTransferTax,
  type NativeTransferTaxParams,
} from '@/utils/nativeTransferTax'

describe('nativeTransferTax', () => {
  const params005: NativeTransferTaxParams = { rate: '0.0005', capUluna: 1_000_000_000_000_000n }

  it('computes net minted CW20 after classic burn tax (#342)', () => {
    const gross = 10_000_000n
    const net = netUlunaAfterTransferTax(gross, params005)
    expect(net).toBe(9_995_002n)
    expect(computeNativeTransferTaxUluna(gross, params005)).toBe(gross - net)
  })

  it('returns gross when tax rate is zero', () => {
    const gross = 1_000_000n
    expect(netUlunaAfterTransferTax(gross, { rate: '0', capUluna: 0n })).toBe(gross)
  })
})
