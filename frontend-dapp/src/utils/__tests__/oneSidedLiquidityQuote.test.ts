import { describe, expect, it, vi } from 'vitest'

const { CLUNC } = vi.hoisted(() => ({
  CLUNC: 'terra1clunc00000000000000000000000000000000',
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>()
  return {
    ...actual,
    LUNC_C_TOKEN_ADDRESS: CLUNC,
    NATIVE_WRAPPED_PAIRS: { uluna: CLUNC, uusd: '' },
    WRAPPED_NATIVE_PAIRS: { [CLUNC]: 'uluna' },
  }
})

vi.mock('@/utils/cw20RouteSolveQuote', () => ({
  quoteCw20ViaRouteSolve: vi.fn(),
}))

vi.mock('@/services/indexer/client', () => ({
  getRouteSolve: vi.fn(),
}))

import type { PairInfo, PoolResponse } from '@/types'
import { quoteCw20ViaRouteSolve } from '@/utils/cw20RouteSolveQuote'
import { getRouteSolve } from '@/services/indexer/client'
import { ONE_SIDED_EMPTY_POOL_ERROR, ONE_SIDED_NO_ROUTE_ERROR, quoteOneSidedAdd } from '../oneSidedLiquidityQuote'
import { wrapNetForZapSolver } from '../oneSidedLiquidity'

const UST1 = 'terra1ust100000000000000000000000000000000'
const OTHER = 'terra1other0000000000000000000000000000000'
const PAIR_ADDR = 'terra1pair000000000000000000000000000000000'

const pair: PairInfo = {
  contract_addr: PAIR_ADDR,
  liquidity_token: 'terra1lp00000000000000000000000000000000000',
  asset_infos: [{ token: { contract_addr: CLUNC } }, { token: { contract_addr: UST1 } }],
}

const pool: PoolResponse = {
  total_share: '100000000000000000000',
  assets: [
    { info: { token: { contract_addr: CLUNC } }, amount: '100000000000' },
    { info: { token: { contract_addr: UST1 } }, amount: '100000000000' },
  ],
}

describe('quoteOneSidedAdd (GitLab #533 T2 T5 T7)', () => {
  it('T5 empty pool disables one-sided', async () => {
    const empty: PoolResponse = {
      ...pool,
      assets: [
        { ...pool.assets[0], amount: '0' },
        { ...pool.assets[1], amount: '0' },
      ],
    }
    const quoted = await quoteOneSidedAdd({
      tokenId: CLUNC,
      pair,
      pairLabel: 'cLUNC / UST1',
      payRaw: '1000000',
      pool: empty,
      feeBps: 30,
      discountBps: 0,
      wrapFeeBps: 200,
      slippagePercent: 5,
      maxSpreadStr: '0.05',
    })
    expect(quoted).toEqual({ status: 'unavailable', disableReason: ONE_SIDED_EMPTY_POOL_ERROR })
  })

  it('T2 wrap-fee-only: 10000 LUNC @ 200 bps nets 9800 into the solver', async () => {
    const gross = '10000000000'
    const quoted = await quoteOneSidedAdd({
      tokenId: 'uluna',
      pair,
      pairLabel: 'cLUNC / UST1',
      payRaw: gross,
      pool,
      feeBps: 30,
      discountBps: 0,
      wrapFeeBps: 200,
      slippagePercent: 5,
      maxSpreadStr: '0.05',
    })
    expect(quoted.status).toBe('ok')
    if (quoted.status !== 'ok') return
    expect(quoted.snapshot.wrapDenom).toBe('uluna')
    expect(quoted.snapshot.wrapGross).toBe(gross)
    const net = wrapNetForZapSolver(BigInt(gross), 200)
    expect(net).toBe(9_800_000_000n)
    expect(BigInt(quoted.snapshot.swapAmount) + BigInt(quoted.snapshot.provideOffer)).toBeLessThanOrEqual(net)
    expect(quoted.snapshot.swapMinReturn).not.toBe('')
  })

  it('T7 off-pair without route → No route', async () => {
    vi.mocked(quoteCw20ViaRouteSolve).mockResolvedValue(null)
    const quoted = await quoteOneSidedAdd({
      tokenId: OTHER,
      pair,
      pairLabel: 'cLUNC / UST1',
      payRaw: '1000000',
      pool,
      feeBps: 30,
      discountBps: 0,
      wrapFeeBps: null,
      slippagePercent: 5,
      maxSpreadStr: '0.05',
    })
    expect(quoted).toEqual({ status: 'unavailable', disableReason: ONE_SIDED_NO_ROUTE_ERROR })
  })

  it('T7 off-pair with mocked route-in then zap', async () => {
    vi.mocked(quoteCw20ViaRouteSolve).mockResolvedValue({
      return_amount: '900000000',
      spread_amount: '0',
      commission_amount: '0',
      indexerOperations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: OTHER } },
            ask_asset_info: { token: { contract_addr: CLUNC } },
          },
        },
      ],
    } as Awaited<ReturnType<typeof quoteCw20ViaRouteSolve>>)
    vi.mocked(getRouteSolve).mockResolvedValue({
      token_in: OTHER,
      token_out: CLUNC,
      hops: [{ ask_token: CLUNC }],
      router_operations: [{}],
    } as never)
    const quoted = await quoteOneSidedAdd({
      tokenId: OTHER,
      pair,
      pairLabel: 'cLUNC / UST1',
      payRaw: '1000000',
      pool,
      feeBps: 30,
      discountBps: 0,
      wrapFeeBps: null,
      slippagePercent: 5,
      maxSpreadStr: '0.05',
    })
    expect(quoted.status).toBe('ok')
    if (quoted.status !== 'ok') return
    expect(quoted.snapshot.routeIn?.token).toBe(OTHER)
    expect(quoted.snapshot.offerCw20).toBe(CLUNC)
    expect(quoted.snapshot.routeIn?.minReturn).toBeTruthy()
  })
})
