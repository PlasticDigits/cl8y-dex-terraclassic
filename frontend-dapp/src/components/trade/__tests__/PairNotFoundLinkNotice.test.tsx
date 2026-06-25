import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PairNotFoundLinkNotice } from '../PairNotFoundLinkNotice'

describe('PairNotFoundLinkNotice', () => {
  it('uses btn-muted for the pair-select CTA (#415)', async () => {
    const user = userEvent.setup()
    const focusTarget = document.createElement('button')
    focusTarget.id = 'trade-pair-select'
    document.body.appendChild(focusTarget)
    const focusSpy = vi.spyOn(focusTarget, 'focus')

    render(<PairNotFoundLinkNotice unknownParam="unknown-pair" pairSelectId="trade-pair-select" />)

    const cta = screen.getByTestId('trade-pair-not-found-link-cta')
    expect(cta).toHaveClass('btn-muted')

    await user.click(cta)
    expect(focusSpy).toHaveBeenCalled()
    focusTarget.remove()
  })
})
