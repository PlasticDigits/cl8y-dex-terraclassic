import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PairSearchSelect, type PairSearchSelectProps } from '@/components/trade/PairSearchSelect'
import * as indexerClient from '@/services/indexer/client'
import type { PairInfo } from '@/types'

const factoryPairs: PairInfo[] = [
  {
    contract_addr: 'terra1pair0000000000000000000000000000000001',
    liquidity_token: 'terra1lp',
    asset_infos: [{ token: { contract_addr: 'EMBER' } }, { token: { contract_addr: 'CORAL' } }],
  },
  {
    contract_addr: 'terra1pair0000000000000000000000000000000002',
    liquidity_token: 'terra1lp2',
    asset_infos: [{ token: { contract_addr: 'LUNC' } }, { token: { contract_addr: 'USTC' } }],
  },
]

function renderPairSearchSelect(props: Partial<PairSearchSelectProps> = {}) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <PairSearchSelect value="" onChange={() => {}} factoryPairs={factoryPairs} id="test-pair" {...props} />
    </QueryClientProvider>
  )
}

describe('PairSearchSelect degraded mode (GitLab #314)', () => {
  beforeEach(() => {
    vi.spyOn(indexerClient, 'getPairs').mockRejectedValue(new Error('indexer down'))
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('filters factory pairs locally when indexer errors and user types a symbol', async () => {
    const user = userEvent.setup()
    renderPairSearchSelect()

    const input = screen.getByRole('combobox')
    await user.click(input)
    await waitFor(() => {
      expect(screen.getAllByRole('option').length).toBeGreaterThan(0)
    })

    await user.clear(input)
    await user.type(input, 'EMBER')

    await waitFor(
      () => {
        const options = screen.getAllByRole('option')
        expect(options.length).toBeGreaterThanOrEqual(1)
        expect(options.some((o) => o.textContent?.includes('EMBER'))).toBe(true)
      },
      { timeout: 3_000 }
    )

    expect(screen.queryByText(/no pairs match your search/i)).not.toBeInTheDocument()
  })
})
