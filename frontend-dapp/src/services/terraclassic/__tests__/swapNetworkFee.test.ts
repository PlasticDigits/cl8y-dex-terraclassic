import { describe, expect, it } from 'vitest'
import {
  MIN_GAS_PRICE_ULUNA,
  UNWRAP_GAS_LIMIT,
  UNWRAP_ROUTER_COMBO_OVERHEAD_GAS,
  WRAP_GAS_LIMIT,
  WRAP_ROUTER_COMBO_OVERHEAD_GAS,
} from '@/utils/constants'
import { estimateSwapNetworkFee } from '../swapNetworkFee'
import { estimateNativeSwapUlunaFeesTotal, nativeSwapFeeExecuteMsgs } from '../transactions'
import { buildTerraClassicFee, gasLimitForRouterExecuteSwapOperations, totalGasLimitForExecuteMsgs } from '../terraGas'
import { estimateTerraClassicFeeForEntries } from '../terraClassicFeeEstimate'

describe('estimateSwapNetworkFee (GitLab #587)', () => {
  it('fee denom is uluna for wrap+2hop', () => {
    const hints = { isDirectWrap: false, needsWrapInput: true, hopCount: 2 }
    const est = estimateSwapNetworkFee(hints)
    const fee = buildTerraClassicFee(est.gasLimit)
    expect(fee.amount[0]?.denom).toBe('uluna')
    expect(est.gasPriceUluna).toBeGreaterThanOrEqual(MIN_GAS_PRICE_ULUNA)
    expect(est.feeUluna).toBe(estimateNativeSwapUlunaFeesTotal(hints))
  })

  it('wrap+1hop is WRAP + router 1-hop (no combo overhead)', () => {
    const est = estimateSwapNetworkFee({ isDirectWrap: false, needsWrapInput: true, hopCount: 1 })
    expect(est.gasLimit).toBe(WRAP_GAS_LIMIT + gasLimitForRouterExecuteSwapOperations(1))
  })

  it('wrap+2hop includes combo overhead and stays tens-of-LUNC class', () => {
    const est = estimateSwapNetworkFee({ isDirectWrap: false, needsWrapInput: true, hopCount: 2 })
    expect(est.gasLimit).toBe(
      WRAP_GAS_LIMIT + gasLimitForRouterExecuteSwapOperations(2) + WRAP_ROUTER_COMBO_OVERHEAD_GAS
    )
    expect(est.gasLimit).toBeGreaterThan(2_310_000)
    expect(Number(est.feeUluna) / 1_000_000).toBeLessThan(100)
  })

  it('wrap+3hop and wrap+4hop scale with hop floor', () => {
    const h2 = estimateSwapNetworkFee({ isDirectWrap: false, needsWrapInput: true, hopCount: 2 })
    const h3 = estimateSwapNetworkFee({ isDirectWrap: false, needsWrapInput: true, hopCount: 3 })
    const h4 = estimateSwapNetworkFee({ isDirectWrap: false, needsWrapInput: true, hopCount: 4 })
    expect(h3.gasLimit).toBeGreaterThan(h2.gasLimit)
    expect(h4.gasLimit).toBeGreaterThan(h3.gasLimit)
  })

  it('unwrap_output 2-hop matches nativeSwapFeeExecuteMsgs and includes unwrap combo (#599)', () => {
    const hints = { isDirectWrap: false, needsWrapInput: false, needsUnwrapOutput: true, hopCount: 2 }
    const est = estimateSwapNetworkFee(hints)
    expect(est.gasLimit).toBe(totalGasLimitForExecuteMsgs(nativeSwapFeeExecuteMsgs(hints)))
    expect(est.gasLimit).toBe(
      gasLimitForRouterExecuteSwapOperations(2) + UNWRAP_GAS_LIMIT + UNWRAP_ROUTER_COMBO_OVERHEAD_GAS
    )
    expect(Number(est.feeUluna) / 1_000_000).toBeLessThan(100)
    expect(est.feeUluna).toBe(
      estimateTerraClassicFeeForEntries([{ contract: '', msg: nativeSwapFeeExecuteMsgs(hints)[0].msg }]).feeUluna
    )
  })

  it('CW20 direct pair uses pool-only swap envelope', () => {
    const est = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: 1,
      cw20DirectPair: true,
    })
    expect(est.gasLimit).toBe(840_000)
  })

  it('does not jump to 15M hybrid on wrap+2hop', () => {
    const est = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: true,
      hopCount: 2,
      cw20Hybrid: true,
    })
    expect(est.gasLimit).toBeLessThan(15_000_000)
  })

  it('CW20 4-hop mixed ops match broadcast send envelope (#679)', () => {
    const hop1 = {
      pool_input: '0',
      book_input: '10000000000',
      max_maker_fills: 8,
      book_start_hint: 1426,
    }
    const ops = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: 'terra1a' } },
          ask_asset_info: { token: { contract_addr: 'terra1b' } },
          hybrid: hop1,
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
    const est = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: 4,
      cw20RouterOperations: ops,
    })
    expect(est.gasLimit).toBe(6_785_500)
    expect(est.gasLimit).not.toBe(3_810_000)
    expect(Number(est.feeUluna) / 1_000_000).toBeLessThan(400)
  })
})
