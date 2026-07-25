import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { LimitOrderLadderPanel } from '../LimitOrderLadderPanel'
import { renderWithProviders } from '@/test-utils'

const PAIR = 'terra1pair0000000000000000000000000000000001'

vi.mock('@/hooks/useLimitOrderConfig', () => ({
  useLimitOrderConfig: () => ({ data: { max_batch_rungs: 20 }, isLoading: false }),
}))

vi.mock('@/hooks/useLimitLadderPlaceGates', () => ({
  useLimitLadderPlaceGates: () => ({
    canPlace: true,
    inlineGate: { canPlaceLimit: true, userMessage: null, tone: 'none' as const },
    batchMinUluna: 0n,
    gasSavingsUlunaVsSeparate: 0n,
    escrowBalanceQuery: { data: '999999999999', isLoading: false, isError: false },
    nativeUlunaQuery: { data: '999999999999', isLoading: false, isError: false },
  }),
}))

vi.mock('@/hooks/useLimitLadderPlacementPlan', () => ({
  useLimitLadderPlacementPlan: () => ({
    data: {
      path: 'thin_ladder' as const,
      recommendedMaxSteps: 32,
      skipRisk: { score: 0, predictedPlaced: 5, predictedSkipped: 0, needsHintedBatchPath: false },
      depth: { windowOrderCount: 0, foreignOrdersBetweenRungs: 0, headToBoundaryDistance: 0, unresolvedHintCount: 0 },
      hints: [],
      probeDegraded: false,
      notes: [],
    },
    isLoading: false,
    isSuccess: true,
  }),
}))

const bestBookMock = vi.fn(() => ({
  bestBid: '1' as string | null,
  bestAsk: '1.5' as string | null,
  isLoading: false,
  isError: false,
  refetch: vi.fn(),
}))

vi.mock('@/hooks/useTradeBestBookPrices', () => ({
  useTradeBestBookPrices: () => bestBookMock(),
}))

const connectedPanelProps = {
  pairAddress: PAIR,
  walletAddress: 'terra1wallet000000000000000000000000001',
  isWalletConnected: true,
  openWalletModal: vi.fn(),
  escrowToken: 'terra1token00000000000000000000000000001',
  escrowDecimals: 6,
  token0Symbol: 'CORAL',
  token1Symbol: 'EMBER',
} as const

describe('LimitOrderLadderPanel crossing guard (GitLab #297)', () => {
  beforeEach(() => {
    bestBookMock.mockReturnValue({
      bestBid: '1',
      bestAsk: '1.5',
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    })
  })

  it('blocks bid ladder when rungs cross best ask', async () => {
    const user = userEvent.setup()
    renderWithProviders(<LimitOrderLadderPanel {...connectedPanelProps} />)

    await user.clear(screen.getByTestId('ladder-start-price'))
    await user.type(screen.getByTestId('ladder-start-price'), '2')
    await user.clear(screen.getByTestId('ladder-end-price'))
    await user.type(screen.getByTestId('ladder-end-price'), '10')

    const guard = await screen.findByTestId('ladder-crossing-guard')
    await waitFor(() => {
      expect(guard).toHaveTextContent(/5 of 5 rungs cross the market/i)
    })
    expect(screen.getByTestId('ladder-place-submit')).toBeDisabled()
  })

  it('blocks ask ladder when rungs cross best bid', async () => {
    const user = userEvent.setup()
    renderWithProviders(<LimitOrderLadderPanel {...connectedPanelProps} />)

    await user.click(screen.getByRole('radio', { name: /Sell CORAL/i }))
    await user.clear(screen.getByTestId('ladder-start-price'))
    await user.type(screen.getByTestId('ladder-start-price'), '0.1')
    await user.clear(screen.getByTestId('ladder-end-price'))
    await user.type(screen.getByTestId('ladder-end-price'), '0.5')

    const guard = await screen.findByTestId('ladder-crossing-guard')
    await waitFor(() => {
      expect(guard).toHaveTextContent(/5 of 5 rungs cross the market/i)
    })
    expect(screen.getByTestId('ladder-place-submit')).toBeDisabled()
  })

  it('blocks bid ladder when best ask is missing but reference price is below rungs (#385)', async () => {
    bestBookMock.mockReturnValue({
      bestBid: '0.8',
      bestAsk: null,
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    })

    const user = userEvent.setup()
    renderWithProviders(<LimitOrderLadderPanel {...connectedPanelProps} refToken1PerToken0={1.1} />)

    await user.clear(screen.getByTestId('ladder-start-price'))
    await user.type(screen.getByTestId('ladder-start-price'), '15')
    await user.clear(screen.getByTestId('ladder-end-price'))
    await user.type(screen.getByTestId('ladder-end-price'), '18')

    const guard = await screen.findByTestId('ladder-crossing-guard')
    await waitFor(() => {
      expect(guard).toHaveTextContent(/5 of 5 rungs cross the market/i)
    })
    expect(screen.getByTestId('ladder-place-submit')).toBeDisabled()
  })

  it('allows non-crossing bid ladder within spread', async () => {
    renderWithProviders(<LimitOrderLadderPanel {...connectedPanelProps} />)

    await waitFor(() => {
      expect(screen.queryByTestId('ladder-crossing-guard')).not.toBeInTheDocument()
    })
    expect(screen.getByTestId('ladder-place-submit')).not.toBeDisabled()
  })
})
