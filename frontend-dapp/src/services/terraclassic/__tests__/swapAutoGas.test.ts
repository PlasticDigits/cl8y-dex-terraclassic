import { afterEach, describe, expect, it } from 'vitest'

import { AUTO_GAS_ADJUSTMENT, AUTO_GAS_FALLBACK_STEP, AUTO_GAS_MIN_USED, WRAP_GAS_LIMIT } from '@/utils/constants'
import { HYBRID_SWAP_GAS_LIMIT } from '../hybridSwapGas'
import { estimateSwapNetworkFee, swapAutoGasProbeEntries } from '../swapNetworkFee'
import {
  AUTO_GAS_SIMULATE_CAP,
  autoGasWantedFromUsed,
  fallbackGasLimitForEntries,
  feeEstimateForGasLimit,
  installSwapGasReaderForTests,
  resolveAutoGasLimit,
  resolveSwapAutoGas,
  simulateAttemptDecision,
  type SwapGasReadRequest,
} from '../swapAutoGas'
import { gasLimitForRouterExecuteSwapOperations, sendHookExecuteSwapOperationsMsg } from '../terraGas'

const TOKEN_A = 'terra1tokena00000000000000000000000000000001'
const TOKEN_B = 'terra1tokenb00000000000000000000000000000002'
const MIDDLE = 'terra1middle000000000000000000000000000000003'

function twoHopSend(offer: string, ask: string) {
  return sendHookExecuteSwapOperationsMsg([
    {
      terra_swap: {
        offer_asset_info: { token: { contract_addr: offer } },
        ask_asset_info: { token: { contract_addr: MIDDLE } },
      },
    },
    {
      terra_swap: {
        offer_asset_info: { token: { contract_addr: MIDDLE } },
        ask_asset_info: { token: { contract_addr: ask } },
      },
    },
  ])
}

afterEach(() => {
  installSwapGasReaderForTests(null)
})

describe('swap auto gas (#1360)', () => {
  it('turns the measured CL8Y → KENA gas_used into ceil × 1.2', () => {
    expect(AUTO_GAS_ADJUSTMENT).toBe(1.2)
    expect(autoGasWantedFromUsed(1_937_976)).toBe(2_325_572)
    expect(autoGasWantedFromUsed('1937775')).toBe(Math.ceil(1_937_775 * 1.2))
    expect(autoGasWantedFromUsed(1_661_441)).toBe(1_993_730)
  })

  it('treats timeout-class and hostile gas_used values as a failed simulate', () => {
    const fallback = gasLimitForRouterExecuteSwapOperations(2)
    for (const used of [null, undefined, '', '0', '-1', 0, 1, AUTO_GAS_MIN_USED - 1, 147_000_000, '147000000']) {
      const decision = resolveAutoGasLimit(used, fallback)
      expect(decision).toEqual({ gasLimit: 1_910_000, source: 'fallback' })
      expect(decision.gasLimit).not.toBe(Math.ceil(147_000_000 * 1.2))
    }
  })

  it('climbs 200k when the query budget is full, then signs 1.2× of a finished simulate', async () => {
    expect(simulateAttemptDecision(2_110_000, 2_110_000)).toBe('step')
    expect(simulateAttemptDecision(1_937_976, 2_110_000 + AUTO_GAS_FALLBACK_STEP)).toBe('success')
    expect(simulateAttemptDecision(147_000_000, 15_000_000)).toBe('stop')

    const fallback = 1_910_000
    installSwapGasReaderForTests(async (request: SwapGasReadRequest) => {
      const query = request.queryGasLimit ?? 0
      if (query === AUTO_GAS_SIMULATE_CAP) return null
      if (query === fallback + AUTO_GAS_FALLBACK_STEP) return query
      if (query === fallback + AUTO_GAS_FALLBACK_STEP * 2) return 1_937_976
      return null
    })

    const decision = await resolveSwapAutoGas(fallback, {
      signer: 'terra1sender',
      entries: [{ contract: TOKEN_A, msg: twoHopSend(TOKEN_A, TOKEN_B) }],
    })
    expect(decision).toEqual({ gasLimit: 2_325_572, source: 'simulate' })
  })

  it('accepts the cap and rejects one gas above it', () => {
    expect(AUTO_GAS_SIMULATE_CAP).toBe(HYBRID_SWAP_GAS_LIMIT)
    expect(autoGasWantedFromUsed(15_000_000)).toBe(18_000_000)
    expect(autoGasWantedFromUsed(15_000_001)).toBeNull()
    expect(autoGasWantedFromUsed(AUTO_GAS_MIN_USED)).toBe(Math.ceil(AUTO_GAS_MIN_USED * 1.2))
  })

  it('uses the same fallback and the same 1.2× result for different CW20 addresses', () => {
    const kena = fallbackGasLimitForEntries([{ msg: twoHopSend(TOKEN_A, TOKEN_B) }])
    const ust1 = fallbackGasLimitForEntries([{ msg: twoHopSend(TOKEN_B, TOKEN_A) }])
    expect(kena).toBe(ust1)
    expect(kena).toBe(1_910_000)
    expect(autoGasWantedFromUsed(1_937_976)).toBe(autoGasWantedFromUsed(1_937_976))
  })

  it('keeps wrap + one hop on 1,800,000 when simulate fails, and 1.2× when it succeeds', () => {
    const entries = [{ msg: { wrap_deposit: {} } }, { msg: sendHookExecuteSwapOperationsMsg([{ terra_swap: {} }]) }]
    const fallback = fallbackGasLimitForEntries(entries)
    expect(fallback).toBe(WRAP_GAS_LIMIT + gasLimitForRouterExecuteSwapOperations(1))
    expect(fallback).toBe(1_800_000)
    expect(resolveAutoGasLimit(null, fallback).gasLimit).toBe(1_800_000)
    expect(resolveAutoGasLimit(1_661_441, fallback)).toEqual({ gasLimit: 1_993_730, source: 'simulate' })
  })

  it('prices the signed gas in uluna', () => {
    const estimate = feeEstimateForGasLimit(2_325_572)
    expect(estimate.gasLimit).toBe(2_325_572)
    expect(estimate.feeUluna).toBeGreaterThan(0n)
    expect(estimate.gasPriceUluna).toBeGreaterThan(0)
  })

  it('copies max_spread and minimum_receive into the fee-row probe', () => {
    const entries = swapAutoGasProbeEntries({
      hints: {
        isDirectWrap: false,
        needsWrapInput: false,
        hopCount: 2,
        cw20RouterOperations: [
          {
            terra_swap: {
              offer_asset_info: { token: { contract_addr: TOKEN_A } },
              ask_asset_info: { token: { contract_addr: MIDDLE } },
            },
          },
          {
            terra_swap: {
              offer_asset_info: { token: { contract_addr: MIDDLE } },
              ask_asset_info: { token: { contract_addr: TOKEN_B } },
            },
          },
        ],
      },
      payContract: TOKEN_A,
      payAmount: '50000000000000000',
      maxSpread: '0.05',
      minimumReceive: '1',
    })
    expect(entries).not.toBeNull()
    const send = entries![0].msg.send as { msg: string }
    const inner = JSON.parse(atob(send.msg)) as {
      execute_swap_operations: { max_spread: string; minimum_receive: string }
    }
    expect(inner.execute_swap_operations.max_spread).toBe('0.05')
    expect(inner.execute_swap_operations.minimum_receive).toBe('1')
    const hinted = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: 2,
      cw20RouterOperations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: TOKEN_A } },
            ask_asset_info: { token: { contract_addr: MIDDLE } },
          },
        },
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: MIDDLE } },
            ask_asset_info: { token: { contract_addr: TOKEN_B } },
          },
        },
      ],
    })
    expect(hinted.gasLimit).toBe(1_910_000)
  })
})
