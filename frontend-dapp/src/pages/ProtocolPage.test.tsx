import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen } from '@testing-library/react'
import { renderWithProviders } from '@/test-utils'
import ProtocolPage from './ProtocolPage'
import * as indexerClient from '@/services/indexer/client'

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    FACTORY_CONTRACT_ADDRESS: 'terra1factory000000000000000000000000001',
    ROUTER_CONTRACT_ADDRESS: 'terra1router00000000000000000000000000001',
    TERRA_LCD_URL: 'http://localhost:1317',
    TERRA_RPC_URL: 'http://localhost:26657',
  }
})

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getOraclePrice: vi.fn(),
    getOracleHistory: vi.fn(),
    getHookEvents: vi.fn(),
  }
})

describe('ProtocolPage contract audit surface', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getOraclePrice).mockResolvedValue({
      ticker: 'ustc',
      price_usd: '1',
      sources: [{ source: 'test', price_usd: '1', fetched_at: '2026-01-01T00:00:00Z' }],
    })
    vi.mocked(indexerClient.getOracleHistory).mockResolvedValue({ ticker: 'ustc', prices: [] })
    vi.mocked(indexerClient.getHookEvents).mockResolvedValue([])
  })

  it('shows factory and router addresses for audit (GitLab #378)', async () => {
    renderWithProviders(<ProtocolPage />)
    expect(await screen.findByTestId('protocol-contract-addresses')).toBeInTheDocument()
    expect(screen.getByTestId('protocol-factory-address')).toHaveTextContent('terra1factory000000000000000000000000001')
    expect(screen.getByTestId('protocol-router-address')).toHaveTextContent('terra1router00000000000000000000000000001')
    expect(screen.getByText('http://localhost:1317')).toBeInTheDocument()
    expect(screen.getByText('http://localhost:26657')).toBeInTheDocument()
  })
})
