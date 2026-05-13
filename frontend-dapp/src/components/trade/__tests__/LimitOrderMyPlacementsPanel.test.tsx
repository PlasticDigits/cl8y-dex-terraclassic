import { describe, it, expect, vi } from 'vitest'
import { screen } from '@testing-library/react'
import { LimitOrderMyPlacementsPanel } from '@/components/trade/LimitOrderMyPlacementsPanel'
import { renderWithProviders } from '@/test-utils'
import type { IndexerLimitPlacement, PairInfo } from '@/types'

vi.mock('@/services/terraclassic/pair', () => ({
  claimExpiredLimitOrder: vi.fn(),
}))

const PAIR: PairInfo = {
  contract_addr: 'terra1pair00000000000000000000000000000001',
  liquidity_token: 'terra1lp000000000000000000000000000000001',
  asset_infos: [
    { token: { contract_addr: 'terra1aaa0000000000000000000000000000001' } },
    { token: { contract_addr: 'terra1bbb0000000000000000000000000000002' } },
  ],
}

const ACTIVE_ROW: IndexerLimitPlacement = {
  id: 1,
  pair_address: PAIR.contract_addr,
  block_height: 1,
  block_timestamp: '2026-01-01T00:00:00Z',
  tx_hash: 'ABC',
  order_id: 42,
  owner: 'terra1wallet',
  side: 'bid',
  price: '1.5',
  lifecycle_status: 'active',
}

describe('LimitOrderMyPlacementsPanel', () => {
  it('emphasizes the active row when highlightOrderId matches (GitLab #161)', () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="compact"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[ACTIVE_ROW]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
        highlightOrderId={42}
      />
    )

    const li = screen.getByTestId('trade-placement-active-42')
    expect(li.className).toMatch(/shadow-\[0_0_0_2px/)
  })
})
