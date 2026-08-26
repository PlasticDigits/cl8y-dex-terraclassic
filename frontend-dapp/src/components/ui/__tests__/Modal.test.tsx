import { describe, expect, it, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Modal } from '@/components/ui/Modal'

describe('Modal', () => {
  it('calls onClose when Escape is pressed if dismissible', () => {
    const onClose = vi.fn()
    render(
      <Modal isOpen onClose={onClose} title="Test" dismissible>
        {null}
      </Modal>
    )
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('does not call onClose on Escape when not dismissible', () => {
    const onClose = vi.fn()
    render(
      <Modal isOpen onClose={onClose} title="Test" dismissible={false}>
        {null}
      </Modal>
    )
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(onClose).not.toHaveBeenCalled()
  })

  it('hides header close control when not dismissible', () => {
    const onClose = vi.fn()
    render(
      <Modal isOpen onClose={onClose} title="Blocking" dismissible={false}>
        {null}
      </Modal>
    )
    expect(screen.queryByTestId('modal-close')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^close$/i })).not.toBeInTheDocument()
  })

  it('shows a labeled Close control and calls onClose when clicked (GitLab #672 D1)', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    render(
      <Modal isOpen onClose={onClose} title="Test">
        {null}
      </Modal>
    )
    const close = screen.getByRole('button', { name: /^close$/i })
    expect(close).toHaveTextContent('Close')
    expect(close).toHaveClass('app-modal-close')
    await user.click(close)
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('calls onClose when the dimmed portal root or backdrop is clicked (GitLab #672 D2)', () => {
    const onClose = vi.fn()
    render(
      <Modal isOpen onClose={onClose} title="Test" rootTestId="modal-root">
        <button type="button">Inside</button>
      </Modal>
    )
    fireEvent.click(screen.getByTestId('modal-backdrop'))
    expect(onClose).toHaveBeenCalledTimes(1)
    onClose.mockClear()
    fireEvent.click(screen.getByTestId('modal-root'))
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('does not close when the panel or its children are clicked (GitLab #672 D4)', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    render(
      <Modal isOpen onClose={onClose} title="Test">
        <button type="button">Inside</button>
      </Modal>
    )
    fireEvent.click(screen.getByRole('dialog'))
    expect(onClose).not.toHaveBeenCalled()
    await user.click(screen.getByRole('button', { name: 'Inside' }))
    expect(onClose).not.toHaveBeenCalled()
  })

  it('does not close on backdrop click when not dismissible (GitLab #672 D7)', () => {
    const onClose = vi.fn()
    render(
      <Modal isOpen onClose={onClose} title="Blocking" dismissible={false} rootTestId="blocking-root">
        {null}
      </Modal>
    )
    fireEvent.click(screen.getByTestId('modal-backdrop'))
    fireEvent.click(screen.getByTestId('blocking-root'))
    expect(onClose).not.toHaveBeenCalled()
  })

  it('uses closeAriaLabel when provided', () => {
    render(
      <Modal isOpen onClose={() => undefined} title="Connect Wallet" closeAriaLabel="Close connect wallet">
        {null}
      </Modal>
    )
    expect(screen.getByRole('button', { name: 'Close connect wallet' })).toBeInTheDocument()
    expect(screen.getByTestId('modal-close')).toHaveTextContent('Close')
  })

  it('wraps children in a scrollable body so the header stays pinned (GitLab #672 D1)', () => {
    render(
      <Modal isOpen onClose={() => undefined} title="Test">
        <p>Body</p>
      </Modal>
    )
    const dialog = screen.getByRole('dialog')
    expect(dialog.querySelector('.app-modal-header')).toBeTruthy()
    expect(dialog.querySelector('.app-modal-body')).toHaveTextContent('Body')
    expect(dialog.className).toContain('app-modal-panel')
  })

  it('applies pairing z-index class above the default Connect Wallet layer (GitLab #554)', () => {
    render(
      <>
        <Modal isOpen onClose={() => undefined} title="Connect" rootTestId="wallet-connect-modal-portal">
          {null}
        </Modal>
        <Modal
          isOpen
          onClose={() => undefined}
          title="Pair"
          zIndexClassName="z-[10001]"
          rootTestId="walletconnect-pairing-portal"
        >
          {null}
        </Modal>
      </>
    )
    expect(screen.getByTestId('wallet-connect-modal-portal').className).toContain('z-[9999]')
    expect(screen.getByTestId('walletconnect-pairing-portal').className).toContain('z-[10001]')
  })
})
