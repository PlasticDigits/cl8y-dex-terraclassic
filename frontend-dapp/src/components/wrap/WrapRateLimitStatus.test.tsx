import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import { renderWithProviders } from '@/test-utils'
import { WrapRateLimitStatus } from './WrapRateLimitStatus'
import * as wrapMapper from '@/services/terraclassic/wrapMapper'

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    WRAP_MAPPER_CONTRACT_ADDRESS: 'terra1wrap_mapper_mock',
  }
})

vi.mock('@/services/terraclassic/wrapMapper', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/terraclassic/wrapMapper')>()
  return {
    ...actual,
    queryRateLimit: vi.fn(),
  }
})

describe('WrapRateLimitStatus', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows available amount and reset countdown', async () => {
    const now = Math.floor(Date.now() / 1000)
    vi.mocked(wrapMapper.queryRateLimit).mockResolvedValue({
      config: { max_amount_per_window: '1000000000', window_seconds: 3600 },
      current_window_start: String(now - 60),
      amount_used: '250000000',
    })

    renderWithProviders(<WrapRateLimitStatus denom="uluna" symbol="LUNC" />)

    await waitFor(() => {
      expect(screen.getByTestId('wrap-rate-limit-status-available')).toHaveTextContent(/750/)
      expect(screen.getByTestId('wrap-rate-limit-status-available')).toHaveTextContent(/1K/)
      expect(screen.getByTestId('wrap-rate-limit-status-reset')).toHaveTextContent(/Resets in/)
    })
  })

  it('shows idle window copy when no wraps yet', async () => {
    vi.mocked(wrapMapper.queryRateLimit).mockResolvedValue({
      config: { max_amount_per_window: '1000000000', window_seconds: 86400 },
      current_window_start: null,
      amount_used: '0',
    })

    renderWithProviders(<WrapRateLimitStatus denom="uusd" symbol="USTC" testId="rl" />)

    await waitFor(() => {
      expect(screen.getByTestId('rl-reset')).toHaveTextContent(/No wraps in the current window/)
    })
  })
})
