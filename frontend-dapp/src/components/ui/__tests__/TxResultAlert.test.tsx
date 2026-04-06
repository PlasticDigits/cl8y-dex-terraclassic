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
})
