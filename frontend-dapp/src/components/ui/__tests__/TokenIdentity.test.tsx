import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import type { AssetInfo } from '@/types'

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

import { TokenIdentity } from '@/components/ui/TokenIdentity'
import * as terraExplorer from '@/utils/terraExplorer'

const CW20 = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const EXPLORER = `https://finder.terraclassic.community/columbus-5/address/${CW20}`

function wrap(ui: ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>)
}

describe('TokenIdentity (GitLab #541)', () => {
  beforeEach(() => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReset()
  })

  it('CW20: symbol is not an anchor; explorer is a sibling with SEC-E10 href', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(EXPLORER)
    wrap(<TokenIdentity info={{ token: { contract_addr: CW20 } }} role="base" />)

    const root = screen.getByTestId('token-identity-base')
    expect(root.querySelector('a.token-display, a > span')).toBeNull()
    const symbol = root.querySelector('.token-identity, span')
    expect(root.textContent).toBeTruthy()
    expect(screen.getByRole('button', { name: /copy /i })).toBeInTheDocument()
    const explorer = screen.getByTestId('token-identity-base-explorer')
    expect(explorer).toHaveAttribute('href', EXPLORER)
    expect(explorer).toHaveAttribute('target', '_blank')
    expect(explorer).toHaveAttribute('rel', 'noopener noreferrer')
    expect(explorer.closest('[data-testid="token-identity-base"]')).toBe(root)
    expect(symbol?.tagName.toLowerCase()).not.toBe('a')
  })

  it('A6: native denom is copy-only with no explorer anchor', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue('https://finder.example/address/uluna')
    wrap(<TokenIdentity info={{ native_token: { denom: 'uluna' } }} role="quote" />)

    expect(screen.getByTestId('token-identity-quote')).toHaveAttribute('data-identity-payload', 'uluna')
    expect(screen.getByRole('button', { name: /copy /i })).toBeInTheDocument()
    expect(screen.queryByTestId('token-identity-quote-explorer')).not.toBeInTheDocument()
    expect(screen.queryByRole('link')).not.toBeInTheDocument()
    expect(terraExplorer.getExplorerAddressUrl).not.toHaveBeenCalled()
  })

  it('A1: javascript / invalid CW20 renders nothing', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(null)
    wrap(<TokenIdentity info={{ token: { contract_addr: 'javascript:alert(1)' } }} role="base" />)
    expect(screen.queryByTestId('token-identity-base')).not.toBeInTheDocument()
    expect(screen.queryByRole('link')).not.toBeInTheDocument()
  })

  it('A4: look-alike symbol does not become the copy payload', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(EXPLORER)
    const info: AssetInfo = { token: { contract_addr: CW20 } }
    wrap(<TokenIdentity info={info} role="base" />)
    expect(screen.getByTestId('token-identity-base')).toHaveAttribute('data-identity-payload', CW20)
    expect(screen.getByTestId('token-identity-base')).not.toHaveAttribute('data-identity-payload', 'UST1')
  })
})
