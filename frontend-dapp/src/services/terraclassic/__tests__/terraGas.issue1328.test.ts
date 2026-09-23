import { describe, expect, it } from 'vitest'

import {
  CL8Y_TOKEN_ADDRESS,
  CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT,
  MAINNET_UST1_TOKEN_ADDRESS,
  UST1_TOKEN_ADDRESS,
} from '@/utils/constants'
import type { SwapOperation } from '../router'
import { estimateSwapNetworkFee } from '../swapNetworkFee'
import { RETAIL_COMBINED_ENVELOPE_FIXTURES } from '../terraGasRetailInventory'
import {
  getGasLimitForTx,
  gasLimitForRouterExecuteSwapOperations,
  sendHookExecuteSwapOperationsMsg,
  totalGasLimitForExecuteMsgs,
} from '../terraGas'

const CL8Y = CL8Y_TOKEN_ADDRESS
const UST1 = UST1_TOKEN_ADDRESS || MAINNET_UST1_TOKEN_ADDRESS
const MIDDLE = 'terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz'
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

describe('CL8Y → UST1 pool-only route gas (#1328)', () => {
  it('uses the measured Columbus-5 floor for the two-hop pool-only route', () => {
    const operations = poolOnlyRoute(CL8Y, UST1)

    expect(CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT).toBe(3_000_000)
    expect(CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT).toBeGreaterThan(2_643_980)
    expect(getGasLimitForTx(sendFor(operations))).toBe(CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT)
  })

  it('keeps Swap Network fee aligned with the submitted router operations', () => {
    const estimate = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: 2,
      cw20RouterOperations: poolOnlyRoute(CL8Y, UST1),
    })

    expect(estimate.gasLimit).toBe(CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT)
  })

  it('keeps the floor when a quote adds a zero book leg or a gas query field', () => {
    const zeroBookRoute = poolOnlyRoute(CL8Y, UST1).map((operation) => ({
      terra_swap: {
        ...operation.terra_swap,
        hybrid: { pool_input: '100', book_input: '0', max_maker_fills: 1 },
      },
    }))
    const msg = { ...sendFor(zeroBookRoute), gas: 1, gas_limit: 1 }

    expect(getGasLimitForTx(msg)).toBe(CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT)
  })

  it('leaves other routes and the reverse direction on their existing envelope', () => {
    const ordinary = getGasLimitForTx(sendFor(poolOnlyRoute(OTHER, UST1)))
    const reverse = getGasLimitForTx(sendFor(poolOnlyRoute(UST1, CL8Y)))

    expect(ordinary).toBe(gasLimitForRouterExecuteSwapOperations(2))
    expect(reverse).toBe(gasLimitForRouterExecuteSwapOperations(2))
    expect(ordinary).toBe(1_910_000)
  })

  it('leaves USTC → USTR wrap+2hop on the existing combo envelope', () => {
    const fixture = RETAIL_COMBINED_ENVELOPE_FIXTURES.find((entry) => entry.id === 'wrap_plus_send_2hop_ustc')!
    expect(totalGasLimitForExecuteMsgs(fixture.msgs)).toBe(2_710_000)
  })
})
