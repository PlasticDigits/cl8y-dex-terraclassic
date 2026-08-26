import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { shortenTraderAddress } from '@/utils/tokenDisplay'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies({ seed }: { seed: string }) {
    return <span data-testid="mock-blockies" data-seed={seed} />
  },
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

import { TraderIdentity } from './TraderIdentity'

const ADDR_A = 'terra1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0abcdx'
const ADDR_B = 'terra1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb0abcdx'
const EVIL_PNG = 'https://evil.example/p.png'

describe('TraderIdentity (GitLab #656)', () => {
  it('T-ID-1/2/3: compact chip is 4/6 with blockie and /trader/{full} href', () => {
    render(
      <MemoryRouter>
        <TraderIdentity address={ADDR_A} linkToProfile data-testid="charts-leaderboard-trader" />
      </MemoryRouter>
    )
    const chip = screen.getByTestId('charts-leaderboard-trader')
    expect(chip.tagName.toLowerCase()).toBe('a')
    expect(chip).toHaveAttribute('href', `/trader/${ADDR_A}`)
    expect(chip).toHaveAttribute('title', ADDR_A)
    expect(chip).toHaveTextContent(shortenTraderAddress(ADDR_A))
    expect(chip.textContent).not.toBe(ADDR_A)
    expect(chip.textContent).not.toMatch(/terra1aaaa/)
    expect(screen.getByTestId('trader-identity-blockie')).toBeInTheDocument()
    expect(screen.getByTestId('mock-blockies')).toHaveAttribute('data-seed', ADDR_A)
    expect(chip.querySelector('img')).toBeNull()
  })

  it('A1: 4/6 collision still uses distinct hrefs and seeds', () => {
    expect(shortenTraderAddress(ADDR_A)).toBe(shortenTraderAddress(ADDR_B))
    render(
      <MemoryRouter>
        <TraderIdentity address={ADDR_A} linkToProfile data-testid="trader-a" />
        <TraderIdentity address={ADDR_B} linkToProfile data-testid="trader-b" />
      </MemoryRouter>
    )
    expect(screen.getByTestId('trader-a')).toHaveAttribute('href', `/trader/${ADDR_A}`)
    expect(screen.getByTestId('trader-b')).toHaveAttribute('href', `/trader/${ADDR_B}`)
    const seeds = screen.getAllByTestId('mock-blockies').map((n) => n.getAttribute('data-seed'))
    expect(seeds).toEqual([ADDR_A, ADDR_B])
  })

  it('A3/A5/T-ID-6: invalid strings are not links or blockies; logo_url is ignored', () => {
    const junk = ['javascript:alert(1)', 'not-terra', 'TERRA1abcdefghijklmnopqrstuvwxyz1234567890abcd', '<img src="x">']
    for (const address of junk) {
      const { unmount } = render(
        <MemoryRouter>
          <TraderIdentity address={address} linkToProfile />
        </MemoryRouter>
      )
      expect(screen.queryByTestId('trader-identity-blockie')).not.toBeInTheDocument()
      expect(screen.queryByRole('link')).not.toBeInTheDocument()
      expect(document.body.innerHTML).not.toMatch(/javascript:/)
      unmount()
    }
    render(
      <MemoryRouter>
        <TraderIdentity address={ADDR_A} linkToProfile />
      </MemoryRouter>
    )
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
    expect(document.body.innerHTML).not.toContain(EVIL_PNG)
  })

  it('does not import or render TokenLogo', () => {
    render(
      <MemoryRouter>
        <TraderIdentity address={ADDR_A} linkToProfile />
      </MemoryRouter>
    )
    expect(document.querySelector('[class*="token-logo"]')).toBeNull()
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
  })
})
