import { describe, expect, it } from 'vitest'
import {
  evaluateProvideLiquidityCw20NativeGasGate,
  PROVIDE_LIQ_NATIVE_GAS_MSG_LOADING,
  PROVIDE_LIQ_NATIVE_GAS_MSG_UNAVAILABLE,
  provideLiquidityCw20NativeGasInsufficientMessage,
} from '@/utils/provideLiquidityNativeGasBalanceGate'

/** Above GitLab #147 three-tx floor (~25.5 LUNC), below “rich” test balance */
const REQUIRED = 33_000_000n

describe('evaluateProvideLiquidityCw20NativeGasGate', () => {
  it('opens when either raw amount is zero (submit gated elsewhere)', () => {
    const q = { data: '0', isLoading: false, isError: false }
    expect(evaluateProvideLiquidityCw20NativeGasGate('', '1', 6, 6, q, REQUIRED).canAddLiquidity).toBe(true)
    expect(evaluateProvideLiquidityCw20NativeGasGate('1', '', 6, 6, q, REQUIRED).canAddLiquidity).toBe(true)
    expect(evaluateProvideLiquidityCw20NativeGasGate('0', '1', 6, 6, q, REQUIRED).canAddLiquidity).toBe(true)
    expect(evaluateProvideLiquidityCw20NativeGasGate('1', '0', 6, 6, q, REQUIRED).canAddLiquidity).toBe(true)
  })

  it('blocks while LUNC balance is loading', () => {
    const r = evaluateProvideLiquidityCw20NativeGasGate(
      '1',
      '1',
      6,
      6,
      { data: undefined, isLoading: true, isError: false },
      REQUIRED
    )
    expect(r.canAddLiquidity).toBe(false)
    expect(r.userMessage).toBe(PROVIDE_LIQ_NATIVE_GAS_MSG_LOADING)
    expect(r.tone).toBe('warning')
  })

  it('blocks when balance query errored or missing data', () => {
    expect(
      evaluateProvideLiquidityCw20NativeGasGate(
        '1',
        '1',
        6,
        6,
        { data: undefined, isLoading: false, isError: true },
        REQUIRED
      ).userMessage
    ).toBe(PROVIDE_LIQ_NATIVE_GAS_MSG_UNAVAILABLE)
    expect(
      evaluateProvideLiquidityCw20NativeGasGate(
        '1',
        '1',
        6,
        6,
        { data: undefined, isLoading: false, isError: false },
        REQUIRED
      ).userMessage
    ).toBe(PROVIDE_LIQ_NATIVE_GAS_MSG_UNAVAILABLE)
  })

  it('opens when balance meets required uluna', () => {
    const r = evaluateProvideLiquidityCw20NativeGasGate(
      '1',
      '1',
      6,
      6,
      { data: '50000000000', isLoading: false, isError: false },
      REQUIRED
    )
    expect(r.canAddLiquidity).toBe(true)
  })

  it('closes when balance is strictly below required', () => {
    const r = evaluateProvideLiquidityCw20NativeGasGate(
      '1',
      '1',
      6,
      6,
      { data: '1000000', isLoading: false, isError: false },
      REQUIRED
    )
    expect(r.canAddLiquidity).toBe(false)
    expect(r.userMessage).toContain('Not enough LUNC')
    expect(r.tone).toBe('error')
  })

  it('opens when balance equals required', () => {
    const r = evaluateProvideLiquidityCw20NativeGasGate(
      '1',
      '1',
      6,
      6,
      { data: String(REQUIRED), isLoading: false, isError: false },
      REQUIRED
    )
    expect(r.canAddLiquidity).toBe(true)
  })

  it('treats malformed balance conservatively', () => {
    const r = evaluateProvideLiquidityCw20NativeGasGate(
      '1',
      '1',
      6,
      6,
      { data: 'x', isLoading: false, isError: false },
      REQUIRED
    )
    expect(r.canAddLiquidity).toBe(false)
    expect(r.userMessage).toBe(PROVIDE_LIQ_NATIVE_GAS_MSG_UNAVAILABLE)
  })
})

describe('provideLiquidityCw20NativeGasInsufficientMessage', () => {
  it('includes a human-readable LUNC estimate and three-tx wording', () => {
    const msg = provideLiquidityCw20NativeGasInsufficientMessage(25_492_500n)
    expect(msg).toMatch(/25\.492/)
    expect(msg).toMatch(/allowance A \+ allowance B \+ provide liquidity/)
  })
})
