import { describe, it, expect, vi, beforeEach } from 'vitest'
import { fireEvent, screen, waitFor } from '@testing-library/react'
import { LimitOrderMyPlacementsPanel } from '@/components/trade/LimitOrderMyPlacementsPanel'
import { renderWithProviders } from '@/test-utils'
import type { IndexerLimitPlacement, PairInfo } from '@/types'

const claimExpiredLimitOrder = vi.fn()
const claimExpiredLimitOrders = vi.fn()

vi.mock('@/services/terraclassic/pair', () => ({
  claimExpiredLimitOrder: (...args: unknown[]) => claimExpiredLimitOrder(...args),
  claimExpiredLimitOrders: (...args: unknown[]) => claimExpiredLimitOrders(...args),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playSuccess: vi.fn(), playError: vi.fn() },
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

function parkedRow(orderId: number): IndexerLimitPlacement {
  return {
    id: orderId + 100,
    pair_address: PAIR.contract_addr,
    block_height: 2,
    block_timestamp: '2026-01-02T00:00:00Z',
    tx_hash: 'DEF',
    order_id: orderId,
    owner: 'terra1wallet',
    side: 'bid',
    price: '1.0',
    lifecycle_status: 'parked_expired',
    remaining_escrow: '1000000',
  }
}

describe('LimitOrderMyPlacementsPanel', () => {
  beforeEach(() => {
    claimExpiredLimitOrder.mockReset()
    claimExpiredLimitOrders.mockReset()
    claimExpiredLimitOrder.mockResolvedValue('TX1')
    claimExpiredLimitOrders.mockResolvedValue('TXBATCH')
    vi.stubGlobal(
      'confirm',
      vi.fn(() => true)
    )
  })

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

  it('renders Claim all parked only when at least two parked rows exist (GitLab #253)', () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="page"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[parkedRow(1)]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
      />
    )

    expect(screen.queryByTestId('limits-page-claim-all-parked')).not.toBeInTheDocument()
  })

  it('shows Claim all parked and confirms batch claim for multiple rows', async () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="page"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[parkedRow(10), parkedRow(11)]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
      />
    )

    const claimAll = screen.getByTestId('limits-page-claim-all-parked')
    expect(claimAll).toHaveTextContent('Claim all parked (2)')

    fireEvent.click(claimAll)

    // Match the stable confirm prefix only; the gas-estimate suffix (GitLab #259) is asserted in
    // limitExpiredClaimBatch.test.ts. Keeps the panel test decoupled from volatile gas-copy numbers.
    expect(window.confirm).toHaveBeenCalledWith(
      expect.stringContaining('Claim all 2 expired refund(s) in one transaction?')
    )
    await waitFor(() => {
      expect(claimExpiredLimitOrders).toHaveBeenCalledWith('terra1wallet', PAIR.contract_addr, [10, 11])
    })
  })

  it('disables Claim all parked while pair is paused', () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="compact"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[parkedRow(1), parkedRow(2)]}
        isLoading={false}
        isWalletConnected
        isPairPaused
        openWalletModal={vi.fn()}
      />
    )

    expect(screen.getByTestId('trade-claim-all-parked')).toBeDisabled()
    expect(screen.getByTestId('trade-claim-all-parked')).toHaveTextContent('Unavailable (pair paused)')
  })

  it('disables Claim all parked when wallet disconnected', () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="page"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress=""
        rows={[parkedRow(1), parkedRow(2)]}
        isLoading={false}
        isWalletConnected={false}
        isPairPaused={false}
        openWalletModal={vi.fn()}
      />
    )

    expect(screen.getByTestId('limits-page-claim-all-parked')).toBeDisabled()
  })
})
