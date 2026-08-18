/**
 * Retail one-sided add/withdraw execute messages (GitLab #533 / Z533; #559 / Z559).
 *
 * Frontend orchestration on existing pair/router/wrap messages — no new Zap execute (Z533-10).
 *
 * **Add order (A18):** optional `wrap_deposit` → optional router route-in → pair pool-only
 * swap (`min_return`) → `increase_allowance` ×2 → `provide_liquidity` (`slippage_tolerance` set).
 *
 * **Z559-1:** `provideAsk` must be ≤ swap `min_return` so a fill in `(min_return, quote)`
 * cannot CW20-underflow TransferFrom. Quotes may be optimistic; execution follows floors.
 *
 * **Withdraw order (A18):** LP `send` + `withdraw_liquidity` (`min_assets`) → pair pool-only
 * swap of the other side (`min_return`) → optional mapper `unwrap` of the **floor-sized**
 * amount only (Z533-8 / Z559-3 / A7 — never `getTokenBalance`).
 */

import type { TerraExecuteContractEntry } from '@/services/terraclassic/terraBroadcast'
import { poolOnlyHybridParams } from '@/services/terraclassic/poolOnlyHybrid'
import type { SwapOperation } from '@/services/terraclassic/router'
import { slippagePercentToBps } from '@/utils/rawAmountMath'
import { ROUTER_CONTRACT_ADDRESS, TREASURY_CONTRACT_ADDRESS, WRAP_MAPPER_CONTRACT_ADDRESS } from '@/utils/constants'

export class RetailZapFloorError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'RetailZapFloorError'
  }
}

/** CosmWasm `Decimal` string for `max_spread` / `slippage_tolerance` (5 → `"0.05"`). */
export function slippagePercentToDecimalString(percent: number): string {
  const bps = slippagePercentToBps(percent)
  if (bps <= 0) return '0'
  if (bps >= 10_000) return '1'
  const whole = Math.floor(bps / 10_000)
  const frac = (bps % 10_000).toString().padStart(4, '0').replace(/0+$/, '')
  return frac.length === 0 ? String(whole) : `${whole}.${frac}`
}

function encodeHook(msg: Record<string, unknown>): string {
  return btoa(JSON.stringify(msg))
}

export type ZapInTxInput = {
  pairAddress: string
  tokenOffer: string
  tokenAsk: string
  wrapDenom?: string | null
  wrapGross?: string | null
  routeIn?: {
    token: string
    amount: string
    operations: SwapOperation[]
    minReturn: string
    maxSpread: string
  } | null
  swapAmount: string
  swapMinReturn: string
  provideOffer: string
  provideAsk: string
  slippagePercent: number
}

export type ZapOutTxInput = {
  pairAddress: string
  lpToken: string
  lpAmount: string
  minAssets: [string, string]
  tokenAsk: string
  swapAmount: string
  swapMinReturn: string
  slippagePercent: number
  unwrap?: { cw20: string; amount: string } | null
}

function pairSwapSend(token: string, pairAddress: string, amount: string, minReturn: string, maxSpread: string) {
  if (!minReturn || minReturn === '0') {
    throw new RetailZapFloorError('Zap swap min_return is required')
  }
  return {
    contract: token,
    msg: {
      send: {
        contract: pairAddress,
        amount,
        msg: encodeHook({
          swap: {
            max_spread: maxSpread,
            min_return: minReturn,
            hybrid: poolOnlyHybridParams(amount),
          },
        }),
      },
    },
  } satisfies TerraExecuteContractEntry
}

/**
 * Two `decrease_allowance` messages in one multi-msg (T10 / A10 / #147).
 */
export function buildProvideAllowanceRollbackMsgs(
  tokenA: string,
  tokenB: string,
  pairAddress: string,
  amountA: string,
  amountB: string
): TerraExecuteContractEntry[] {
  return [
    { contract: tokenA, msg: { decrease_allowance: { spender: pairAddress, amount: amountA } } },
    { contract: tokenB, msg: { decrease_allowance: { spender: pairAddress, amount: amountB } } },
  ]
}

export function buildZapInMessages(input: ZapInTxInput): TerraExecuteContractEntry[] {
  const maxSpread = slippagePercentToDecimalString(input.slippagePercent)
  const slippage = maxSpread
  if (!slippage || slippage === '0') {
    throw new RetailZapFloorError('Provide slippage_tolerance is required')
  }
  if (!input.swapMinReturn || input.swapMinReturn === '0') {
    throw new RetailZapFloorError('Zap swap min_return is required')
  }
  if (BigInt(input.provideAsk) > BigInt(input.swapMinReturn)) {
    throw new RetailZapFloorError('provideAsk exceeds swap min_return')
  }

  const msgs: TerraExecuteContractEntry[] = []

  if (input.wrapDenom && input.wrapGross) {
    msgs.push({
      contract: TREASURY_CONTRACT_ADDRESS,
      msg: { wrap_deposit: {} },
      coins: [{ denom: input.wrapDenom, amount: input.wrapGross }],
    })
  }

  if (input.routeIn) {
    if (!input.routeIn.minReturn || input.routeIn.minReturn === '0') {
      throw new RetailZapFloorError('Route-in min_return is required')
    }
    msgs.push({
      contract: input.routeIn.token,
      msg: {
        send: {
          contract: ROUTER_CONTRACT_ADDRESS,
          amount: input.routeIn.amount,
          msg: encodeHook({
            execute_swap_operations: {
              operations: input.routeIn.operations.map((op) => ({ terra_swap: op.terra_swap })),
              max_spread: input.routeIn.maxSpread,
              minimum_receive: input.routeIn.minReturn,
            },
          }),
        },
      },
    })
  }

  msgs.push(pairSwapSend(input.tokenOffer, input.pairAddress, input.swapAmount, input.swapMinReturn, maxSpread))

  msgs.push({
    contract: input.tokenOffer,
    msg: {
      increase_allowance: {
        spender: input.pairAddress,
        amount: input.provideOffer,
        expires: { never: {} },
      },
    },
  })
  msgs.push({
    contract: input.tokenAsk,
    msg: {
      increase_allowance: {
        spender: input.pairAddress,
        amount: input.provideAsk,
        expires: { never: {} },
      },
    },
  })
  msgs.push({
    contract: input.pairAddress,
    msg: {
      provide_liquidity: {
        assets: [
          { info: { token: { contract_addr: input.tokenOffer } }, amount: input.provideOffer },
          { info: { token: { contract_addr: input.tokenAsk } }, amount: input.provideAsk },
        ],
        slippage_tolerance: slippage,
        receiver: null,
        deadline: null,
      },
    },
  })

  assertRetailZapFloors(msgs)
  return msgs
}

export function buildZapOutMessages(input: ZapOutTxInput): TerraExecuteContractEntry[] {
  const maxSpread = slippagePercentToDecimalString(input.slippagePercent)
  if (!input.minAssets[0] || !input.minAssets[1]) {
    throw new RetailZapFloorError('Withdraw min_assets is required')
  }
  if (!input.swapMinReturn || input.swapMinReturn === '0') {
    throw new RetailZapFloorError('Zap-out swap min_return is required')
  }

  const msgs: TerraExecuteContractEntry[] = [
    {
      contract: input.lpToken,
      msg: {
        send: {
          contract: input.pairAddress,
          amount: input.lpAmount,
          msg: encodeHook({
            withdraw_liquidity: { min_assets: input.minAssets },
          }),
        },
      },
    },
  ]

  if (BigInt(input.swapAmount) > 0n) {
    msgs.push(pairSwapSend(input.tokenAsk, input.pairAddress, input.swapAmount, input.swapMinReturn, maxSpread))
  }

  if (input.unwrap) {
    if (!input.unwrap.amount || input.unwrap.amount === '0') {
      throw new RetailZapFloorError('Unwrap amount is required')
    }
    msgs.push({
      contract: input.unwrap.cw20,
      msg: {
        send: {
          contract: WRAP_MAPPER_CONTRACT_ADDRESS,
          amount: input.unwrap.amount,
          msg: encodeHook({ unwrap: { recipient: null } }),
        },
      },
    })
  }

  assertRetailZapFloors(msgs)
  return msgs
}

function innerSendMsg(entry: TerraExecuteContractEntry): Record<string, unknown> | null {
  const send = entry.msg.send as { msg?: string } | undefined
  if (!send?.msg) return null
  try {
    return JSON.parse(atob(send.msg)) as Record<string, unknown>
  } catch {
    return null
  }
}

/**
 * Missing floors are a test failure (A12). Native-wrap provide must not omit slippage_tolerance.
 */
export function assertRetailZapFloors(msgs: TerraExecuteContractEntry[]): void {
  let sawProvide = false
  let sawSwap = false
  for (const entry of msgs) {
    if ('provide_liquidity' in entry.msg) {
      sawProvide = true
      const pl = entry.msg.provide_liquidity as { slippage_tolerance?: string | null }
      if (pl.slippage_tolerance == null || pl.slippage_tolerance === '') {
        throw new RetailZapFloorError('provide_liquidity slippage_tolerance must not be null')
      }
    }
    const inner = innerSendMsg(entry)
    if (!inner) continue
    if ('swap' in inner) {
      sawSwap = true
      const swap = inner.swap as { min_return?: string | null; hybrid?: { book_input?: string } }
      if (!swap.min_return || swap.min_return === '0') {
        throw new RetailZapFloorError('swap min_return is required')
      }
      if (swap.hybrid && swap.hybrid.book_input != null && swap.hybrid.book_input !== '0') {
        throw new RetailZapFloorError('zap swap must be pool-only (A13)')
      }
    }
    if ('withdraw_liquidity' in inner) {
      const w = inner.withdraw_liquidity as { min_assets?: unknown }
      if (!w.min_assets) {
        throw new RetailZapFloorError('withdraw_liquidity min_assets is required')
      }
    }
    if ('unwrap' in inner) {
      const send = entry.msg.send as { amount?: string }
      if (!send.amount || send.amount === '0') {
        throw new RetailZapFloorError('unwrap amount must be the floor-sized zap-out, not wallet balance')
      }
    }
  }
  void sawProvide
  void sawSwap
}

/** True when unwrap send amount equals the quote (A7 regression). */
export function unwrapAmountMatchesQuote(msgs: TerraExecuteContractEntry[], quotedUnwrap: string): boolean {
  for (const entry of msgs) {
    const inner = innerSendMsg(entry)
    if (inner && 'unwrap' in inner) {
      const send = entry.msg.send as { amount?: string }
      return send.amount === quotedUnwrap
    }
  }
  return quotedUnwrap === '0' || quotedUnwrap === ''
}
