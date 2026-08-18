import { describe, expect, it } from 'vitest'
import {
  assertRetailZapFloors,
  buildProvideAllowanceRollbackMsgs,
  buildZapInMessages,
  buildZapOutMessages,
  RetailZapFloorError,
  slippagePercentToDecimalString,
  unwrapAmountMatchesQuote,
} from '../oneSidedLiquidityTx'
import { classifyZapToken } from '../oneSidedLiquidity'
import type { SwapOperation } from '@/services/terraclassic/router'

const PAIR = 'terra1pair0000000000000000000000000000000000'
const OFFER = 'terra1offer000000000000000000000000000000000'
const ASK = 'terra1ask00000000000000000000000000000000000'
const LP = 'terra1lp000000000000000000000000000000000000'
const ROUTE_TOKEN = 'terra1route00000000000000000000000000000000'

function inner(msg: Record<string, unknown>): Record<string, unknown> {
  const send = msg.send as { msg: string }
  return JSON.parse(atob(send.msg)) as Record<string, unknown>
}

describe('oneSidedLiquidityTx (GitLab #533 T7 / T10 / A7 / A12 / A13 / A18)', () => {
  it('slippage 5% → Decimal 0.05', () => {
    expect(slippagePercentToDecimalString(5)).toBe('0.05')
    expect(slippagePercentToDecimalString(0.5)).toBe('0.005')
    expect(slippagePercentToDecimalString(1)).toBe('0.01')
  })

  it('T10 rollback batches two decrease_allowance in one array', () => {
    const msgs = buildProvideAllowanceRollbackMsgs(OFFER, ASK, PAIR, '1000', '2000')
    expect(msgs).toHaveLength(2)
    expect(msgs[0]!.msg).toEqual({ decrease_allowance: { spender: PAIR, amount: '1000' } })
    expect(msgs[1]!.msg).toEqual({ decrease_allowance: { spender: PAIR, amount: '2000' } })
  })

  it('A18 zap-in order: wrap → swap → allowances → provide with slippage_tolerance set', () => {
    const msgs = buildZapInMessages({
      pairAddress: PAIR,
      tokenOffer: OFFER,
      tokenAsk: ASK,
      wrapDenom: 'uluna',
      wrapGross: '10000000',
      swapAmount: '4000000',
      swapMinReturn: '3900000',
      provideOffer: '5800000',
      provideAsk: '3900000',
      slippagePercent: 5,
    })
    expect(msgs.map((m) => Object.keys(m.msg)[0])).toEqual([
      'wrap_deposit',
      'send',
      'increase_allowance',
      'increase_allowance',
      'provide_liquidity',
    ])
    const provide = msgs[4]!.msg.provide_liquidity as { slippage_tolerance: string | null }
    expect(provide.slippage_tolerance).toBe('0.05')
    expect(provide.slippage_tolerance).not.toBeNull()
    const swap = inner(msgs[1]!.msg).swap as { min_return: string; hybrid: { book_input: string } }
    expect(swap.min_return).toBe('3900000')
    expect(swap.hybrid.book_input).toBe('0')
  })

  it('T7 route-in then zap; empty route is caller disable (classify needs_route)', () => {
    expect(classifyZapToken(ROUTE_TOKEN, [OFFER, ASK]).kind).toBe('needs_route')
    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: ROUTE_TOKEN } },
          ask_asset_info: { token: { contract_addr: OFFER } },
        },
      },
    ]
    const msgs = buildZapInMessages({
      pairAddress: PAIR,
      tokenOffer: OFFER,
      tokenAsk: ASK,
      routeIn: {
        token: ROUTE_TOKEN,
        amount: '1000',
        operations: ops,
        minReturn: '900',
        maxSpread: '0.05',
      },
      swapAmount: '400',
      swapMinReturn: '1',
      provideOffer: '500',
      provideAsk: '1',
      slippagePercent: 5,
    })
    expect(Object.keys(msgs[0]!.msg)[0]).toBe('send')
    expect('execute_swap_operations' in inner(msgs[0]!.msg)).toBe(true)
  })

  it('A12 missing min_return throws', () => {
    expect(() =>
      buildZapInMessages({
        pairAddress: PAIR,
        tokenOffer: OFFER,
        tokenAsk: ASK,
        swapAmount: '400',
        swapMinReturn: '0',
        provideOffer: '500',
        provideAsk: '1',
        slippagePercent: 5,
      })
    ).toThrow(RetailZapFloorError)
  })

  it('A7 unwrap send uses quoted amount not a balance query', () => {
    const quoted = '777'
    const msgs = buildZapOutMessages({
      pairAddress: PAIR,
      lpToken: LP,
      lpAmount: '1000000000000000000',
      minAssets: ['1', '1'],
      tokenAsk: ASK,
      swapAmount: '50',
      swapMinReturn: '40',
      slippagePercent: 5,
      unwrap: { cw20: OFFER, amount: quoted },
    })
    expect(unwrapAmountMatchesQuote(msgs, quoted)).toBe(true)
    const unwrapSend = msgs[msgs.length - 1]!.msg.send as { amount: string }
    expect(unwrapSend.amount).toBe(quoted)
    expect(unwrapSend.amount).not.toBe('999999999')
  })

  it('zap-out order: withdraw → swap → unwrap (A18)', () => {
    const msgs = buildZapOutMessages({
      pairAddress: PAIR,
      lpToken: LP,
      lpAmount: '1',
      minAssets: ['10', '20'],
      tokenAsk: ASK,
      swapAmount: '20',
      swapMinReturn: '1',
      slippagePercent: 5,
      unwrap: { cw20: OFFER, amount: '30' },
    })
    expect(msgs).toHaveLength(3)
    expect('withdraw_liquidity' in inner(msgs[0]!.msg)).toBe(true)
    expect('swap' in inner(msgs[1]!.msg)).toBe(true)
    expect('unwrap' in inner(msgs[2]!.msg)).toBe(true)
  })

  it('assertRetailZapFloors rejects null slippage_tolerance', () => {
    expect(() =>
      assertRetailZapFloors([
        {
          contract: PAIR,
          msg: { provide_liquidity: { assets: [], slippage_tolerance: null } },
        },
      ])
    ).toThrow(/slippage_tolerance/)
  })

  it('T-Z2 / AC1 provideAsk > swapMinReturn is rejected (never TransferFrom quoted ask)', () => {
    expect(() =>
      buildZapInMessages({
        pairAddress: PAIR,
        tokenOffer: OFFER,
        tokenAsk: ASK,
        swapAmount: '400',
        swapMinReturn: '500571',
        provideOffer: '500',
        provideAsk: '526916',
        slippagePercent: 5,
      })
    ).toThrow(/provideAsk exceeds swap min_return/)
  })

  it('T-Z2 fill 525495 covers floor-sized provideAsk; quoted 526916 would Cannot Sub', () => {
    const fill = 525495n
    const quoteAsk = 526916n
    const minReturn = '500571'
    const msgs = buildZapInMessages({
      pairAddress: PAIR,
      tokenOffer: OFFER,
      tokenAsk: ASK,
      swapAmount: '400000',
      swapMinReturn: minReturn,
      provideOffer: '580000',
      provideAsk: minReturn,
      slippagePercent: 5,
    })
    const provide = msgs[msgs.length - 1]!.msg.provide_liquidity as {
      assets: Array<{ amount: string; info: { token: { contract_addr: string } } }>
    }
    const askAmt = BigInt(provide.assets.find((a) => a.info.token.contract_addr === ASK)!.amount)
    expect(askAmt).toBeLessThanOrEqual(BigInt(minReturn))
    expect(askAmt).toBeLessThanOrEqual(fill)
    expect(quoteAsk).toBeGreaterThan(fill)
    const innerSwap = inner(msgs[0]!.msg).swap as { min_return: string; hybrid: { book_input: string } }
    expect(innerSwap.min_return).toBe(minReturn)
    expect(innerSwap.hybrid.book_input).toBe('0')
    expect(msgs.map((m) => Object.keys(m.msg)[0])).toEqual([
      'send',
      'increase_allowance',
      'increase_allowance',
      'provide_liquidity',
    ])
  })
})
