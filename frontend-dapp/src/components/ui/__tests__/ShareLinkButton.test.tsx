import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import type { ComponentProps } from 'react'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

vi.mock('@/utils/copyToClipboard', () => ({
  copyToClipboard: vi.fn(),
}))

import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ShareLinkButton } from '@/components/ui/ShareLinkButton'
import { COPY_BUTTON_FAILURE_MESSAGE } from '@/utils/copyButtonCopy'
import { copyToClipboard } from '@/utils/copyToClipboard'
import { SHARE_LINK_BUTTON_LABEL, SHARE_LINK_COPIED_MESSAGE, SHARE_LINK_TITLE } from '@/utils/sharePageLinkCopy'
import { sounds } from '@/lib/sounds'

const mockCopyToClipboard = vi.mocked(copyToClipboard)
const playButtonPress = vi.mocked(sounds.playButtonPress)

const CANONICAL = 'https://dex.example.test/trader/terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'

function renderShare(overrides: Partial<ComponentProps<typeof ShareLinkButton>> = {}) {
  return render(
    <ShareLinkButton
      url={CANONICAL}
      title={SHARE_LINK_TITLE}
      text={`${SHARE_LINK_TITLE} terra1x46…k38v`}
      ariaLabel="Share trader profile link"
      {...overrides}
    />
  )
}

describe('ShareLinkButton', () => {
  beforeEach(() => {
    mockCopyToClipboard.mockReset()
    mockCopyToClipboard.mockResolvedValue({ ok: true })
    playButtonPress.mockClear()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('exposes type=button, aria-label, visible Share, and share-link-button testid', () => {
    renderShare()
    const button = screen.getByTestId('share-link-button')
    expect(button).toHaveAttribute('type', 'button')
    expect(button).toHaveAttribute('aria-label', 'Share trader profile link')
    expect(button).toHaveTextContent(SHARE_LINK_BUTTON_LABEL)
    expect(button.tagName).toBe('BUTTON')
  })

  it('calls navigator.share and does not copy when share resolves', async () => {
    const share = vi.fn().mockResolvedValue(undefined)
    const user = userEvent.setup()
    renderShare({ share, 'data-testid': 'trader-share-link' })
    await user.click(screen.getByTestId('trader-share-link'))
    expect(playButtonPress).toHaveBeenCalled()
    expect(share).toHaveBeenCalledWith({
      url: CANONICAL,
      title: SHARE_LINK_TITLE,
      text: `${SHARE_LINK_TITLE} terra1x46…k38v`,
    })
    expect(mockCopyToClipboard).not.toHaveBeenCalled()
    expect(screen.queryByText(SHARE_LINK_COPIED_MESSAGE)).not.toBeInTheDocument()
  })

  it('does not announce an error when share is aborted', async () => {
    const share = vi.fn().mockRejectedValue(new DOMException('canceled', 'AbortError'))
    const user = userEvent.setup()
    renderShare({ share })
    await user.click(screen.getByRole('button', { name: 'Share trader profile link' }))
    expect(mockCopyToClipboard).not.toHaveBeenCalled()
    expect(screen.queryByText(COPY_BUTTON_FAILURE_MESSAGE)).not.toBeInTheDocument()
    expect(screen.queryByText(SHARE_LINK_COPIED_MESSAGE)).not.toBeInTheDocument()
  })

  it('falls back to clipboard and announces Link copied when share throws', async () => {
    const share = vi.fn().mockRejectedValue(new TypeError('share failed'))
    const user = userEvent.setup()
    renderShare({ share })
    await user.click(screen.getByRole('button', { name: 'Share trader profile link' }))
    expect(mockCopyToClipboard).toHaveBeenCalledWith(CANONICAL)
    await waitFor(() => {
      expect(screen.getByText(SHARE_LINK_COPIED_MESSAGE)).toBeInTheDocument()
    })
    expect(screen.getByText(SHARE_LINK_COPIED_MESSAGE).closest('[aria-live]')).toHaveAttribute('aria-live', 'polite')
  })

  it('copies when share is omitted', async () => {
    const user = userEvent.setup()
    renderShare()
    await user.click(screen.getByRole('button', { name: 'Share trader profile link' }))
    expect(mockCopyToClipboard).toHaveBeenCalledWith(CANONICAL)
    await waitFor(() => {
      expect(screen.getByText(SHARE_LINK_COPIED_MESSAGE)).toBeInTheDocument()
    })
  })

  it('announces permission-safe failure when clipboard denies', async () => {
    mockCopyToClipboard.mockResolvedValueOnce({
      ok: false,
      message: COPY_BUTTON_FAILURE_MESSAGE,
    })
    const user = userEvent.setup()
    renderShare()
    await user.click(screen.getByRole('button', { name: 'Share trader profile link' }))
    await waitFor(() => {
      expect(screen.getByText(COPY_BUTTON_FAILURE_MESSAGE)).toBeInTheDocument()
    })
    expect(screen.queryByText(/DOMException/i)).not.toBeInTheDocument()
  })

  it('Swap override renders buttonContent without dropping type=button or copy', async () => {
    const user = userEvent.setup()
    renderShare({
      buttonContent: (
        <span>
          Share <span data-testid="swap-share-logo-slot">pair</span>
        </span>
      ),
      'data-testid': 'swap-share-link',
    })
    const button = screen.getByTestId('swap-share-link')
    expect(button).toHaveAttribute('type', 'button')
    expect(button).toHaveAttribute('aria-label', 'Share trader profile link')
    expect(screen.getByTestId('swap-share-logo-slot')).toHaveTextContent('pair')
    await user.click(button)
    expect(mockCopyToClipboard).toHaveBeenCalledWith(CANONICAL)
  })
})
