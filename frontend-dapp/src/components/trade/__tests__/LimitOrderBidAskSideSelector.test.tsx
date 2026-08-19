import { describe, expect, it, vi } from 'vitest'
import { useState } from 'react'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { LimitOrderBidAskSideSelector } from '@/components/trade/LimitOrderBidAskSideSelector'
import { limitSideControlClass } from '@/components/trade/limitSideControlClass'

function ControlledHarness({ compact }: { compact?: boolean }) {
  const [side, setSide] = useState<'bid' | 'ask'>('bid')
  return (
    <LimitOrderBidAskSideSelector
      idPrefix="test"
      compact={compact}
      side={side}
      onSideChange={setSide}
      bidLabel="Buy AAA"
      askLabel="Sell AAA"
    />
  )
}

describe('LimitOrderBidAskSideSelector', () => {
  it('exposes radiogroup semantics and test ids', () => {
    render(
      <LimitOrderBidAskSideSelector
        idPrefix="pfx"
        side="bid"
        onSideChange={() => {}}
        bidLabel="Buy AAA"
        askLabel="Sell AAA"
      />
    )

    expect(screen.getByTestId('pfx-side-radiogroup')).toHaveAttribute('role', 'radiogroup')
    expect(screen.getByTestId('pfx-side-bid')).toHaveAttribute('aria-checked', 'true')
    expect(screen.getByTestId('pfx-side-ask')).toHaveAttribute('aria-checked', 'false')
  })

  it('switches side immediately on click (GitLab #153)', async () => {
    const user = userEvent.setup()
    const onSideChange = vi.fn()
    const { rerender } = render(
      <LimitOrderBidAskSideSelector
        idPrefix="test"
        side="bid"
        onSideChange={onSideChange}
        bidLabel="Buy AAA"
        askLabel="Sell AAA"
      />
    )

    await user.click(screen.getByTestId('test-side-ask'))
    expect(onSideChange).toHaveBeenCalledTimes(1)
    expect(onSideChange).toHaveBeenCalledWith('ask')

    rerender(
      <LimitOrderBidAskSideSelector
        idPrefix="test"
        side="ask"
        onSideChange={onSideChange}
        bidLabel="Buy AAA"
        askLabel="Sell AAA"
      />
    )

    expect(screen.getByTestId('test-side-ask')).toHaveClass('side-sell-selected')
    expect(screen.getByTestId('test-side-bid')).toHaveClass('side-buy-idle')
  })

  it('maps bid=Buy green and ask=Sell red regardless of label copy (GitLab #563 A1)', () => {
    render(
      <LimitOrderBidAskSideSelector
        idPrefix="test"
        side="bid"
        onSideChange={() => {}}
        bidLabel="Sell TOKEN0"
        askLabel="Buy TOKEN0"
      />
    )

    const bid = screen.getByTestId('test-side-bid')
    const ask = screen.getByTestId('test-side-ask')
    expect(bid).toHaveClass('side-buy-selected')
    expect(bid).not.toHaveClass('tab-glass-active')
    expect(ask).toHaveClass('side-sell-idle')
    expect(ask).not.toHaveClass('tab-glass-active')
    expect(ask).not.toHaveClass('alert-error')
    expect(onSideStillBid(bid)).toBe(true)
  })

  it('applies compact density classes without dropping semantic colors (GitLab #563)', () => {
    render(
      <LimitOrderBidAskSideSelector
        idPrefix="test"
        compact
        side="ask"
        onSideChange={() => {}}
        bidLabel="Buy AAA"
        askLabel="Sell AAA"
      />
    )
    expect(screen.getByTestId('test-side-ask')).toHaveClass('side-control-compact', 'side-sell-selected')
    expect(screen.getByTestId('test-side-bid')).toHaveClass('side-control-compact', 'side-buy-idle')
  })

  it('moves selection and focus with arrow keys (roving tabindex)', async () => {
    const user = userEvent.setup()
    render(<ControlledHarness />)

    const bid = screen.getByTestId('test-side-bid')
    const ask = screen.getByTestId('test-side-ask')

    bid.focus()
    expect(bid).toHaveFocus()

    await user.keyboard('{ArrowRight}')
    expect(ask).toHaveAttribute('aria-checked', 'true')
    expect(ask).toHaveFocus()

    await user.keyboard('{ArrowLeft}')
    expect(bid).toHaveAttribute('aria-checked', 'true')
    expect(bid).toHaveFocus()
  })

  it('Home/End move side and focus (GitLab #153 / #563 T7)', async () => {
    const user = userEvent.setup()
    render(<ControlledHarness />)

    const bid = screen.getByTestId('test-side-bid')
    const ask = screen.getByTestId('test-side-ask')

    bid.focus()
    await user.keyboard('{End}')
    expect(ask).toHaveAttribute('aria-checked', 'true')
    expect(ask).toHaveFocus()

    await user.keyboard('{Home}')
    expect(bid).toHaveAttribute('aria-checked', 'true')
    expect(bid).toHaveFocus()
  })

  it('limitSideControlClass never uses tab-glass-active', () => {
    expect(limitSideControlClass({ tone: 'buy', selected: true })).toBe('side-control side-buy-selected')
    expect(limitSideControlClass({ tone: 'sell', selected: false, compact: true })).toBe(
      'side-control side-control-compact side-sell-idle'
    )
    expect(limitSideControlClass({ tone: 'buy', selected: true })).not.toMatch(/tab-glass/)
  })
})

function onSideStillBid(bid: HTMLElement): boolean {
  return bid.getAttribute('aria-checked') === 'true'
}
