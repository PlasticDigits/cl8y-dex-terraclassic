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

  it('renders a labeled button when buttonLabel is set (GitLab #519)', async () => {
    const user = userEvent.setup()
    render(
      <CopyButton
        text="wc:topic@1?bridge=https%3A%2F%2Fexample"
        ariaLabel="Copy pairing link"
        buttonLabel="Copy pairing link"
        data-testid="pairing-copy"
      />
    )
    const button = screen.getByTestId('pairing-copy')
    expect(button).not.toHaveAttribute('role', 'menuitem')
    expect(button).toHaveTextContent('Copy pairing link')
    await user.click(button)
    expect(mockCopyToClipboard).toHaveBeenCalledWith('wc:topic@1?bridge=https%3A%2F%2Fexample')
  })

  it('renders wallet menu row when menuLabel is set (GitLab #185)', async () => {
    const user = userEvent.setup()
    render(
      <CopyButton
        text="terra1menu"
        ariaLabel="Copy wallet address"
        menuLabel="Copy address"
        data-testid="wallet-menu-copy"
      />
    )
    const item = screen.getByTestId('wallet-menu-copy')
    expect(item).toHaveAttribute('role', 'menuitem')
    expect(item).toHaveClass('wallet-menu-item')
    expect(item.className).not.toMatch(/inline-flex/)
    expect(item).toHaveTextContent('Copy address')
    await user.click(item)
    expect(mockCopyToClipboard).toHaveBeenCalledWith('terra1menu')
  })

  it('icon-only copy is not a menuitem (GitLab #183 / #671)', () => {
    render(<CopyButton text="terra1abc" ariaLabel="Copy wallet address" />)
    const button = screen.getByRole('button', { name: 'Copy wallet address' })
    expect(button).not.toHaveAttribute('role', 'menuitem')
    expect(button).toHaveClass('copy-button')
    expect(button).not.toHaveClass('wallet-menu-item')
  })
})
