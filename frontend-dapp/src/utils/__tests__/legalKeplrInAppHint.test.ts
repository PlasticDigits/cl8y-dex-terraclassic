import { afterEach, describe, expect, it, vi } from 'vitest'
import { WalletName } from '@goblinhunt/cosmes/wallet'
import {
  LEGAL_TERMS_DEX_WALLET_NAMES,
  LEGAL_TERMS_WALLET_HINT,
  hasLegalSignerInjector,
  legalTermsWalletHint,
  shouldShowLegalWalletInAppHint,
} from '../legalKeplrInAppHint'

describe('shouldShowLegalWalletInAppHint (GitLab #554 / #658)', () => {
  it('shows only when unsigned and no signer injector is present', () => {
    expect(shouldShowLegalWalletInAppHint({ hasSignerInjector: false, signedLatest: false })).toBe(true)
    expect(shouldShowLegalWalletInAppHint({ hasSignerInjector: true, signedLatest: false })).toBe(false)
    expect(shouldShowLegalWalletInAppHint({ hasSignerInjector: false, signedLatest: true })).toBe(false)
    expect(shouldShowLegalWalletInAppHint({ hasSignerInjector: false, signedLatest: null })).toBe(false)
    expect(shouldShowLegalWalletInAppHint({ hasSignerInjector: true, signedLatest: true })).toBe(false)
  })
})

describe('legalTermsWalletHint (GitLab #658)', () => {
  it('names the DEX Connect set when wallet type is unknown', () => {
    expect(legalTermsWalletHint(null)).toBe(LEGAL_TERMS_WALLET_HINT)
    expect(legalTermsWalletHint(undefined)).toBe(LEGAL_TERMS_WALLET_HINT)
    expect(legalTermsWalletHint('simulated')).toBe(LEGAL_TERMS_WALLET_HINT)
    for (const name of LEGAL_TERMS_DEX_WALLET_NAMES) {
      expect(LEGAL_TERMS_WALLET_HINT).toContain(name)
    }
    expect(LEGAL_TERMS_WALLET_HINT).not.toMatch(/Keplr browser/i)
    expect(LEGAL_TERMS_WALLET_HINT).not.toMatch(/Leap/i)
    expect(LEGAL_TERMS_WALLET_HINT).not.toMatch(/ADR-036|window\.keplr/i)
  })

  it('uses a one-wallet sentence for the connected DEX wallet', () => {
    expect(legalTermsWalletHint('station')).toBe('Open this site in Station to accept terms.')
    expect(legalTermsWalletHint('keplr')).toBe('Open this site in Keplr to accept terms.')
    expect(legalTermsWalletHint('cosmostation')).toBe('Open this site in Cosmostation to accept terms.')
    expect(legalTermsWalletHint('luncdash')).toBe('Open this site in Lunc Dash to accept terms.')
    expect(legalTermsWalletHint('galaxy')).toBe('Open this site in Galaxy Station to accept terms.')
    expect(legalTermsWalletHint('keplr')).not.toMatch(/Keplr browser/i)
    expect(legalTermsWalletHint('keplr')).not.toMatch(/Leap/i)
  })
})

describe('hasLegalSignerInjector (GitLab #658)', () => {
  afterEach(() => {
    const w = window as unknown as {
      keplr?: unknown
      station?: unknown
      cosmostation?: unknown
    }
    delete w.keplr
    delete w.station
    delete w.cosmostation
  })

  it('is false with no injector and when Station has no keplr shim', () => {
    expect(hasLegalSignerInjector()).toBe(false)
    ;(window as unknown as { station: object }).station = {}
    expect(hasLegalSignerInjector()).toBe(false)
  })

  it('is true for window.keplr', () => {
    ;(window as unknown as { keplr: object }).keplr = {}
    expect(hasLegalSignerInjector()).toBe(true)
  })

  it('is true for Station station.keplr without window.keplr', () => {
    ;(window as unknown as { station: { keplr: object } }).station = { keplr: {} }
    expect(hasLegalSignerInjector()).toBe(true)
  })

  it('is true for Cosmostation providers.keplr without window.keplr', () => {
    ;(window as unknown as { cosmostation: { providers: { keplr: object } } }).cosmostation = {
      providers: { keplr: {} },
    }
    expect(hasLegalSignerInjector()).toBe(true)
  })

  it('reuses getKeplrLikeExtension surfaces', async () => {
    const { getKeplrLikeExtension } = await import('@/services/terraclassic/keplrLikeExtension')
    ;(window as unknown as { keplr: { getKey: () => void } }).keplr = { getKey: vi.fn() }
    expect(getKeplrLikeExtension(WalletName.KEPLR)).toBeTruthy()
    expect(hasLegalSignerInjector()).toBe(true)
  })
})
