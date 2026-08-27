import { describe, expect, it } from 'vitest'
import { MIN_GAS_PRICE_ULUNA, MIXED_HYBRID_ROUTER_HEADROOM_GAS } from '@/utils/constants'
import {
  HYBRID_SWAP_GAS_LIMIT,
  SendHookGasDecodeError,
  buildTerraClassicFee,
  getGasLimitForTx,
  sendHookExecuteSwapOperationsMsg,
} from '../terraGas'
import { SEND_4HOP_HYBRID_FIRST_POOL_REST_GAS } from '../terraGasRetailInventory'
import { gasLimitForHybridParams } from '../hybridSwapGas'
import { estimateSwapNetworkFee, estimateTradeMarketNetworkFeeUluna } from '../swapNetworkFee'
import { estimateTerraClassicFeeForMsg } from '../terraClassicFeeEstimate'
import type { SwapOperation } from '../router'

const COLUMBUS_5_GAS_USED = 5_026_176
const LEGACY_15M_PER_POOL_HOP = 46_785_500

function terraSwap(hybrid?: Record<string, unknown>) {
  return {
    terra_swap: {
      offer_asset_info: { token: { contract_addr: 'terra1offer' } },
      ask_asset_info: { token: { contract_addr: 'terra1ask' } },
      ...(hybrid ? { hybrid } : {}),
    },
  }
}

const hop1Book8 = {
  pool_input: '0',
  book_input: '10000000000',
  max_maker_fills: 8,
  book_start_hint: 1426,
}

function sendOps(operations: Array<{ terra_swap: Record<string, unknown> }>): Record<string, unknown> {
  return sendHookExecuteSwapOperationsMsg(operations)
}

describe('mixed hybrid router gas (GitLab #679)', () => {
  it('G-AUTO-1: 4-hop hybrid-first + pool rest is above live used and below 15M', () => {
    const msg = sendOps([terraSwap(hop1Book8), terraSwap(), terraSwap(), terraSwap()])
    const gas = getGasLimitForTx(msg)
    expect(gas).toBe(SEND_4HOP_HYBRID_FIRST_POOL_REST_GAS)
    expect(gas).toBe(6_785_500)
    expect(gas).toBeGreaterThan(COLUMBUS_5_GAS_USED)
    expect(gas).toBeLessThan(HYBRID_SWAP_GAS_LIMIT)
    expect(gas).not.toBe(LEGACY_15M_PER_POOL_HOP)
    const fee = buildTerraClassicFee(gas)
    expect(fee.amount[0]?.denom).toBe('uluna')
    expect(Number(fee.amount[0]?.amount) / 1_000_000).toBeLessThan(400)
    expect(Number(fee.amount[0]?.amount) / 1_000_000).toBeCloseTo(192.199, 2)
    expect(fee.amount[0]?.amount).toBe(String(Math.ceil(MIN_GAS_PRICE_ULUNA * gas)))
  })

  it('G-AUTO-3: hops without hybrid never add HYBRID_SWAP_GAS_LIMIT', () => {
    const gas = getGasLimitForTx(sendOps([terraSwap(hop1Book8), terraSwap(), terraSwap(), terraSwap()]))
    const hop1 = gasLimitForHybridParams({
      pool_input: '0',
      book_input: '10000000000',
      max_maker_fills: 8,
      book_start_hint: 1426,
    })
    expect(gas).toBe(hop1 + 3 * 950_000 + MIXED_HYBRID_ROUTER_HEADROOM_GAS)
    expect(gas).toBeLessThan(hop1 + HYBRID_SWAP_GAS_LIMIT)
  })

  it('pool-only 2-hop / 4-hop keep router floors', () => {
    expect(
      getGasLimitForTx({
        execute_swap_operations: { operations: [terraSwap(), terraSwap()] },
      })
    ).toBe(1_910_000)
    expect(
      getGasLimitForTx({
        execute_swap_operations: { operations: [terraSwap(), terraSwap(), terraSwap(), terraSwap()] },
      })
    ).toBe(3_810_000)
  })

  it('G-AUTO-5: hybrid on every hop of a 2-hop stays 3,058,600', () => {
    const hybrid = { pool_input: '1', book_input: '1', max_maker_fills: 4 }
    expect(
      getGasLimitForTx({
        execute_swap_operations: { operations: [terraSwap(hybrid), terraSwap(hybrid)] },
      })
    ).toBe(3_058_600)
  })

  it('hybrid first hop + one pool hop is quote-driven + pool, not 15M + hop1', () => {
    const gas = getGasLimitForTx({
      execute_swap_operations: { operations: [terraSwap(hop1Book8), terraSwap()] },
    })
    const hop1 = gasLimitForHybridParams({
      pool_input: '0',
      book_input: '10000000000',
      max_maker_fills: 8,
      book_start_hint: 1426,
    })
    expect(gas).toBe(hop1 + 950_000 + MIXED_HYBRID_ROUTER_HEADROOM_GAS)
    expect(gas).not.toBe(hop1 + HYBRID_SWAP_GAS_LIMIT)
    expect(gas).toBeLessThan(HYBRID_SWAP_GAS_LIMIT)
  })

  it('unknown hybrid on all hops is a single 15M fallback', () => {
    expect(
      getGasLimitForTx({
        execute_swap_operations: {
          operations: [terraSwap({}), terraSwap({}), terraSwap({})],
        },
      })
    ).toBe(HYBRID_SWAP_GAS_LIMIT)
  })

  it('G-AUTO-6: direct pair swap book_input=0 is 840k; book_input>0 is quote-driven', () => {
    expect(
      getGasLimitForTx({
        swap: { hybrid: { pool_input: '1', book_input: '0', max_maker_fills: 8 } },
      })
    ).toBe(840_000)
    expect(
      getGasLimitForTx({
        swap: { hybrid: { pool_input: '0', book_input: '1', max_maker_fills: 8 } },
      })
    ).toBe(1_785_500)
    expect(
      getGasLimitForTx({
        swap: { hybrid: { pool_input: '0', book_input: '1', max_maker_fills: 8 } },
      })
    ).not.toBe(HYBRID_SWAP_GAS_LIMIT)
  })

  it('malformed send.msg throws SendHookGasDecodeError', () => {
    expect(() => getGasLimitForTx({ send: { msg: '%%%not-base64%%%' } })).toThrow(SendHookGasDecodeError)
  })

  it('G-AUTO-2: estimateSwapNetworkFee with indexer ops matches broadcast send envelope', () => {
    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: 'terra1a' } },
          ask_asset_info: { token: { contract_addr: 'terra1b' } },
          hybrid: hop1Book8,
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: 'terra1b' } },
          ask_asset_info: { token: { contract_addr: 'terra1c' } },
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: 'terra1c' } },
          ask_asset_info: { token: { contract_addr: 'terra1d' } },
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: 'terra1d' } },
          ask_asset_info: { token: { contract_addr: 'terra1e' } },
        },
      },
    ]
    const hint = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: 4,
      cw20RouterOperations: ops,
    })
    const broadcast = estimateTerraClassicFeeForMsg(
      sendOps([terraSwap(hop1Book8), terraSwap(), terraSwap(), terraSwap()])
    )
    expect(hint.gasLimit).toBe(broadcast.gasLimit)
    expect(hint.feeUluna).toBe(broadcast.feeUluna)
    expect(hint.gasLimit).toBe(6_785_500)
    expect(Number(hint.feeUluna) / 1_000_000).toBeLessThan(400)
  })

  it('pool-only 2-hop hint without ops stays on hopCount floor', () => {
    const hint = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: 2,
    })
    expect(hint.gasLimit).toBe(1_910_000)
  })

  it('wrap+2hop is unchanged and cw20Hybrid must not jump to 15M', () => {
    const est = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: true,
      hopCount: 2,
      cw20Hybrid: true,
      cw20RouterOperations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: 'terra1a' } },
            ask_asset_info: { token: { contract_addr: 'terra1b' } },
            hybrid: hop1Book8,
          },
        },
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: 'terra1b' } },
            ask_asset_info: { token: { contract_addr: 'terra1c' } },
          },
        },
      ],
    })
    expect(est.gasLimit).toBe(2_710_000)
    expect(est.gasLimit).toBeLessThan(HYBRID_SWAP_GAS_LIMIT)
  })

  it('Trade market multi-hop shares the mixed-hop helper (allowance + send)', () => {
    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: 'terra1a' } },
          ask_asset_info: { token: { contract_addr: 'terra1b' } },
          hybrid: hop1Book8,
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: 'terra1b' } },
          ask_asset_info: { token: { contract_addr: 'terra1c' } },
        },
      },
    ]
    const total = estimateTradeMarketNetworkFeeUluna(ops)
    const swap = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: 2,
      cw20RouterOperations: ops,
    })
    const allowance = estimateTerraClassicFeeForMsg({ increase_allowance: { spender: '', amount: '' } })
    expect(total).toBe(allowance.feeUluna + swap.feeUluna)
  })
})
