import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import type { AssetInfo } from '@/types'
import { MAINNET_CUSTC_TOKEN_ADDRESS, MAINNET_UST1_TOKEN_ADDRESS } from '@/utils/ust1SecondaryMarket'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

vi.mock('@/utils/copyToClipboard', () => ({
  copyToClipboard: vi.fn().mockResolvedValue({ ok: true }),
}))

vi.mock('@/utils/terraExplorer', () => ({
  getExplorerAddressUrl: vi.fn(),
}))

vi.mock('@/services/indexer/client', () => ({
  getTokens: vi.fn().mockResolvedValue([]),
}))

import { PairTokenLinks } from '@/components/ui/PairTokenLinks'
import * as terraExplorer from '@/utils/terraExplorer'

const PAIR = 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'
const UST1: AssetInfo = { token: { contract_addr: MAINNET_UST1_TOKEN_ADDRESS } }
const CUSTC: AssetInfo = { token: { contract_addr: MAINNET_CUSTC_TOKEN_ADDRESS } }

function wrap(ui: ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>)
}

describe('PairTokenLinks (GitLab #541)', () => {
  beforeEach(() => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockImplementation(
      (addr: string) => `https://finder.terraclassic.community/columbus-5/address/${addr}`
    )
  })

  it('renders factory-stable base/quote/pair chips', () => {
    wrap(<PairTokenLinks pairAddress={PAIR} asset0={UST1} asset1={CUSTC} />)
    expect(screen.getByTestId('pair-token-links')).toBeInTheDocument()
    expect(screen.getByTestId('token-identity-base')).toHaveAttribute(
      'data-identity-payload',
      MAINNET_UST1_TOKEN_ADDRESS
    )
    expect(screen.getByTestId('token-identity-quote')).toHaveAttribute(
      'data-identity-payload',
      MAINNET_CUSTC_TOKEN_ADDRESS
    )
    expect(screen.getByTestId('token-identity-pair')).toBeInTheDocument()
    expect(screen.getByTestId('token-identity-pair-explorer')).toHaveAttribute(
      'href',
      `https://finder.terraclassic.community/columbus-5/address/${PAIR}`
    )
    expect(screen.getByTestId('token-identity-pair-explorer')).toHaveAttribute('rel', 'noopener noreferrer')
  })

  it('T5 / A7: invert reorders chips but does not swap payloads', () => {
    wrap(<PairTokenLinks pairAddress={PAIR} asset0={UST1} asset1={CUSTC} inverted />)
    const row = screen.getByTestId('pair-token-links')
    const chips = [...row.children].map((el) => el.getAttribute('data-testid'))
    expect(chips[0]).toBe('token-identity-quote')
    expect(chips[1]).toBe('token-identity-base')
    expect(screen.getByTestId('token-identity-base')).toHaveAttribute(
      'data-identity-payload',
      MAINNET_UST1_TOKEN_ADDRESS
    )
    expect(screen.getByTestId('token-identity-quote')).toHaveAttribute(
      'data-identity-payload',
      MAINNET_CUSTC_TOKEN_ADDRESS
    )
  })

  it('T541-6: invalid / missing pair hides the row', () => {
    const { rerender } = wrap(<PairTokenLinks pairAddress="" asset0={UST1} asset1={CUSTC} />)
    expect(screen.queryByTestId('pair-token-links')).not.toBeInTheDocument()
    rerender(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <PairTokenLinks pairAddress="javascript:alert(1)" asset0={UST1} asset1={CUSTC} />
      </QueryClientProvider>
    )
    expect(screen.queryByTestId('pair-token-links')).not.toBeInTheDocument()
    rerender(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <PairTokenLinks
          pairAddress="terra1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
          asset0={UST1}
          asset1={CUSTC}
        />
      </QueryClientProvider>
    )
    expect(screen.queryByTestId('pair-token-links')).not.toBeInTheDocument()
  })

  it('A2: explorer hrefs come from the helper, not host concatenation in the component', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(null)
    wrap(<PairTokenLinks pairAddress={PAIR} asset0={UST1} asset1={CUSTC} />)
    expect(screen.queryByTestId('token-identity-base-explorer')).not.toBeInTheDocument()
    expect(screen.queryByTestId('token-identity-pair-explorer')).not.toBeInTheDocument()
    expect(screen.queryByRole('link')).not.toBeInTheDocument()
  })
})
