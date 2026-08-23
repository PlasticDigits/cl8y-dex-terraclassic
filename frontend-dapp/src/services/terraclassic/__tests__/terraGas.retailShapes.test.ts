import { afterEach, describe, expect, it, vi } from 'vitest'
import { UNWRAP_GAS_LIMIT } from '@/utils/constants'
import { RETAIL_COMBINED_ENVELOPE_FIXTURES, RETAIL_GAS_SHAPE_FIXTURES } from '../terraGasRetailInventory'
import {
  BASE_GAS_LIMIT,
  BASE_GAS_LIMIT_ALLOWLIST,
  FAUCET_DRIP_GAS_LIMIT,
  SWAP_GAS_LIMIT,
  SendHookGasDecodeError,
  gasLimitForRouterExecuteSwapOperations,
  getGasLimitForTx,
  isBaseGasLimitAllowlisted,
  primaryExecuteMsgKey,
  totalGasLimitForExecuteMsgs,
} from '../terraGas'
import { UNWRAP_ROUTER_COMBO_OVERHEAD_GAS, WRAP_GAS_LIMIT, WRAP_ROUTER_COMBO_OVERHEAD_GAS } from '@/utils/constants'

/**
 * Soft-launch faucet drip LocalTerra ballpark (CW20 Mint submsg).
 * Constant must stay above this floor; refine via `make verify-issue-475` live rung.
 */
const MEASURED_FAUCET_DRIP_GAS_FLOOR = 248_000

describe('retail gas inventory guardrail (GitLab #475)', () => {
  it('allowlist is only CW20 allowance adjusts', () => {
    expect([...BASE_GAS_LIMIT_ALLOWLIST].sort()).toEqual(['decrease_allowance', 'increase_allowance'].sort())
  })

  it.each(RETAIL_GAS_SHAPE_FIXTURES)('maps $id ($note) to expected envelope', ({ msg, expectedGas }) => {
    const limit = getGasLimitForTx(msg)
    expect(limit).toBe(expectedGas)

    const key = primaryExecuteMsgKey(msg)
    if (isBaseGasLimitAllowlisted(key)) {
      expect(limit).toBe(BASE_GAS_LIMIT)
    } else {
      expect(limit).toBeGreaterThan(BASE_GAS_LIMIT)
    }
  })

  it('drip does not fall through to BASE_GAS_LIMIT (#474)', () => {
    const limit = getGasLimitForTx({ drip: { token: 'terra1token' } })
    expect(limit).toBe(FAUCET_DRIP_GAS_LIMIT)
    expect(limit).toBeGreaterThan(BASE_GAS_LIMIT)
    expect(limit).toBeGreaterThan(MEASURED_FAUCET_DRIP_GAS_FLOOR)
  })

  it('send→unwrap uses UNWRAP_GAS_LIMIT (not legacy SWAP_GAS_LIMIT)', () => {
    const inner = btoa(JSON.stringify({ unwrap: { recipient: null } }))
    expect(getGasLimitForTx({ send: { msg: inner } })).toBe(UNWRAP_GAS_LIMIT)
  })

  it('unknown synthetic msg falls back to BASE_GAS_LIMIT', () => {
    expect(getGasLimitForTx({ unknown_action: {} })).toBe(BASE_GAS_LIMIT)
  })

  it('send without inner msg still uses SWAP_GAS_LIMIT', () => {
    expect(getGasLimitForTx({ send: {} })).toBe(SWAP_GAS_LIMIT)
  })

  it('malformed send.msg throws instead of granting 600k (#587)', () => {
    expect(() => getGasLimitForTx({ send: { msg: '!!!invalid!!!' } })).toThrow(SendHookGasDecodeError)
  })
})

describe('combined wrap+router envelopes (GitLab #587)', () => {
  it.each(RETAIL_COMBINED_ENVELOPE_FIXTURES)('maps $id ($note) to expected envelope', ({ msgs, expectedGas }) => {
    expect(totalGasLimitForExecuteMsgs(msgs)).toBe(expectedGas)
  })

  it('wrap+2hop exceeds gem-calibrated 2.31M and never 1.0M fallback', () => {
    const wrap2 = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((f) => f.id === 'wrap_plus_send_2hop')!
    const gas = totalGasLimitForExecuteMsgs(wrap2.msgs)
    expect(gas).toBeGreaterThan(2_310_000)
    expect(gas).toBe(WRAP_GAS_LIMIT + gasLimitForRouterExecuteSwapOperations(2) + WRAP_ROUTER_COMBO_OVERHEAD_GAS)
    expect(gas).not.toBe(WRAP_GAS_LIMIT + SWAP_GAS_LIMIT)
    expect(gas).toBeLessThan(15_000_000)
  })

  it('wrap+2hop+unwrap exceeds wrap+2hop', () => {
    const a = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((f) => f.id === 'wrap_plus_send_2hop')!
    const b = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((f) => f.id === 'wrap_plus_send_2hop_unwrap')!
    expect(totalGasLimitForExecuteMsgs(b.msgs)).toBeGreaterThan(totalGasLimitForExecuteMsgs(a.msgs))
  })

  it('wrap+1hop stays #353 1.8M (no combo overhead)', () => {
    const wrap1 = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((f) => f.id === 'wrap_plus_send_1hop')!
    expect(totalGasLimitForExecuteMsgs(wrap1.msgs)).toBe(WRAP_GAS_LIMIT + gasLimitForRouterExecuteSwapOperations(1))
  })

  it('wrap+2hop+invoice Send exceeds wrap+2hop swap-only (#595)', () => {
    const swapOnly = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((f) => f.id === 'wrap_plus_send_2hop')!
    const invoice = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((f) => f.id === 'wrap_plus_2hop_plus_invoice_send')!
    expect(totalGasLimitForExecuteMsgs(invoice.msgs)).toBeGreaterThan(totalGasLimitForExecuteMsgs(swapOnly.msgs))
  })
})

describe('unwrap+≥2hop combo (GitLab #599)', () => {
  it('USTR→USTC / USTR→LUNC share the unwrap combo and exceed the 2.71M OOG sum', () => {
    const lunc = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((f) => f.id === 'send_2hop_unwrap')!
    const ustc = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((f) => f.id === 'send_2hop_unwrap_ustc')!
    const gas = totalGasLimitForExecuteMsgs(ustc.msgs)
    expect(gas).toBe(totalGasLimitForExecuteMsgs(lunc.msgs))
    expect(gas).toBe(gasLimitForRouterExecuteSwapOperations(2) + UNWRAP_GAS_LIMIT + UNWRAP_ROUTER_COMBO_OVERHEAD_GAS)
    expect(gas).toBe(3_110_000)
    expect(gas).toBeGreaterThan(2_710_000)
    expect(gas).toBeLessThan(15_000_000)
  })

  it('direct mapper unwrap stays 800k (no hub-multihop combo)', () => {
    const unwrap = RETAIL_GAS_SHAPE_FIXTURES.find((f) => f.id === 'send_inner_unwrap')!
    expect(getGasLimitForTx(unwrap.msg)).toBe(UNWRAP_GAS_LIMIT)
    expect(getGasLimitForTx(unwrap.msg)).toBe(800_000)
  })

  it('router 1-hop + unwrap does not pay unwrap combo', () => {
    const hop1 = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((f) => f.id === 'send_1hop_unwrap')!
    expect(totalGasLimitForExecuteMsgs(hop1.msgs)).toBe(gasLimitForRouterExecuteSwapOperations(1) + UNWRAP_GAS_LIMIT)
  })

  it('unwrap+2hop fee stays tens-of-LUNC class at 28.325', () => {
    const ustc = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((f) => f.id === 'send_2hop_unwrap_ustc')!
    const gas = totalGasLimitForExecuteMsgs(ustc.msgs)
    expect((gas * 28.325) / 1_000_000).toBeLessThan(100)
  })
})

describe('unmapped gas fallback warn (GitLab #475)', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('warns in DEV for unrecognized keys', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    getGasLimitForTx({ totally_new_retail_msg: {} })
    expect(warn).toHaveBeenCalledWith(expect.stringContaining('unmapped execute msg'))
    expect(warn).toHaveBeenCalledWith(expect.stringContaining('totally_new_retail_msg'))
  })

  it('does not warn for allowlisted BASE keys', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    getGasLimitForTx({ increase_allowance: { spender: 'x', amount: '1' } })
    expect(warn).not.toHaveBeenCalled()
  })
})
