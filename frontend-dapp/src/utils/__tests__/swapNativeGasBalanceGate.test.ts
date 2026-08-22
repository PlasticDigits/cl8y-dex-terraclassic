import { describe, expect, it } from 'vitest'
import {
  SWAP_NATIVE_GAS_MSG_LOADING,
  SWAP_NATIVE_GAS_MSG_UNAVAILABLE,
  evaluateSwapNativeGasGate,
} from '@/utils/swapNativeGasBalanceGate'

describe('evaluateSwapNativeGasGate (GitLab #587)', () => {
  const fee = 65_000_000n

  it('opens when amount is empty', () => {
    const r = evaluateSwapNativeGasGate('', 6, true, '0', { data: '1000', isLoading: false, isError: false }, fee)
    expect(r.canSubmit).toBe(true)
    expect(r.userMessage).toBeNull()
  })

  it('closes while LUNC balance is loading', () => {
    const r = evaluateSwapNativeGasGate('1', 6, true, '1000000', { isLoading: true, isError: false }, fee)
    expect(r.canSubmit).toBe(false)
    expect(r.userMessage).toBe(SWAP_NATIVE_GAS_MSG_LOADING)
  })

  it('closes when LUNC balance is unreadable', () => {
    const r = evaluateSwapNativeGasGate('1', 6, false, '1000000', { isError: true, isLoading: false }, fee)
    expect(r.canSubmit).toBe(false)
    expect(r.userMessage).toBe(SWAP_NATIVE_GAS_MSG_UNAVAILABLE)
  })

  it('requires pay + fee when paying native LUNC', () => {
    const pay = 1_000_000n
    const bank = pay + fee - 1n
    const r = evaluateSwapNativeGasGate(
      '1',
      6,
      true,
      pay.toString(),
      { data: bank.toString(), isLoading: false, isError: false },
      fee
    )
    expect(r.canSubmit).toBe(false)
    expect(r.userMessage).toMatch(/LUNC/)
    expect(r.userMessage).not.toMatch(/USTC/)
  })

  it('opens when bank covers pay + fee', () => {
    const pay = 1_000_000n
    const bank = pay + fee
    const r = evaluateSwapNativeGasGate(
      '1',
      6,
      true,
      pay.toString(),
      { data: bank.toString(), isLoading: false, isError: false },
      fee
    )
    expect(r.canSubmit).toBe(true)
  })

  it('CW20 pay only requires the fee envelope', () => {
    const r = evaluateSwapNativeGasGate(
      '1',
      6,
      false,
      '999999999',
      { data: fee.toString(), isLoading: false, isError: false },
      fee
    )
    expect(r.canSubmit).toBe(true)
  })
})
