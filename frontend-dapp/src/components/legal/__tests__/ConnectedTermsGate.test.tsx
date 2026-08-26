import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import ConnectedTermsGate from '@/components/legal/ConnectedTermsGate'
import { useWalletStore } from '@/hooks/useWallet'

const getSignatureStatus = vi.fn()
const getTermsLatest = vi.fn()

vi.mock('@plasticdigits/cl8y-clickwrap', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@plasticdigits/cl8y-clickwrap')>()
  return {
    ...actual,
    createClient: () => ({
      apiBaseUrl: 'https://api.terms.cl8y.com',
      termsBaseUrl: 'https://terms.cl8y.com',
      getSignatureStatus,
      getTermsLatest,
      getTermsContent: vi.fn(),
      submitWallet: vi.fn(),
      submitTelegram: vi.fn(),
    }),
  }
})

vi.mock('@/utils/legalClickwrap', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/legalClickwrap')>()
  return {
    ...actual,
    skipLegalClickwrapForAutomation: () => false,
    getLegalClickwrapClient: () => ({
      apiBaseUrl: 'https://api.terms.cl8y.com',
      termsBaseUrl: 'https://terms.cl8y.com',
      getSignatureStatus,
      getTermsLatest,
      getTermsContent: vi.fn(),
      submitWallet: vi.fn(),
      submitTelegram: vi.fn(),
    }),
  }
})

const UNSIGNED_STATUS = {
  property: 'dex.cl8y.com',
  latest_version: '1.0.0',
  signed_latest: false,
  signed_version: null,
  signed_at: null,
}

const UNSIGNED_TERMS = {
  property: 'dex.cl8y.com',
  version_label: '1.0.0',
  effective_date: '2026-01-01',
  content_sha256: 'abc',
  published_at: '2026-01-01T00:00:00Z',
  sign_urls: {
    telegram: 'https://terms.cl8y.com/sign/telegram',
    evm: 'https://terms.cl8y.com/sign/evm',
    terra_classic: 'https://terms.cl8y.com/sign/terra-classic?property=dex.cl8y.com',
    solana: 'https://terms.cl8y.com/sign/solana',
  },
}

function mockUnsignedLegal() {
  getSignatureStatus.mockResolvedValue(UNSIGNED_STATUS)
  getTermsLatest.mockResolvedValue(UNSIGNED_TERMS)
}

function clearWindowInjectors() {
  const w = window as unknown as {
    keplr?: unknown
    station?: unknown
    cosmostation?: unknown
  }
  delete w.keplr
  delete w.station
  delete w.cosmostation
}

describe('ConnectedTermsGate', () => {
  beforeEach(() => {
    getSignatureStatus.mockReset()
    getTermsLatest.mockReset()
    getTermsLatest.mockResolvedValue(null)
    useWalletStore.setState({ address: null, walletType: null, isConnecting: false, error: null })
    clearWindowInjectors()
  })

  afterEach(() => {
    clearWindowInjectors()
  })

  it('renders children when wallet is disconnected (browse OK)', () => {
    render(
      <ConnectedTermsGate>
        <button type="button">Swap CTA</button>
      </ConnectedTermsGate>
    )
    expect(screen.getByRole('button', { name: /swap cta/i })).toBeVisible()
    expect(getSignatureStatus).not.toHaveBeenCalled()
    expect(screen.queryByTestId('legal-wallet-inapp-hint')).not.toBeInTheDocument()
  })

  it('fail-closed: does not render transactional children when status errors', async () => {
    useWalletStore.setState({ address: 'terra1unsignedexample', walletType: 'keplr' })
    getSignatureStatus.mockRejectedValue(new Error('Legal API down'))
    getTermsLatest.mockResolvedValue(null)

    render(
      <ConnectedTermsGate>
        <button type="button">Swap CTA</button>
      </ConnectedTermsGate>
    )

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/unable to verify terms/i)
    })
    expect(screen.queryByRole('button', { name: /swap cta/i })).not.toBeInTheDocument()
    expect(screen.queryByTestId('legal-wallet-inapp-hint')).not.toBeInTheDocument()
  })

  it('blocks unsigned wallets and queries TERRA_CLASSIC for dex.cl8y.com', async () => {
    useWalletStore.setState({ address: 'terra1unsignedexample', walletType: 'keplr' })
    mockUnsignedLegal()

    render(
      <ConnectedTermsGate>
        <button type="button">Swap CTA</button>
      </ConnectedTermsGate>
    )

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /accept terms/i })).toBeVisible()
    })
    expect(screen.queryByRole('button', { name: /swap cta/i })).not.toBeInTheDocument()
    expect(getSignatureStatus).toHaveBeenCalledWith('dex.cl8y.com', 'TERRA_CLASSIC', 'terra1unsignedexample')
  })

  it('renders children when signed_latest is true', async () => {
    useWalletStore.setState({ address: 'terra1signedexample', walletType: 'keplr' })
    getSignatureStatus.mockResolvedValue({
      property: 'dex.cl8y.com',
      latest_version: '1.0.0',
      signed_latest: true,
      signed_version: '1.0.0',
      signed_at: '2026-01-02T00:00:00Z',
    })

    render(
      <ConnectedTermsGate>
        <button type="button">Swap CTA</button>
      </ConnectedTermsGate>
    )

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /swap cta/i })).toBeVisible()
    })
    expect(screen.queryByTestId('legal-wallet-inapp-hint')).not.toBeInTheDocument()
  })

  it.each([
    ['luncdash', /Open this site in Lunc Dash to accept terms/i],
    ['galaxy', /Open this site in Galaxy Station to accept terms/i],
    ['station', /Open this site in Station to accept terms/i],
    ['cosmostation', /Open this site in Cosmostation to accept terms/i],
    ['keplr', /Open this site in Keplr to accept terms/i],
  ] as const)('shows named-wallet hint when unsigned %s has no injector (GitLab #658)', async (walletType, copy) => {
    useWalletStore.setState({ address: 'terra1unsignedexample', walletType })
    mockUnsignedLegal()

    render(
      <ConnectedTermsGate>
        <button type="button">Swap CTA</button>
      </ConnectedTermsGate>
    )

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /accept terms/i })).toBeVisible()
    })
    const hint = screen.getByTestId('legal-wallet-inapp-hint')
    expect(hint).toHaveTextContent(copy)
    expect(hint).not.toHaveTextContent(/Keplr browser/i)
    expect(hint).not.toHaveTextContent(/Leap/i)
    expect(hint.tagName).toBe('P')
    expect(hint.querySelector('a')).toBeNull()
  })

  it('falls back to the DEX wallet list when wallet type is unknown (GitLab #658)', async () => {
    useWalletStore.setState({ address: 'terra1unsignedexample', walletType: null })
    mockUnsignedLegal()

    render(
      <ConnectedTermsGate>
        <button type="button">Swap CTA</button>
      </ConnectedTermsGate>
    )

    await waitFor(() => {
      expect(screen.getByTestId('legal-wallet-inapp-hint')).toHaveTextContent(
        /Station, Keplr, Cosmostation, Lunc Dash, or Galaxy Station/i
      )
    })
    expect(screen.getByRole('button', { name: /accept terms/i })).toBeVisible()
    expect(screen.getByTestId('legal-wallet-inapp-hint')).not.toHaveTextContent(/Leap/i)
  })

  it('hides the hint when window.keplr is injected (GitLab #554)', async () => {
    ;(window as unknown as { keplr: object }).keplr = {}
    useWalletStore.setState({ address: 'terra1unsignedexample', walletType: 'keplr' })
    mockUnsignedLegal()

    render(
      <ConnectedTermsGate>
        <button type="button">Swap CTA</button>
      </ConnectedTermsGate>
    )

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /accept terms/i })).toBeVisible()
    })
    expect(screen.queryByTestId('legal-wallet-inapp-hint')).not.toBeInTheDocument()
  })

  it('hides the hint when Station station.keplr is injected without window.keplr (GitLab #658)', async () => {
    ;(window as unknown as { station: { keplr: object } }).station = { keplr: {} }
    useWalletStore.setState({ address: 'terra1unsignedexample', walletType: 'station' })
    mockUnsignedLegal()

    render(
      <ConnectedTermsGate>
        <button type="button">Swap CTA</button>
      </ConnectedTermsGate>
    )

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /accept terms/i })).toBeVisible()
    })
    expect(screen.queryByTestId('legal-wallet-inapp-hint')).not.toBeInTheDocument()
  })

  it('hides the hint when Cosmostation providers.keplr is injected (GitLab #658)', async () => {
    ;(window as unknown as { cosmostation: { providers: { keplr: object } } }).cosmostation = {
      providers: { keplr: {} },
    }
    useWalletStore.setState({ address: 'terra1unsignedexample', walletType: 'cosmostation' })
    mockUnsignedLegal()

    render(
      <ConnectedTermsGate>
        <button type="button">Swap CTA</button>
      </ConnectedTermsGate>
    )

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /accept terms/i })).toBeVisible()
    })
    expect(screen.queryByTestId('legal-wallet-inapp-hint')).not.toBeInTheDocument()
  })

  it("still shows the hint when 'station' is on window but station.keplr is missing", async () => {
    ;(window as unknown as { station: object }).station = {}
    useWalletStore.setState({ address: 'terra1unsignedexample', walletType: 'station' })
    mockUnsignedLegal()

    render(
      <ConnectedTermsGate>
        <button type="button">Swap CTA</button>
      </ConnectedTermsGate>
    )

    await waitFor(() => {
      expect(screen.getByTestId('legal-wallet-inapp-hint')).toHaveTextContent(/Station/i)
    })
    expect(screen.getByRole('button', { name: /accept terms/i })).toBeVisible()
  })
})
