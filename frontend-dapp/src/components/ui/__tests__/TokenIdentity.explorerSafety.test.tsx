import { afterEach, describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

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

vi.mock('@/services/indexer/client', () => ({
  getTokens: vi.fn().mockResolvedValue([]),
}))

describe('TokenIdentity explorer link safety (#541 / #430 / SEC-E10)', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
    vi.resetModules()
  })

  it('omits explorer anchors when indexer contract_addr fails URL validation', async () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.resetModules()
    const { TokenIdentity } = await import('@/components/ui/TokenIdentity')
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <TokenIdentity info={{ token: { contract_addr: 'javascript:alert(1)' } }} role="base" />
      </QueryClientProvider>
    )
    expect(screen.queryByRole('link')).not.toBeInTheDocument()
    expect(screen.queryByTestId('token-identity-base')).not.toBeInTheDocument()
  })
})
