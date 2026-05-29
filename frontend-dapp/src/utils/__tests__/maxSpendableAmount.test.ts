import { describe, it, expect } from 'vitest'
import { estimateFeeUlunaAmountForGasLimit, getGasLimitForTx } from '@/services/terraclassic/terraGas'
import {
  estimateLimitOrderPlaceSequenceUlunaFeesTotal,
  estimateMarketPairSwapSequenceUlunaFeesTotal,
  estimateNativeSwapUlunaFeesTotal,
  estimateProvideLiquidityCw20SequenceUlunaFeesTotal,
  estimateProvideLiquidityNativeWrapUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import { computeMaxSpendableHumanAmount, maxAmountReserveUlunaForContext } from '@/utils/maxSpendableAmount'
import { isDecimalAmountDraft } from '@/utils/decimalAmountInput'

describe('maxSpendableAmount (GitLab #213)', () => {
  const balance = '10000000000' // 10_000 LUNC @ 6 decimals

  it('CW20 swap Max uses full balance without gas reserve', () => {
    const result = computeMaxSpendableHumanAmount({
      balanceRaw: balance,
      decimals: 6,
      assetIsNativeUluna: false,
      context: 'swap_cw20',
    })
    expect(result.human).toBe('10000')
    expect(result.spendableRaw).toBe(10_000_000_000n)
    expect(result.cappedByGas).toBe(false)
    expect(result.reserveUluna).toBe(0n)
  })

  it('native swap Max subtracts wrap_deposit fee envelope for direct wrap', () => {
    const reserve = estimateNativeSwapUlunaFeesTotal({ isDirectWrap: true, needsWrapInput: false })
    const result = computeMaxSpendableHumanAmount({
      balanceRaw: balance,
      decimals: 6,
      assetIsNativeUluna: true,
      context: 'swap_native',
      nativeSwapHints: { isDirectWrap: true, needsWrapInput: false },
    })
    expect(result.reserveUluna).toBe(reserve)
    expect(result.spendableRaw).toBe(10_000_000_000n - reserve)
    expect(result.cappedByGas).toBe(true)
    expect(isDecimalAmountDraft(result.human)).toBe(true)
  })

  it('native swap Max subtracts wrap + router send combined fee', () => {
    const reserve = estimateNativeSwapUlunaFeesTotal({ isDirectWrap: false, needsWrapInput: true, hopCount: 1 })
    const result = computeMaxSpendableHumanAmount({
      balanceRaw: balance,
      decimals: 6,
      assetIsNativeUluna: true,
      context: 'swap_native',
      nativeSwapHints: { isDirectWrap: false, needsWrapInput: true, hopCount: 1 },
    })
    expect(result.reserveUluna).toBe(reserve)
    expect(result.spendableRaw).toBe(10_000_000_000n - reserve)
  })

  it('limit place CW20 Max stays full balance', () => {
    const result = computeMaxSpendableHumanAmount({
      balanceRaw: '5000000',
      decimals: 6,
      assetIsNativeUluna: false,
      context: 'limit_place',
    })
    expect(result.human).toBe('5')
    expect(result.reserveUluna).toBe(0n)
  })

  it('market swap Max stays full CW20 balance', () => {
    const result = computeMaxSpendableHumanAmount({
      balanceRaw: '2500000',
      decimals: 6,
      assetIsNativeUluna: false,
      context: 'market_swap',
      marketUsesHybrid: true,
    })
    expect(result.human).toBe('2.5')
    expect(result.cappedByGas).toBe(false)
  })

  it('pool native wrap Max subtracts combined multi-msg fee', () => {
    const reserve = estimateProvideLiquidityNativeWrapUlunaFeesTotal(1)
    const result = computeMaxSpendableHumanAmount({
      balanceRaw: balance,
      decimals: 6,
      assetIsNativeUluna: true,
      context: 'provide_liquidity_native_side',
      nativeWrapDepositCount: 1,
    })
    expect(result.reserveUluna).toBe(reserve)
    expect(result.spendableRaw).toBe(10_000_000_000n - reserve)
  })

  it('pool CW20 Max uses full balance', () => {
    const result = computeMaxSpendableHumanAmount({
      balanceRaw: '1234567',
      decimals: 6,
      assetIsNativeUluna: false,
      context: 'provide_liquidity_cw20',
    })
    expect(result.human).toBe('1.234567')
    expect(result.reserveUluna).toBe(0n)
  })

  it('book leg Max caps to pay amount and balance', () => {
    const result = computeMaxSpendableHumanAmount({
      balanceRaw: '5000000',
      decimals: 6,
      assetIsNativeUluna: false,
      context: 'book_leg',
      payAmountRaw: '2000000',
    })
    expect(result.human).toBe('2')
    expect(result.spendableRaw).toBe(2_000_000n)
  })

  it('does not subtract LUNC reserve from CW20 amount (cross-asset guard)', () => {
    const result = computeMaxSpendableHumanAmount({
      balanceRaw: '1000000',
      decimals: 6,
      assetIsNativeUluna: false,
      context: 'swap_native',
    })
    expect(result.spendableRaw).toBe(1_000_000n)
    expect(result.reserveUluna).toBe(0n)
  })

  it('floors native Max at zero when balance below reserve', () => {
    const reserve = estimateNativeSwapUlunaFeesTotal({ isDirectWrap: true, needsWrapInput: false })
    const tiny = (reserve - 1n).toString()
    const result = computeMaxSpendableHumanAmount({
      balanceRaw: tiny,
      decimals: 6,
      assetIsNativeUluna: true,
      context: 'swap_native',
      nativeSwapHints: { isDirectWrap: true, needsWrapInput: false },
    })
    expect(result.spendableRaw).toBe(0n)
    expect(result.human).toBe('0')
  })

  describe('maxAmountReserveUlunaForContext regression vs terraGas', () => {
    it('swap_native direct wrap reserve >= single-tx fee estimate', () => {
      const reserve = maxAmountReserveUlunaForContext('swap_native', {
        nativeSwapHints: { isDirectWrap: true, needsWrapInput: false },
      })
      const min = estimateFeeUlunaAmountForGasLimit(getGasLimitForTx({ wrap_deposit: {} }))
      expect(reserve).toBeGreaterThanOrEqual(min)
    })

    it('limit_place reserve matches estimateLimitOrderPlaceSequenceUlunaFeesTotal', () => {
      expect(maxAmountReserveUlunaForContext('limit_place')).toBe(estimateLimitOrderPlaceSequenceUlunaFeesTotal())
    })

    it('market_swap reserve matches estimateMarketPairSwapSequenceUlunaFeesTotal', () => {
      expect(maxAmountReserveUlunaForContext('market_swap', { marketUsesHybrid: true })).toBe(
        estimateMarketPairSwapSequenceUlunaFeesTotal(true)
      )
    })

    it('provide_liquidity_cw20 reserve matches three-tx sequence (native gate only)', () => {
      expect(maxAmountReserveUlunaForContext('provide_liquidity_cw20')).toBe(
        estimateProvideLiquidityCw20SequenceUlunaFeesTotal()
      )
    })

    it('native wrap provide reserve is positive and matches helper', () => {
      const reserve = maxAmountReserveUlunaForContext('provide_liquidity_native_side', {
        nativeWrapDepositCount: 2,
      })
      expect(reserve).toBe(estimateProvideLiquidityNativeWrapUlunaFeesTotal(2))
      expect(reserve).toBeGreaterThan(0n)
    })
  })
})
