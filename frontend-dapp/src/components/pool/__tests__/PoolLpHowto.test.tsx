import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import { PoolLpHowto } from '@/components/pool/PoolLpHowto'
import { POOL_LP_HOWTO_HINT_DISMISSED_KEY, POOL_LP_HOWTO_SECTION_DISMISSED_KEY } from '@/utils/poolLpHowto'
import {
  POOL_LP_HOWTO_HINT,
  POOL_LP_HOWTO_NO_INCENTIVE,
  POOL_LP_HOWTO_SUMMARY,
  POOL_LP_HOWTO_TWO_SIDED,
} from '@/utils/poolLpHowtoCopy'

function renderHowto(hash = '') {
  return render(
    <MemoryRouter initialEntries={[`/pool${hash}`]}>
      <PoolLpHowto />
    </MemoryRouter>
  )
}

describe('PoolLpHowto (#531 / #547)', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  afterEach(() => {
    window.localStorage.clear()
  })

  it('shows hint and details control without a fixed overlay', () => {
    renderHowto()
    const root = screen.getByTestId('pool-lp-howto')
    expect(root).toBeInTheDocument()
    expect(screen.getByTestId('pool-lp-howto-hint')).toHaveTextContent(POOL_LP_HOWTO_HINT)
    expect(screen.getByTestId('pool-lp-howto-summary')).toHaveTextContent(POOL_LP_HOWTO_SUMMARY)
    expect(root.className).toMatch(/\brelative\b/)
    expect(root.className).not.toMatch(/fixed|absolute/)
  })

  it('opens details from How to and keeps Provide-adjacent copy static (A7)', () => {
    renderHowto()
    const details = screen.getByTestId('pool-lp-howto-details') as HTMLDetailsElement
    expect(details.open).toBe(false)
    return userEvent
      .setup()
      .click(screen.getByTestId('pool-lp-howto-open'))
      .then(() => {
        expect(details.open).toBe(true)
        expect(screen.getByTestId('pool-lp-howto-step-two-sided')).toHaveTextContent(POOL_LP_HOWTO_TWO_SIDED)
        expect(screen.getByTestId('pool-lp-howto-step-no-incentive')).toHaveTextContent(POOL_LP_HOWTO_NO_INCENTIVE)
        expect(details.innerHTML).not.toMatch(/dangerouslySetInnerHTML/)
      })
  })

  it('dismisses the whole how-to section (hint + details) (C3 / AC8)', async () => {
    const user = userEvent.setup()
    renderHowto()
    await user.click(screen.getByTestId('pool-lp-howto-dismiss'))
    expect(screen.queryByTestId('pool-lp-howto-hint')).not.toBeInTheDocument()
    expect(screen.queryByTestId('pool-lp-howto-details')).not.toBeInTheDocument()
    expect(screen.getByTestId('pool-lp-howto-restore')).toBeInTheDocument()
    expect(window.localStorage.getItem(POOL_LP_HOWTO_SECTION_DISMISSED_KEY)).toBe('1')
    expect(window.localStorage.getItem(POOL_LP_HOWTO_HINT_DISMISSED_KEY)).toBe('1')
  })

  it('stays hidden after dismiss on remount (C4)', () => {
    window.localStorage.setItem(POOL_LP_HOWTO_SECTION_DISMISSED_KEY, '1')
    renderHowto()
    expect(screen.queryByTestId('pool-lp-howto-details')).not.toBeInTheDocument()
    expect(screen.getByTestId('pool-lp-howto-restore')).toBeInTheDocument()
  })

  it('opens details when landed on #lp-howto even if previously dismissed (C5)', () => {
    window.localStorage.setItem(POOL_LP_HOWTO_SECTION_DISMISSED_KEY, '1')
    renderHowto('#lp-howto')
    expect(screen.getByTestId('pool-lp-howto-details')).toBeInTheDocument()
    expect((screen.getByTestId('pool-lp-howto-details') as HTMLDetailsElement).open).toBe(true)
  })

  it('links only to in-app Wrap / Trade / Limits / Create Pair', () => {
    renderHowto()
    expect(screen.getByTestId('pool-lp-howto-wrap-link')).toHaveAttribute('href', '/wrap')
    expect(screen.getByTestId('pool-lp-howto-trade-link')).toHaveAttribute('href', '/trade')
    expect(screen.getByTestId('pool-lp-howto-limits-link')).toHaveAttribute('href', '/limits')
    expect(screen.getByTestId('pool-lp-howto-create-link')).toHaveAttribute('href', '/create')
  })
})
