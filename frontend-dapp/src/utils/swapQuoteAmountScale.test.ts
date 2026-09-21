import { describe, expect, it } from 'vitest'
import { fromRawAmount, getDecimals } from '@/utils/formatAmount'
import {
  isTheaterRouteQuote,
  swapAmountDecimals,
  swapAmountDecimalsFromAssetInfo,
  swapRouteSlippageBlocksSubmit,
} from '@/utils/swapQuoteAmountScale'
import { USDT_CW20_ADDRESS, USTR_CW20_ADDRESS } from '@/utils/tokenRegistry'

describe('swapQuoteAmountScale (#1257)', () => {
  it('pins 18 for registry USTR and unique-symbol USDT', () => {
    expect(swapAmountDecimals(USTR_CW20_ADDRESS)).toBe(18)
    expect(swapAmountDecimals(USDT_CW20_ADDRESS)).toBe(18)
    expect(swapAmountDecimals(USTR_CW20_ADDRESS.toUpperCase())).toBe(18)
    expect(getDecimals({ token: { contract_addr: USDT_CW20_ADDRESS } })).toBe(18)
    expect(swapAmountDecimalsFromAssetInfo({ token: { contract_addr: USDT_CW20_ADDRESS } })).toBe(18)
  })

  it('keeps getDecimals unknown→6 for non-Swap chrome; Swap resolver is fail-closed (#1255)', () => {
    expect(getDecimals({ token: { contract_addr: 'terra1unknown' } })).toBe(6)
    expect(swapAmountDecimals('terra1from00000000000000000000000000000001')).toBe(6)
  })

  it('does not print 9.091e15 USDT raw as 9.091B', () => {
    const raw = '9091000000000000'
    expect(fromRawAmount(raw, swapAmountDecimals(USDT_CW20_ADDRESS))).toBe('0.009091')
    expect(fromRawAmount(raw, 6)).toBe('9091000000')
  })

  it('formats 1 human USTR / USDT from 18-dec raw', () => {
    expect(fromRawAmount('1000000000000000000', swapAmountDecimals(USTR_CW20_ADDRESS))).toBe('1')
    expect(fromRawAmount('1000000000000000000', swapAmountDecimals(USDT_CW20_ADDRESS))).toBe('1')
  })

  it('treats ≥99% as theater; Expert does not waive', () => {
    expect(isTheaterRouteQuote(98.99)).toBe(false)
    expect(isTheaterRouteQuote(99)).toBe(true)
    expect(isTheaterRouteQuote(99.97)).toBe(true)
    expect(swapRouteSlippageBlocksSubmit(45, false)).toBe(true)
    expect(swapRouteSlippageBlocksSubmit(45, true)).toBe(false)
    expect(swapRouteSlippageBlocksSubmit(99.97, false)).toBe(true)
    expect(swapRouteSlippageBlocksSubmit(99.97, true)).toBe(true)
    expect(swapRouteSlippageBlocksSubmit(12, false)).toBe(false)
    expect(swapRouteSlippageBlocksSubmit(null, false)).toBe(false)
  })
})
