import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

vi.mock('@/utils/copyToClipboard', () => ({
  copyToClipboard: vi.fn(),
}))

import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CopyButton } from '@/components/ui/CopyButton'
import { COPY_BUTTON_FAILURE_MESSAGE, COPY_BUTTON_SUCCESS_MESSAGE } from '@/utils/copyButtonCopy'
import { copyToClipboard } from '@/utils/copyToClipboard'

const mockCopyToClipboard = vi.mocked(copyToClipboard)

describe('CopyButton', () => {
  beforeEach(() => {
    mockCopyToClipboard.mockReset()
    mockCopyToClipboard.mockResolvedValue({ ok: true })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('copies text and announces success via aria-live', async () => {
    const user = userEvent.setup()
    render(<CopyButton text="terra1abc" ariaLabel="Copy wallet address" />)

    await user.click(screen.getByRole('button', { name: 'Copy wallet address' }))

    expect(mockCopyToClipboard).toHaveBeenCalledWith('terra1abc')
    await waitFor(() => {
      expect(screen.getByText(COPY_BUTTON_SUCCESS_MESSAGE)).toBeInTheDocument()
    })
  })

  it('announces failure when clipboard helper returns an error', async () => {
    mockCopyToClipboard.mockResolvedValueOnce({
      ok: false,
      message: COPY_BUTTON_FAILURE_MESSAGE,
    })
    const user = userEvent.setup()
    render(<CopyButton text="terra1fail" ariaLabel="Copy address" />)

    await user.click(screen.getByRole('button', { name: 'Copy address' }))

    await waitFor(() => {
      expect(screen.getByText(COPY_BUTTON_FAILURE_MESSAGE)).toBeInTheDocument()
    })
  })

  it('exposes aria-label from props', () => {
    render(<CopyButton text="hash" ariaLabel="Copy transaction hash" data-testid="copy-tx" />)
    expect(screen.getByTestId('copy-tx')).toHaveAttribute('aria-label', 'Copy transaction hash')
  })
})
