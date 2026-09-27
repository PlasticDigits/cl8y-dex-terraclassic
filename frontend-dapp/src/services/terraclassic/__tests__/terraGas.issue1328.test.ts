import { describe, expect, it } from 'vitest'

import { CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT } from '@/utils/constants'
import type { SwapOperation } from '../router'
import { estimateSwapNetworkFee } from '../swapNetworkFee'
import { RETAIL_COMBINED_ENVELOPE_FIXTURES } from '../terraGasRetailInventory'
import { autoGasWantedFromUsed } from '../swapAutoGas'
import {
  getGasLimitForTx,
  gasLimitForRouterExecuteSwapOperations,
  sendHookExecuteSwapOperationsMsg,
  totalGasLimitForExecuteMsgs,
} from '../terraGas'

const MIDDLE = 'terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz'
const CL8Y = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
const OTHER = 'terra1other0000000000000000000000000000000000'

function poolOnlyRoute(from: string, to: string): SwapOperation[] {
  return [
    {
      terra_swap: {
        offer_asset_info: { token: { contract_addr: from } },
        ask_asset_info: { token: { contract_addr: MIDDLE } },
        hybrid: null,
      },
    },
    {
      terra_swap: {
        offer_asset_info: { token: { contract_addr: MIDDLE } },
        ask_asset_info: { token: { contract_addr: to } },
        hybrid: null,
      },
    },
  ]
}

function sendFor(operations: SwapOperation[]) {
  return sendHookExecuteSwapOperationsMsg(operations as unknown as Array<{ terra_swap: Record<string, unknown> }>)
}

describe('CL8Y → UST1 pool-only route gas (#1328 / #1360)', () => {
  it('keeps the 3M figure as a historical measurement and does not use it as the fee', () => {
    const operations = poolOnlyRoute(CL8Y, UST1)

    expect(CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT).toBe(3_000_000)
    expect(CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT).toBeGreaterThan(2_643_980)
    expect(getGasLimitForTx(sendFor(operations))).toBe(gasLimitForRouterExecuteSwapOperations(2))
    expect(getGasLimitForTx(sendFor(operations))).toBe(1_910_000)
    expect(getGasLimitForTx(sendFor(operations))).not.toBe(CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT)
  })

  it('signs ceil(sim × 1.2) for a successful simulate, not the retired pair floor', () => {
    const wanted = autoGasWantedFromUsed(2_643_980)
    expect(wanted).toBe(Math.ceil(2_643_980 * 1.2))
    expect(wanted).not.toBe(CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT)
  })

  it('keeps Swap Network fee fallback aligned with the generic two-hop envelope', () => {
    const estimate = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: 2,
      cw20RouterOperations: poolOnlyRoute(CL8Y, UST1),
    })

    expect(estimate.gasLimit).toBe(1_910_000)
  })

  it('does not raise the envelope when a quote adds a zero book leg or a gas query field', () => {
    const zeroBookRoute = poolOnlyRoute(CL8Y, UST1).map((operation) => ({
      terra_swap: {
        ...operation.terra_swap,
        hybrid: { pool_input: '100', book_input: '0', max_maker_fills: 1 },
      },
    }))
    const msg = { ...sendFor(zeroBookRoute), gas: 1, gas_limit: 1 }

    expect(getGasLimitForTx(msg)).toBe(1_910_000)
  })

  it('uses the same fallback for other routes, the reverse direction, and a different ask token', () => {
    const cl8yUst1 = getGasLimitForTx(sendFor(poolOnlyRoute(CL8Y, UST1)))
    const ordinary = getGasLimitForTx(sendFor(poolOnlyRoute(OTHER, UST1)))
    const reverse = getGasLimitForTx(sendFor(poolOnlyRoute(UST1, CL8Y)))
    const otherAsk = getGasLimitForTx(sendFor(poolOnlyRoute(CL8Y, OTHER)))

    expect(cl8yUst1).toBe(ordinary)
    expect(reverse).toBe(ordinary)
    expect(otherAsk).toBe(ordinary)
    expect(ordinary).toBe(1_910_000)
    expect(autoGasWantedFromUsed(1_937_976)).toBe(2_325_572)
  })

  it('leaves USTC → USTR wrap+2hop on the existing combo envelope', () => {
    const fixture = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((entry) => entry.id === 'wrap_plus_send_2hop_ustc')!
    expect(totalGasLimitForExecuteMsgs(fixture.msgs)).toBe(2_710_000)
  })
})
