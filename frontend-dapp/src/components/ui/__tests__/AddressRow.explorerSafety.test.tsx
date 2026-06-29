import { afterEach, describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

vi.mock('@/utils/copyToClipboard', () => ({
  copyToClipboard: vi.fn().mockResolvedValue({ ok: true }),
}))

describe('AddressRow explorer link safety (#430 / SEC-E10)', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
    vi.resetModules()
  })

  it('omits explorer anchors when address fails URL validation', async () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.resetModules()
    const { AddressRow } = await import('@/components/ui/AddressRow')

    render(<AddressRow address="javascript:alert(1)" />)

    expect(screen.queryByRole('link')).not.toBeInTheDocument()
    expect(screen.queryByTestId('address-row-explorer')).not.toBeInTheDocument()
    // Builder rejects non-bech32 segments; React href sanitization is a secondary layer ([#430]).
  })
})
