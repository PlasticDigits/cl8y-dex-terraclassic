import { describe, expect, it, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
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
    expect(screen.queryByRole('button', { name: /close modal/i })).not.toBeInTheDocument()
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
