import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TradeRecentTradesSection } from '../TradeRecentTradesSection'
import type { IndexerPair, IndexerTrade } from '@/types'

const pair: IndexerPair = {
  pair_address: 'terra1pair',
  asset_0: { symbol: 'UST1', contract_addr: 'terra1ust1', denom: null, decimals: 6 },
  asset_1: { symbol: 'cUSTC', contract_addr: 'terra1custc', denom: null, decimals: 6 },
  lp_token: null,
  fee_bps: 30,
  is_active: true,
}

const trade: IndexerTrade = {
  id: 1,
  pair_address: 'terra1pair',
  block_height: 1,
  block_timestamp: '2026-08-18T00:00:00Z',
  tx_hash: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
  sender: 'terra1t',
  offer_asset: 'UST1',
  ask_asset: 'cUSTC',
  offer_amount: '1000000',
  return_amount: '206000000',
  offer_decimals: 6,
  ask_decimals: 6,
  price: '206',
}

const tradesQuery = {
  isLoading: false,
  isError: false,
  error: null,
  data: [trade],
  refetch: () => undefined,
}

describe('TradeRecentTradesSection (GitLab #557)', () => {
  it('passes invert so tape Price is reciprocal and amounts stay offer→ask', () => {
    render(
      <TradeRecentTradesSection
        pairRouteReady
        tradesQuery={tradesQuery}
        activePair={pair}
        formatTimeFn={() => 'now'}
        skeletonHeight="4rem"
        inverted
      />
    )
    expect(screen.getByText(/1(\.0+)? UST1/)).toBeInTheDocument()
    expect(screen.getByText(/206(\.0+)? cUSTC/)).toBeInTheDocument()
    expect(screen.getByText(/0\.00485/)).toBeInTheDocument()
  })
})
