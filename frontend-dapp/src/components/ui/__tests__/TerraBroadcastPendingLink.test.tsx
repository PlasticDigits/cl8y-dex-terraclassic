import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
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

  it('shows recovery copy during recovering phase (GitLab #368)', () => {
    render(<TerraBroadcastPendingLink phase="recovering" txHash={SAMPLE_HASH} />)
    expect(screen.getByTestId('terra-broadcast-recovery-status')).toHaveTextContent(/broadcast status unknown/i)
    expect(screen.getByTestId('terra-broadcast-pending-tx')).toBeInTheDocument()
  })

  it('hides after confirming when hash is cleared', () => {
    const { rerender } = render(<TerraBroadcastPendingLink phase="confirming" txHash={SAMPLE_HASH} />)
    expect(screen.getByTestId('terra-broadcast-pending-tx')).toBeInTheDocument()

    rerender(<TerraBroadcastPendingLink phase={null} txHash={null} />)
    expect(screen.queryByTestId('terra-broadcast-pending-tx')).toBeNull()
  })
})
