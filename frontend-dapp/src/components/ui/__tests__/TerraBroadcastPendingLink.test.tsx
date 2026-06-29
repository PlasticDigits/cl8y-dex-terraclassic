import { render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { TerraBroadcastPendingLink } from '../TerraBroadcastPendingLink'

const SAMPLE_HASH = 'ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890'

describe('TerraBroadcastPendingLink (GitLab #305 / #330)', () => {
  it('renders TX link only during confirming with a hash', () => {
    const { rerender } = render(<TerraBroadcastPendingLink phase="signing" txHash={SAMPLE_HASH} />)
    expect(screen.queryByTestId('terra-broadcast-pending-tx')).toBeNull()

    rerender(<TerraBroadcastPendingLink phase="broadcasting" txHash={SAMPLE_HASH} />)
    expect(screen.queryByTestId('terra-broadcast-pending-tx')).toBeNull()

    rerender(<TerraBroadcastPendingLink phase="confirming" txHash={SAMPLE_HASH} />)
    const link = screen.getByTestId('terra-broadcast-pending-tx')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
    expect(link.textContent).toMatch(/ABCDEF12/)
  })

  it('renders recovery status and tx link during recovering (GitLab #359)', () => {
    render(<TerraBroadcastPendingLink phase="recovering" txHash={SAMPLE_HASH} />)
    expect(screen.getByTestId('terra-broadcast-recovery-status')).toHaveTextContent(/Broadcast status unknown/)
    expect(screen.getByTestId('terra-broadcast-pending-tx')).toBeInTheDocument()
  })

  it('hides after confirming when hash is cleared', () => {
    const { rerender } = render(<TerraBroadcastPendingLink phase="confirming" txHash={SAMPLE_HASH} />)
    expect(screen.getByTestId('terra-broadcast-pending-tx')).toBeInTheDocument()

    rerender(<TerraBroadcastPendingLink phase={null} txHash={null} />)
    expect(screen.queryByTestId('terra-broadcast-pending-tx')).toBeNull()
  })

  describe('explorer link safety (#430 / SEC-E10)', () => {
    afterEach(() => {
      vi.unstubAllEnvs()
      vi.resetModules()
    })

    it('renders span (no anchor) when tx hash fails explorer URL validation', async () => {
      vi.stubEnv('VITE_NETWORK', 'mainnet')
      vi.resetModules()
      const { TerraBroadcastPendingLink: Link } = await import('../TerraBroadcastPendingLink')

      render(<Link phase="confirming" txHash="javascript:alert(1)" />)

      expect(screen.queryByRole('link')).not.toBeInTheDocument()
      const el = screen.getByTestId('terra-broadcast-pending-tx')
      expect(el.tagName).toBe('SPAN')
      // React also blocks javascript: in href at render time; builder returns null first ([#430]).
    })
  })
})
