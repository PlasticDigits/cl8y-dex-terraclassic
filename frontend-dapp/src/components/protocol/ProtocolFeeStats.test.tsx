import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { UseQueryResult } from '@tanstack/react-query'
import { renderWithProviders } from '@/test-utils'
import type { IndexerOverview, ProtocolFeesResponse } from '@/types'
import { ProtocolFeeStats } from './ProtocolFeeStats'

function renderFees(configured: boolean, eventCount = 1) {
  const data: ProtocolFeesResponse = {
    window: '24h',
    wrap_mapper_configured: false,
    ust1_window_configured: false,
    factory_configured: configured,
    community_invoices_configured: configured,
    by_source: ['pair_creation', 'sku_unlock', 'settings_fee', '<script>bad</script>', 'toString'].map((source) => ({
      source,
      amount_usd: null,
      share_pct: null,
      event_count: eventCount,
    })),
    by_token: [],
  }
  return renderWithProviders(
    <ProtocolFeeStats
      overviewQuery={{ data: undefined, isLoading: false, isError: false } as UseQueryResult<IndexerOverview>}
      feesQuery={{ data, isSuccess: true, isLoading: false, isError: false } as UseQueryResult<ProtocolFeesResponse>}
    />
  )
}

describe('pinned invoice fee source labels', () => {
  it('renders the fixed labels while hiding unknown source strings', () => {
    renderFees(true)
    expect(screen.getByText('Pair creation')).toBeInTheDocument()
    expect(screen.getByText('SKU unlock')).toBeInTheDocument()
    expect(screen.getByText('Settings invoice')).toBeInTheDocument()
    expect(screen.queryByText('<script>bad</script>')).not.toBeInTheDocument()
    expect(screen.queryByText('toString')).not.toBeInTheDocument()
  })
  it('retains paid historical rows when a pin is removed', () => {
    renderFees(false)
    expect(screen.getByText('Pair creation')).toBeInTheDocument()
    expect(screen.getByText('SKU unlock')).toBeInTheDocument()
    expect(screen.getByText('Settings invoice')).toBeInTheDocument()
  })
  it('omits unconfigured idle source families', () => {
    renderFees(false, 0)
    expect(screen.queryByText('Pair creation')).not.toBeInTheDocument()
    expect(screen.queryByText('SKU unlock')).not.toBeInTheDocument()
    expect(screen.queryByText('Settings invoice')).not.toBeInTheDocument()
  })
})
