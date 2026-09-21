import { describe, expect, it } from 'vitest'
import {
  evaluateLimitOrderEscrowPlaceGate,
  LIMIT_ORDER_ESCROW_MSG_BALANCE_UNAVAILABLE,
  LIMIT_ORDER_ESCROW_MSG_INSUFFICIENT,
  LIMIT_ORDER_ESCROW_MSG_LOADING,
  LIMIT_ORDER_MIN_PLACE_MSG,
} from '@/utils/limitOrderEscrowBalanceGate'

describe('evaluateLimitOrderEscrowPlaceGate', () => {
  it('closes the gate on empty / zero human amount without a user message', () => {
    const q = { data: '1000000', isLoading: false, isError: false }
    expect(evaluateLimitOrderEscrowPlaceGate('', 6, q).canPlaceLimit).toBe(false)
    expect(evaluateLimitOrderEscrowPlaceGate('', 6, q).userMessage).toBeNull()
    expect(evaluateLimitOrderEscrowPlaceGate('0', 6, q).canPlaceLimit).toBe(false)
    expect(evaluateLimitOrderEscrowPlaceGate('0.0', 6, q).userMessage).toBeNull()
  })

  it('blocks while balance is loading', () => {
    const r = evaluateLimitOrderEscrowPlaceGate('1', 6, { data: undefined, isLoading: true, isError: false })
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toBe(LIMIT_ORDER_ESCROW_MSG_LOADING)
    expect(r.tone).toBe('warning')
  })

  it('blocks when balance query errored', () => {
    const r = evaluateLimitOrderEscrowPlaceGate('1', 6, { data: undefined, isLoading: false, isError: true })
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toBe(LIMIT_ORDER_ESCROW_MSG_BALANCE_UNAVAILABLE)
  })

  it('blocks when balance is still undefined (idle / disabled query)', () => {
    const r = evaluateLimitOrderEscrowPlaceGate('1', 6, { data: undefined, isLoading: false, isError: false })
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toBe(LIMIT_ORDER_ESCROW_MSG_BALANCE_UNAVAILABLE)
  })

  it('opens the gate when raw spend equals balance', () => {
    const r = evaluateLimitOrderEscrowPlaceGate('1', 6, { data: '1000000', isLoading: false, isError: false })
    expect(r.canPlaceLimit).toBe(true)
    expect(r.userMessage).toBeNull()
  })

  it('opens the gate when raw spend is strictly less than balance', () => {
    const r = evaluateLimitOrderEscrowPlaceGate('0.5', 6, { data: '1000000', isLoading: false, isError: false })
    expect(r.canPlaceLimit).toBe(true)
  })

  it('closes the gate with insufficient message when spend exceeds balance', () => {
    const r = evaluateLimitOrderEscrowPlaceGate('2', 6, { data: '1000000', isLoading: false, isError: false })
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toBe(LIMIT_ORDER_ESCROW_MSG_INSUFFICIENT)
    expect(r.tone).toBe('error')
  })

  it('treats zero wallet balance as insufficient for a place-sized spend', () => {
    const r = evaluateLimitOrderEscrowPlaceGate('0.000010', 6, { data: '0', isLoading: false, isError: false })
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toBe(LIMIT_ORDER_ESCROW_MSG_INSUFFICIENT)
  })

  it('closes the gate with named min-size when raw is 1–9 (Forgejo #1219)', () => {
    const q = { data: '1000000', isLoading: false, isError: false }
    const r1 = evaluateLimitOrderEscrowPlaceGate('0.000001', 6, q)
    expect(r1.canPlaceLimit).toBe(false)
    expect(r1.userMessage).toBe(LIMIT_ORDER_MIN_PLACE_MSG)
    const r9 = evaluateLimitOrderEscrowPlaceGate('0.000009', 6, q)
    expect(r9.canPlaceLimit).toBe(false)
    expect(r9.userMessage).toBe(LIMIT_ORDER_MIN_PLACE_MSG)
  })

  it('opens the gate at exactly 10 raw units when balance covers it', () => {
    const r = evaluateLimitOrderEscrowPlaceGate('0.000010', 6, {
      data: '10',
      isLoading: false,
      isError: false,
    })
    expect(r.canPlaceLimit).toBe(true)
    expect(r.userMessage).toBeNull()
  })

  it('handles malformed balance string conservatively', () => {
    const r = evaluateLimitOrderEscrowPlaceGate('1', 6, { data: 'not-a-number', isLoading: false, isError: false })
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toBe(LIMIT_ORDER_ESCROW_MSG_BALANCE_UNAVAILABLE)
  })
})
