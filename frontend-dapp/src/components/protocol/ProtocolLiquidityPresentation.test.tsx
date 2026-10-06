import { act, fireEvent, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { renderWithProviders } from '@/test-utils'
import type { IndexerOverview } from '@/types'
import { getOverview } from '@/services/indexer/client'
import { ProtocolGlobalStats } from './ProtocolGlobalStats'
import { useProtocolOverviewQuery } from './useProtocolOverviewQuery'

vi.mock('./ProtocolVolumeDailyChart', () => ({ ProtocolVolumeDailyChart: () => null }))
vi.mock('@/services/indexer/client', () => ({ getOverview: vi.fn() }))

function observation(): IndexerOverview {
  return {
    total_volume_24h: '0',
    total_trades_24h: 0,
    pair_count: 2,
    token_count: 2,
    total_liquidity_usd: '250000',
    liquidity_quality: 'fresh',
    liquidity_as_of: new Date().toISOString(),
    liquidity_max_age_seconds: 20,
    liquidity_change_24h_pct: null,
    total_volume_24h_usd: '1234',
  }
}

function Harness() {
  return <ProtocolGlobalStats overviewQuery={useProtocolOverviewQuery()} />
}

async function settle() {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(1)
  })
}

describe('liquidity headline disclosure and polling', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-10-06T12:00:00Z'))
    vi.mocked(getOverview)
      .mockReset()
      .mockImplementation(async () => observation())
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('keeps the same amount element through healthy refreshes before the 20-second expiry', async () => {
    renderWithProviders(<Harness />)
    await settle()
    const card = screen.getByTestId('protocol-stat-liquidity')
    const amount = within(card).getByText('$250K')
    expect(card).not.toHaveTextContent(/Fresh|As of|2026|—24h/)
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(25_000)
    })
    expect(getOverview).toHaveBeenCalledTimes(3)
    expect(within(card).getByText('$250K')).toBe(amount)
    expect(card.querySelector('.animate-pulse')).toBeNull()
    expect(within(card).queryByTestId('protocol-stat-liquidity-24h')).not.toBeInTheDocument()
  })

  it('retains the last verified amount through expiry and a pending fetch, then recovers', async () => {
    let complete: ((value: IndexerOverview) => void) | undefined
    vi.mocked(getOverview)
      .mockResolvedValueOnce(observation())
      .mockImplementation(
        () =>
          new Promise((resolve) => {
            complete = resolve
          })
      )
    renderWithProviders(<Harness />)
    await settle()
    const card = screen.getByTestId('protocol-stat-liquidity')
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000)
    })
    expect(card).toHaveTextContent('$250K')
    expect(card.querySelector('.animate-pulse')).toBeNull()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(11_000)
    })
    expect(card).toHaveTextContent('$250K')
    expect(card).not.toHaveTextContent('$0')
    expect(screen.getByTestId('protocol-stat-volume-24h')).toHaveTextContent('$1.2K')
    await act(async () => {
      complete?.(observation())
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(card).toHaveTextContent('$250K')
  })

  it('retains only the public complete snapshot across unmount, unavailable response and request failure', async () => {
    const first = renderWithProviders(<Harness />)
    await settle()
    expect(screen.getByTestId('protocol-stat-liquidity')).toHaveTextContent('$250K')
    first.unmount()
    vi.mocked(getOverview).mockResolvedValue({
      ...observation(),
      liquidity_quality: 'partial',
      total_liquidity_usd: '218',
    })
    const partial = renderWithProviders(<Harness />)
    await settle()
    expect(screen.getByTestId('protocol-stat-liquidity')).toHaveTextContent('$250K')
    partial.unmount()
    vi.mocked(getOverview).mockRejectedValue(new Error('offline'))
    renderWithProviders(<Harness />)
    await settle()
    const card = screen.getByTestId('protocol-stat-liquidity')
    expect(card).toHaveTextContent('$250K')
    expect(card.querySelector('.animate-pulse')).toBeNull()
    expect(within(card).queryByTestId('protocol-stat-liquidity-24h')).not.toBeInTheDocument()
    const keys = Object.keys(localStorage).filter((key) => key.startsWith('cl8y:protocol-liquidity:'))
    expect(keys).toHaveLength(1)
    expect(Object.keys(JSON.parse(localStorage.getItem(keys[0])!)).sort()).toEqual(['asOf', 'quality', 'value'])
  })

  it('reveals a relative timestamp on keyboard focus and tap, and closes with Escape or outside tap', async () => {
    renderWithProviders(<Harness />)
    await settle()
    const control = screen.getByRole('button', { name: 'Liquidity details' })
    act(() => control.focus())
    expect(screen.getByRole('tooltip')).toHaveTextContent('Updated just now.')
    expect(screen.getByRole('tooltip')).not.toHaveTextContent('2026-')
    expect(control).toHaveAttribute('aria-expanded', 'true')
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
    fireEvent.click(control)
    expect(screen.getByRole('tooltip')).toHaveTextContent('Value of liquidity in factory pools.')
    fireEvent.pointerDown(document.body)
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
  })
})
