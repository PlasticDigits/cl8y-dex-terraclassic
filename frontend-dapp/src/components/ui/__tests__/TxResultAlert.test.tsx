import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TxResultAlert } from '../TxResultAlert'

vi.mock('@/utils/terraExplorer', async (importOriginal) => {
  const mod = await importOriginal<typeof import('@/utils/terraExplorer')>()
  return {
    ...mod,
    getExplorerTxUrl: vi.fn(),
  }
})

import * as terraExplorer from '@/utils/terraExplorer'

const LONG_TX = `aaaaaaaa${'b'.repeat(48)}cccccc`

describe('TxResultAlert', () => {
  beforeEach(() => {
    vi.mocked(terraExplorer.getExplorerTxUrl).mockReset()
  })

  it('shows middle-elided tx hash as explorer link when explorer URL exists', () => {
    vi.mocked(terraExplorer.getExplorerTxUrl).mockReturnValue('https://explorer.test/tx/abc')
    render(<TxResultAlert type="success" message="Done" txHash={LONG_TX} />)
    const link = screen.getByRole('link', { name: 'aaaaaaaa…cccccc' })
    expect(link).toHaveAttribute('href', 'https://explorer.test/tx/abc')
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
    expect(link).toHaveAttribute('title', LONG_TX)
  })

  it('shows middle-elided tx hash without link when explorer URL is missing', () => {
    vi.mocked(terraExplorer.getExplorerTxUrl).mockReturnValue(null)
    render(<TxResultAlert type="success" message="Done" txHash={LONG_TX} />)
    expect(screen.queryByRole('link')).not.toBeInTheDocument()
    expect(screen.getByText('aaaaaaaa…cccccc')).toBeInTheDocument()
  })

  it('does not render TX line for error type even when txHash is set', () => {
    vi.mocked(terraExplorer.getExplorerTxUrl).mockReturnValue('https://explorer.test/tx/abc')
    render(<TxResultAlert type="error" message="Bad" txHash={LONG_TX} />)
    expect(screen.queryByText(/TX:/)).not.toBeInTheDocument()
  })

  it('humanizes Max spread chain errors for retail display (GitLab #134)', () => {
    const raw =
      'Transaction failed: failed to execute message; message index: 0: Max spread assertion: actual spread (0.969) exceeds max allowed (0.01): execute wasm contract failed'
    render(<TxResultAlert type="error" message={raw} />)
    expect(screen.getByText(/Trade rejected: price impact/)).toBeInTheDocument()
    expect(screen.queryByText(/execute wasm contract failed/)).not.toBeInTheDocument()
  })

  it('humanizes extension signed fee undershoot for retail display (GitLab #371)', () => {
    const raw =
      'Wallet signed a fee far below what this dApp submitted (GitLab #127). On LocalTerra with Station: disconnect, reconnect, and approve any chain-update prompt. Run `cd frontend-dapp && npm ci` so the cosmes patch is applied, then retry. Expected at least ~50985000 uluna; wallet returned ~29 uluna. Expected gas at least ~1800000; wallet returned ~1.'
    render(<TxResultAlert type="error" message={raw} />)
    expect(screen.getByText(/Transaction fee mismatch/)).toBeInTheDocument()
    expect(screen.queryByText(/GitLab #127/)).not.toBeInTheDocument()
    expect(screen.queryByText(/npm ci/)).not.toBeInTheDocument()
  })
})
