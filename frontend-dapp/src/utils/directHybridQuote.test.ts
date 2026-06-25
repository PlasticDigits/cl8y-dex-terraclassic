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

describe('quoteDirectHybridSwap (#418)', () => {
  beforeEach(() => {
    vi.spyOn(swapRoutePreflight, 'preflightSwapRouteSpread').mockResolvedValue({
      worstSpreadPercent: '0.25',
      anyHopExceedsMaxSpread: false,
    })
  })

  it('returns indexer estimate when POST /route/solve succeeds', async () => {
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
    const lcdSpy = vi.spyOn(pair, 'simulateHybridSwap')

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

    expect(result.return_amount).toBe('950000')
    expect(result.indexerQuoteKind).toBe('indexer_hybrid_lcd')
    expect(lcdSpy).not.toHaveBeenCalled()
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
