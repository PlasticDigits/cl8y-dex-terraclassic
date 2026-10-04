import { render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router-dom'

import { TerraClassicTxFeeHint } from '@/components/common/TerraClassicTxFeeHint'
import { SwapImpactAside } from '@/components/swap/SwapImpactAside'
import { SwapPayAcquireGuidanceBanner } from '@/components/swap/SwapPayAcquireGuidanceBanner'
import { useAutoGasFeeEstimate } from '@/hooks/useAutoGasFeeEstimate'
import {
  feeEstimateForGasLimit,
  installSwapGasReaderForTests,
  type SwapGasReadRequest,
} from '@/services/terraclassic/swapAutoGas'
import { formatTerraClassicFeeLunc } from '@/services/terraclassic/terraClassicFeeEstimate'
import {
  SWAP_ACQUIRE_COPY,
  acquireReduceHiddenForBroadcastPhase,
  type SwapPayAcquireGuidance,
} from '@/utils/swapPayAcquireGuidance'

const fallback = feeEstimateForGasLimit(1_910_000)
const probe: SwapGasReadRequest = {
  signer: 'terra1sender',
  entries: [
    {
      contract: 'terra1token0000000000000000000000000000000001',
      msg: { execute_swap_operations: { operations: [{}, {}], max_spread: '0.05' } },
    },
  ],
}

function FeeRow() {
  const estimate = useAutoGasFeeEstimate(fallback, probe)
  return (
    <>
      <TerraClassicTxFeeHint estimate={estimate} data-testid="swap-network-fee" />
      <TerraClassicTxFeeHint estimate={estimate} data-testid="trade-market-network-fee" />
    </>
  )
}

const highImpact: SwapPayAcquireGuidance = {
  kind: 'high_impact',
  message: SWAP_ACQUIRE_COPY.highImpact,
  guideHref: null,
  guideLabel: null,
  reduceToHuman: '0.05',
  suggestedVfdusdHuman: null,
}

afterEach(() => {
  installSwapGasReaderForTests(null)
})

describe('Swap and Trade market network fee (#1360)', () => {
  it('renders the 1.2× uluna amount after a resolved simulate', async () => {
    installSwapGasReaderForTests(async () => 1_937_976)
    render(<FeeRow />)
    const expected = formatTerraClassicFeeLunc(feeEstimateForGasLimit(2_325_572).feeUluna)
    await waitFor(() => {
      expect(screen.getByTestId('swap-network-fee')).toHaveTextContent(expected)
      expect(screen.getByTestId('trade-market-network-fee')).toHaveTextContent(expected)
    })
    expect(screen.getByTestId('swap-network-fee')).toHaveTextContent(/LUNC/)
    expect(screen.getByTestId('swap-network-fee').textContent).not.toMatch(/uusd/)
  })

  it('renders the per-hop uluna amount when simulate fails', async () => {
    installSwapGasReaderForTests(async () => null)
    render(<FeeRow />)
    const expected = formatTerraClassicFeeLunc(fallback.feeUluna)
    await waitFor(() => {
      expect(screen.getByTestId('swap-network-fee')).toHaveTextContent(expected)
      expect(screen.getByTestId('trade-market-network-fee')).toHaveTextContent(expected)
    })
  })
})

describe('acquire banner and impact aside (#1360)', () => {
  it('does not place a 2.63% worst-hop line beside the high-impact sentence', () => {
    render(
      <MemoryRouter>
        <SwapPayAcquireGuidanceBanner guidance={highImpact} testIdPrefix="swap" onReduce={() => undefined} />
        <SwapImpactAside routeSlippagePct={6.2} worstHopPercent="2.63" />
      </MemoryRouter>
    )
    expect(screen.getByTestId('swap-acquire-guidance')).toHaveTextContent('This size moves the pool')
    expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent('Expected slippage 6.20%')
    expect(screen.getByTestId('swap-impact-aside').textContent).not.toMatch(/2\.63/)
    expect(document.body.textContent).not.toMatch(/2\.63/)
  })

  it('hides Use amount instead while recovering or confirming, and the handler only reports the amount', () => {
    expect(acquireReduceHiddenForBroadcastPhase('recovering')).toBe(true)
    expect(acquireReduceHiddenForBroadcastPhase('confirming')).toBe(true)
    expect(acquireReduceHiddenForBroadcastPhase('signing')).toBe(false)

    const onReduce = vi.fn()
    const { rerender } = render(
      <MemoryRouter>
        <SwapPayAcquireGuidanceBanner guidance={highImpact} testIdPrefix="swap" hideReduce onReduce={onReduce} />
      </MemoryRouter>
    )
    expect(screen.queryByTestId('swap-acquire-reduce')).not.toBeInTheDocument()

    rerender(
      <MemoryRouter>
        <SwapPayAcquireGuidanceBanner guidance={highImpact} testIdPrefix="swap" onReduce={onReduce} />
      </MemoryRouter>
    )
    screen.getByTestId('swap-acquire-reduce').click()
    expect(onReduce).toHaveBeenCalledTimes(1)
    expect(onReduce).toHaveBeenCalledWith('0.05')
  })
})
