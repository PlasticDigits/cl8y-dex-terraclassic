import { describe, expect, it } from 'vitest'
import {
  evaluateLimitOrderNativeGasPlaceGate,
  LIMIT_ORDER_NATIVE_GAS_MSG_LOADING,
  LIMIT_ORDER_NATIVE_GAS_MSG_UNAVAILABLE,
  limitOrderNativeGasInsufficientMessage,
} from '@/utils/limitOrderNativeGasBalanceGate'

const REQUIRED = 33_000_000n // 33 LUNC in uluna — strictly above typical two-tx floor, below test “rich” balance

describe('evaluateLimitOrderNativeGasPlaceGate', () => {
  it('opens when human amount is empty or zero raw (escrow gate owns empty state)', () => {
    const q = { data: '0', isLoading: false, isError: false }
    expect(evaluateLimitOrderNativeGasPlaceGate('', 6, q, REQUIRED).canPlaceLimit).toBe(true)
    expect(evaluateLimitOrderNativeGasPlaceGate('0', 6, q, REQUIRED).userMessage).toBeNull()
  })

  it('blocks while LUNC balance is loading', () => {
    const r = evaluateLimitOrderNativeGasPlaceGate(
      '1',
      6,
      { data: undefined, isLoading: true, isError: false },
      REQUIRED
    )
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toBe(LIMIT_ORDER_NATIVE_GAS_MSG_LOADING)
    expect(r.tone).toBe('warning')
  })

  it('blocks when balance query errored or missing data', () => {
    expect(
      evaluateLimitOrderNativeGasPlaceGate('1', 6, { data: undefined, isLoading: false, isError: true }, REQUIRED)
        .userMessage
    ).toBe(LIMIT_ORDER_NATIVE_GAS_MSG_UNAVAILABLE)
    expect(
      evaluateLimitOrderNativeGasPlaceGate('1', 6, { data: undefined, isLoading: false, isError: false }, REQUIRED)
        .userMessage
    ).toBe(LIMIT_ORDER_NATIVE_GAS_MSG_UNAVAILABLE)
  })

  it('opens when balance meets required uluna', () => {
    const r = evaluateLimitOrderNativeGasPlaceGate(
      '1',
      6,
      { data: '50000000000', isLoading: false, isError: false },
      REQUIRED
    )
    expect(r.canPlaceLimit).toBe(true)
  })

  it('closes when balance is strictly below required', () => {
    const r = evaluateLimitOrderNativeGasPlaceGate(
      '1',
      6,
      { data: '1000000', isLoading: false, isError: false },
      REQUIRED
    )
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toContain('Need ~')
    expect(r.tone).toBe('error')
  })

  it('opens when balance equals required', () => {
    const r = evaluateLimitOrderNativeGasPlaceGate(
      '1',
      6,
      { data: String(REQUIRED), isLoading: false, isError: false },
      REQUIRED
    )
    expect(r.canPlaceLimit).toBe(true)
  })

  it('treats malformed balance conservatively', () => {
    const r = evaluateLimitOrderNativeGasPlaceGate('1', 6, { data: 'x', isLoading: false, isError: false }, REQUIRED)
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toBe(LIMIT_ORDER_NATIVE_GAS_MSG_UNAVAILABLE)
  })
})

describe('limitOrderNativeGasInsufficientMessage', () => {
  it('includes a human-readable LUNC estimate', () => {
    const msg = limitOrderNativeGasInsufficientMessage(32_573_750n)
    expect(msg).toMatch(/32\.574/)
    expect(msg).toMatch(/allowance \+ place order/)
  })
})
