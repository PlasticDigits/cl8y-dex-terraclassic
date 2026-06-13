import { render, screen, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it, vi } from 'vitest'
import ProtocolPage from '@/pages/ProtocolPage'

vi.mock('@/services/indexer/client', () => ({
  getOraclePrice: vi.fn().mockResolvedValue({ price_usd: 1, sources: [] }),
  getOracleHistory: vi.fn().mockResolvedValue({ prices: [] }),
  getHookEvents: vi.fn().mockResolvedValue({ events: [] }),
}))

function renderPage() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <ProtocolPage />
      </MemoryRouter>
    </QueryClientProvider>
  )
}

describe('ProtocolPage core contracts (GitLab #378)', () => {
  it('renders factory and router audit section on protocol page', async () => {
    renderPage()
    const section = await screen.findByTestId('protocol-core-contracts')
    expect(section).toBeInTheDocument()
    expect(within(section).getByText('Factory')).toBeInTheDocument()
    expect(within(section).getByText('Router')).toBeInTheDocument()
  })
})
