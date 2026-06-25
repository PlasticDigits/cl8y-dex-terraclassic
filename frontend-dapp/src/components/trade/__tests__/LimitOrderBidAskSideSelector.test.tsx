import { describe, expect, it, vi } from 'vitest'
import { useState } from 'react'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { LimitOrderBidAskSideSelector } from '@/components/trade/LimitOrderBidAskSideSelector'

function ControlledHarness() {
  const [side, setSide] = useState<'bid' | 'ask'>('bid')
  return (
    <LimitOrderBidAskSideSelector
      idPrefix="test"
      side={side}
      onSideChange={setSide}
      bidLabel="Bid (AAA)"
      askLabel="Ask (BBB)"
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
        bidLabel="Bid (AAA)"
        askLabel="Ask (BBB)"
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
        bidLabel="Bid (AAA)"
        askLabel="Ask (BBB)"
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
        bidLabel="Bid (AAA)"
        askLabel="Ask (BBB)"
      />
    )

    expect(screen.getByTestId('test-side-ask')).toHaveClass('tab-glass-active')
    expect(screen.getByTestId('test-side-bid')).toHaveClass('tab-glass-inactive')
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
})
