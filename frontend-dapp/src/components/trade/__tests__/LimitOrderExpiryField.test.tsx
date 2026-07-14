import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { LimitOrderExpiryField } from '../LimitOrderExpiryField'

describe('LimitOrderExpiryField (GitLab #156 / #488)', () => {
  const noop = () => {}

  it('marks No expiry button as active when value is null', () => {
    render(<LimitOrderExpiryField value={null} onChange={noop} idPrefix="test" />)
    const noExpiryBtn = screen.getByRole('button', { name: /no expiry/i })
    expect(noExpiryBtn.getAttribute('data-active')).toBe('true')
    expect(noExpiryBtn.getAttribute('aria-pressed')).toBe('true')
  })

  it('exposes No expiry semantics via title tooltip', () => {
    render(<LimitOrderExpiryField value={null} onChange={noop} idPrefix="test" />)
    const noExpiryBtn = screen.getByRole('button', { name: /no expiry/i })
    expect(noExpiryBtn.getAttribute('title')).toMatch(/no expiry/i)
  })

  it('links Docs from the Expiry label without instructional paragraphs', () => {
    render(<LimitOrderExpiryField value={null} onChange={noop} idPrefix="test" />)
    expect(screen.getByRole('link', { name: /^Docs$/i })).toHaveAttribute(
      'href',
      expect.stringContaining('limit-orders.md')
    )
    expect(screen.queryByText(/rests indefinitely until filled or cancelled/i)).not.toBeInTheDocument()
  })

  it('does not show indefinitely hint when an expiry is set', () => {
    const futureSec = Math.floor(Date.now() / 1000) + 3600
    render(<LimitOrderExpiryField value={futureSec} onChange={noop} idPrefix="test" />)
    expect(screen.queryByText(/rests indefinitely/i)).not.toBeInTheDocument()
  })

  it('shows past-date error when expiry is in the past', () => {
    const fixedNowMs = 1_700_000_000_000
    const pastSec = Math.floor(fixedNowMs / 1000) - 60
    render(<LimitOrderExpiryField value={pastSec} onChange={noop} idPrefix="test" nowMs={() => fixedNowMs} />)
    expect(screen.getByTestId('expiry-past-error')).toBeInTheDocument()
    expect(screen.getByText(/expiry must be in the future/i)).toBeInTheDocument()
  })

  it('does not show past-date error for a future expiry', () => {
    const fixedNowMs = 1_700_000_000_000
    const futureSec = Math.floor(fixedNowMs / 1000) + 3600
    render(<LimitOrderExpiryField value={futureSec} onChange={noop} idPrefix="test" nowMs={() => fixedNowMs} />)
    expect(screen.queryByTestId('expiry-past-error')).not.toBeInTheDocument()
  })

  it('toggles to No-expiry-active when user clicks the button', () => {
    const onChange = vi.fn()
    render(<LimitOrderExpiryField value={null} onChange={onChange} idPrefix="test" />)
    const noExpiryBtn = screen.getByRole('button', { name: /no expiry/i })
    fireEvent.click(noExpiryBtn)
    expect(onChange).toHaveBeenCalledWith(null)
  })
})
