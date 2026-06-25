import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import { TradeOnboardingStrip } from '@/components/common/TradeOnboardingStrip'
import { TRADE_ONBOARDING_DISMISSED_KEY } from '@/utils/tradeOnboarding'

describe('TradeOnboardingStrip', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  afterEach(() => {
    window.localStorage.clear()
  })

  it('renders guidance and dismisses without blocking navigation', async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <TradeOnboardingStrip />
      </MemoryRouter>
    )

    expect(screen.getByTestId('trade-onboarding-strip')).toBeInTheDocument()
    expect(screen.getByTestId('trade-onboarding-swap-link')).toHaveAttribute('href', '/')
    expect(screen.getByText(/chart, book, and limit or market tickets/i)).toBeInTheDocument()
    expect(screen.getByText(/ladder placements across pairs/i)).toBeInTheDocument()

    await user.click(screen.getByTestId('trade-onboarding-dismiss'))
    expect(screen.queryByTestId('trade-onboarding-strip')).not.toBeInTheDocument()
    expect(window.localStorage.getItem(TRADE_ONBOARDING_DISMISSED_KEY)).toBe('1')
  })

  it('stays hidden after dismiss on remount', () => {
    window.localStorage.setItem(TRADE_ONBOARDING_DISMISSED_KEY, '1')
    render(
      <MemoryRouter>
        <TradeOnboardingStrip />
      </MemoryRouter>
    )
    expect(screen.queryByTestId('trade-onboarding-strip')).not.toBeInTheDocument()
  })
})
