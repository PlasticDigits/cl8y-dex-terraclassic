import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

vi.mock('@/utils/copyToClipboard', () => ({
  copyToClipboard: vi.fn().mockResolvedValue({ ok: true }),
}))

vi.mock('@/utils/terraExplorer', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/terraExplorer')>()
  return {
    ...actual,
    getExplorerAddressUrl: vi.fn(),
  }
})

import { AddressRow } from '@/components/ui/AddressRow'
import { shortenAddress } from '@/utils/tokenDisplay'
import * as terraExplorer from '@/utils/terraExplorer'

const SAMPLE = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const EXPLORER = 'https://finder.test/address/terra1'

describe('AddressRow', () => {
  beforeEach(() => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReset()
  })

  it('renders shortened label, copy, and explorer affordances', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(EXPLORER)

    render(<AddressRow address={SAMPLE} copyAriaLabel="Copy LP token address" />)

    expect(screen.getByTestId('address-row')).toBeInTheDocument()
    expect(screen.getByText(shortenAddress(SAMPLE))).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Copy LP token address' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'View address on explorer' })).toHaveAttribute('href', EXPLORER)
    expect(screen.getByRole('link', { name: /terra16w/ })).toHaveAttribute('href', EXPLORER)
  })

  it('shows full address when showFull is true', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(null)

    render(<AddressRow address={SAMPLE} showFull explorerAriaLabel="Explorer" />)

    expect(screen.getByText(SAMPLE)).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'Explorer' })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Copy address' })).toBeInTheDocument()
  })

  it('omits explorer link when URL helper returns null', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(null)

    render(<AddressRow address={SAMPLE} startChars={6} endChars={4} />)

    expect(screen.getByText(shortenAddress(SAMPLE, 6, 4))).toBeInTheDocument()
    expect(screen.queryByTestId('address-row-explorer')).not.toBeInTheDocument()
    expect(screen.queryByRole('link')).not.toBeInTheDocument()
  })

  it('respects custom shorten lengths', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(null)

    render(<AddressRow address={SAMPLE} startChars={12} endChars={6} />)

    expect(screen.getByText(shortenAddress(SAMPLE, 12, 6))).toBeInTheDocument()
  })

  it('keeps label and icons on one row when nowrap is set (#671)', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(null)

    render(<AddressRow address={SAMPLE} nowrap data-testid="wallet-menu-address-row" />)

    const row = screen.getByTestId('wallet-menu-address-row')
    expect(row).toHaveClass('flex-nowrap')
    expect(row).not.toHaveClass('flex-wrap')
    expect(row).not.toHaveTextContent(SAMPLE)
    expect(screen.getByText(shortenAddress(SAMPLE, 8, 6))).toHaveClass('truncate')
    expect(screen.getByText(shortenAddress(SAMPLE, 8, 6))).toHaveAttribute('title', SAMPLE)
  })

  it('still shows the full string with break-all when showFull is set without nowrap', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(null)

    render(<AddressRow address={SAMPLE} showFull />)

    const label = screen.getByText(SAMPLE)
    expect(label).toHaveClass('break-all')
    expect(label).not.toHaveClass('truncate')
  })

  it('renders spoofed HTML as a text node, not markup (#671)', () => {
    vi.mocked(terraExplorer.getExplorerAddressUrl).mockReturnValue(null)
    const spoof = '<script>alert(1)</script>'

    const { container } = render(<AddressRow address={spoof} showFull />)

    expect(container.querySelector('script')).toBeNull()
    expect(screen.getByText(spoof)).toBeInTheDocument()
    expect(screen.getByTitle(spoof)).toHaveAttribute('title', spoof)
  })
})
