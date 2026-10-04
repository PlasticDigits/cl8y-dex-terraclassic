import { queryRouterTaxFundingVersion } from '@/services/terraclassic/routeTaxFunding'
import * as router from '@/services/terraclassic/router'
vi.mock('@/services/terraclassic/routeTaxFunding', () => ({
  queryRouterTaxFundingVersion: vi.fn().mockResolvedValue(undefined),
}))
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { quoteDirectHybridSwap, quoteDisclosureForIndexerKind } from './directHybridQuote'
import * as indexerClient from '@/services/indexer/client'
import * as pair from '@/services/terraclassic/pair'
import * as swapRoutePreflight from '@/services/terraclassic/swapRoutePreflight'

const HYBRID = {
  pool_input: '800000',
  book_input: '200000',
  max_maker_fills: 8,
  book_start_hint: null,
}

const OFFER = { token: { contract_addr: 'terra1from' } }
const ASK = { token: { contract_addr: 'terra1to' } }

describe('quoteDirectHybridSwap (#418, #471)', () => {
  beforeEach(() => {
    vi.mocked(queryRouterTaxFundingVersion).mockResolvedValue(undefined)
    vi.spyOn(swapRoutePreflight, 'preflightSwapRouteSpread').mockResolvedValue({
      worstSpreadPercent: '0.25',
      anyHopExceedsMaxSpread: false,
    })
  })

  it('reconciles indexer estimate to wallet hybrid_simulation (#471)', async () => {
    vi.spyOn(indexerClient, 'postRouteSolve').mockResolvedValue({
      estimated_amount_out: '950000',
      quote_kind: 'indexer_hybrid_lcd',
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: OFFER,
            ask_asset_info: ASK,
            hybrid: HYBRID,
          },
        },
      ],
      hops: [{ pair: 'terra1pair', offer_token: 'terra1from', ask_token: 'terra1to' }],
      token_in: 'terra1from',
      token_out: 'terra1to',
      slippage_percent: '0.5',
      spot_amount_out: '960000',
    })
    vi.spyOn(pair, 'simulateHybridSwap').mockResolvedValue({
      return_amount: '940000',
      spread_amount: '100',
      commission_amount: '3000',
    })

    const result = await quoteDirectHybridSwap({
      pairAddress: 'terra1pair',
      fromToken: 'terra1from',
      toToken: 'terra1to',
      offerAssetInfo: OFFER,
      askAssetInfo: ASK,
      simRaw: '1000000',
      hybrid: HYBRID,
      maxSpreadStr: '0.01',
    })

    expect(pair.simulateHybridSwap).toHaveBeenCalledWith('terra1pair', OFFER, '1000000', HYBRID, undefined)
    expect(result.return_amount).toBe('940000')
    expect(result.spread_amount).toBe('100')
    expect(result.commission_amount).toBe('3000')
    expect(result.indexerQuoteKind).toBe('indexer_hybrid_lcd')
    expect(result.routeSlippagePercent).toBe('2.08')
    expect(result.indexerAmountReconciled).toBe(true)
  })

  it('uses wallet amount without reconcile flag when indexer matches', async () => {
    vi.spyOn(indexerClient, 'postRouteSolve').mockResolvedValue({
      estimated_amount_out: '940000',
      quote_kind: 'indexer_hybrid_lcd',
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: OFFER,
            ask_asset_info: ASK,
            hybrid: HYBRID,
          },
        },
      ],
      hops: [{ pair: 'terra1pair', offer_token: 'terra1from', ask_token: 'terra1to' }],
      token_in: 'terra1from',
      token_out: 'terra1to',
      slippage_percent: '0.5',
      spot_amount_out: '960000',
    })
    vi.spyOn(pair, 'simulateHybridSwap').mockResolvedValue({
      return_amount: '940000',
      spread_amount: '0',
      commission_amount: '0',
    })

    const result = await quoteDirectHybridSwap({
      pairAddress: 'terra1pair',
      fromToken: 'terra1from',
      toToken: 'terra1to',
      offerAssetInfo: OFFER,
      askAssetInfo: ASK,
      simRaw: '1000000',
      hybrid: HYBRID,
      maxSpreadStr: '0.01',
    })

    expect(result.return_amount).toBe('940000')
    expect(result.indexerAmountReconciled).toBe(false)
  })

  it('falls back to LCD hybrid_simulation when indexer fails', async () => {
    vi.spyOn(indexerClient, 'postRouteSolve').mockRejectedValue(new Error('indexer down'))
    vi.spyOn(pair, 'simulateHybridSwap').mockResolvedValue({
      return_amount: '940000',
      spread_amount: '100',
      commission_amount: '3000',
    })

    const result = await quoteDirectHybridSwap({
      pairAddress: 'terra1pair',
      fromToken: 'terra1from',
      toToken: 'terra1to',
      offerAssetInfo: OFFER,
      askAssetInfo: ASK,
      simRaw: '1000000',
      hybrid: HYBRID,
      maxSpreadStr: '0.01',
    })

    expect(result.return_amount).toBe('940000')
    expect(result.indexerQuoteKind).toBeUndefined()
    expect(pair.simulateHybridSwap).toHaveBeenCalledWith('terra1pair', OFFER, '1000000', HYBRID, undefined)
  })

  it('throws when both indexer and LCD hybrid sim fail (no pool-only fallback)', async () => {
    vi.spyOn(indexerClient, 'postRouteSolve').mockRejectedValue(new Error('indexer down'))
    vi.spyOn(pair, 'simulateHybridSwap').mockRejectedValue(new Error('lcd fail'))

    await expect(
      quoteDirectHybridSwap({
        pairAddress: 'terra1pair',
        fromToken: 'terra1from',
        toToken: 'terra1to',
        offerAssetInfo: OFFER,
        askAssetInfo: ASK,
        simRaw: '1000000',
        hybrid: HYBRID,
        maxSpreadStr: '0.01',
      })
    ).rejects.toThrow('lcd fail')
  })
})

describe('quoteDisclosureForIndexerKind', () => {
  it('describes hybrid paths with retail copy', () => {
    expect(quoteDisclosureForIndexerKind('indexer_hybrid_lcd')).toMatch(/limit book \+ pool/i)
    expect(quoteDisclosureForIndexerKind(undefined)).toMatch(/limit book \+ pool/i)
    expect(quoteDisclosureForIndexerKind(undefined)).not.toMatch(/hybrid_simulation|Pattern C/i)
  })
})

describe('funded manual hybrid (#1381)', () => {
  it('uses authorized gross input, net output and versioned router operations', async () => {
    vi.mocked(queryRouterTaxFundingVersion).mockResolvedValue(1)
    vi.spyOn(indexerClient, 'postRouteSolve').mockRejectedValue(new Error('503 unavailable'))
    vi.spyOn(swapRoutePreflight, 'enrichSwapOperationsWithHopMinReturns').mockImplementation(async (ops) => ops)
    vi.spyOn(swapRoutePreflight, 'preflightSwapRouteSpread').mockResolvedValue({
      worstSpreadPercent: '0.25',
      anyHopExceedsMaxSpread: false,
      anyHopMissingSlippageFloor: false,
    })
    const sim = vi.spyOn(router, 'simulateMultiHopSwap').mockResolvedValue({ amount: '955000', taxFundingVersion: 1 })
    const result = await quoteDirectHybridSwap({
      pairAddress: 'terra1pair',
      fromToken: 'terra1from',
      toToken: 'terra1to',
      offerAssetInfo: OFFER,
      askAssetInfo: ASK,
      simRaw: '1010000',
      hybrid: { ...HYBRID, pool_input: '810000' },
      maxSpreadStr: '0.01',
      quoteTrader: { trader: 'terra1synthetic' },
    })
    expect(result).toMatchObject({ return_amount: '955000', taxFundingVersion: 1 })
    expect(result.indexerOperations?.[0].terra_swap.hybrid).toMatchObject({
      pool_input: '810000',
      book_input: '200000',
    })
    expect(sim).toHaveBeenCalledWith('1010000', expect.any(Array), expect.objectContaining({ taxFundingVersion: 1 }))
  })
})
