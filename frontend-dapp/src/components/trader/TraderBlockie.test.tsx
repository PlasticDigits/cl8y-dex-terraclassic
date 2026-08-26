import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies({ seed }: { seed: string }) {
    return <span data-testid="mock-blockies" data-seed={seed} />
  },
}))

import { TraderBlockie } from './TraderBlockie'

const ADDR = 'terra1abcdefghijklmnopqrstuvwxyz1234567890abcd'

describe('TraderBlockie (GitLab #656)', () => {
  it('passes lowercase seed to react-blockies and is decorative', () => {
    render(<TraderBlockie address={ADDR} />)
    const wrap = screen.getByTestId('trader-identity-blockie')
    expect(wrap).toHaveAttribute('aria-hidden')
    expect(wrap).toHaveAttribute('data-blockie-seed', ADDR.toLowerCase())
    expect(screen.getByTestId('mock-blockies')).toHaveAttribute('data-seed', ADDR.toLowerCase())
    expect(wrap.querySelector('img')).toBeNull()
    expect(document.body.innerHTML).not.toMatch(/https?:\/\//)
  })

  it('same address → same seed; mixed-case input is still lowercased when valid', () => {
    const { rerender } = render(<TraderBlockie address={ADDR} />)
    expect(screen.getByTestId('mock-blockies')).toHaveAttribute('data-seed', ADDR)
    rerender(<TraderBlockie address={ADDR} />)
    expect(screen.getAllByTestId('mock-blockies')).toHaveLength(1)
    expect(screen.getByTestId('mock-blockies')).toHaveAttribute('data-seed', ADDR.toLowerCase())
  })

  it('does not paint a blockie for invalid / non-terra1 strings', () => {
    const { rerender } = render(<TraderBlockie address="javascript:alert(1)" />)
    expect(screen.queryByTestId('trader-identity-blockie')).not.toBeInTheDocument()
    rerender(<TraderBlockie address="not-terra" />)
    expect(screen.queryByTestId('trader-identity-blockie')).not.toBeInTheDocument()
    rerender(<TraderBlockie address="TERRA1abcdefghijklmnopqrstuvwxyz1234567890abcd" />)
    expect(screen.queryByTestId('trader-identity-blockie')).not.toBeInTheDocument()
    rerender(<TraderBlockie address={`${ADDR}/../admin`} />)
    expect(screen.queryByTestId('trader-identity-blockie')).not.toBeInTheDocument()
  })
})
