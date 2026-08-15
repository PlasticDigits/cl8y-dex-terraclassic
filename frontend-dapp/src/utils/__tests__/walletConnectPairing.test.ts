import { afterEach, describe, expect, it } from 'vitest'
import {
  buildAndroidWalletIntent,
  buildIosWalletIntent,
  buildLuncDashDeepLink,
  buildWalletConnectDeepLinks,
  isAllowedWalletConnectDeepLink,
  isWalletConnectMobileClient,
  isWalletConnectPairingUri,
} from '../walletConnectPairing'

const WC_V1 = 'wc:11111111-1111-1111-1111-111111111111@1?bridge=https%3A%2F%2Fwalletconnect.luncdash.com&key=test'
const WC_V2 = 'wc:2222222222222222222222222222222222222222222222222222222222222222@2?relay-protocol=irn&symKey=test'

describe('walletConnectPairing (GitLab #519)', () => {
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('accepts WC v1 and v2 pairing URIs and rejects junk', () => {
    expect(isWalletConnectPairingUri(WC_V1)).toBe(true)
    expect(isWalletConnectPairingUri(WC_V2)).toBe(true)
    expect(isWalletConnectPairingUri('https://evil.example/wc')).toBe(false)
    expect(isWalletConnectPairingUri('wc:nocolon')).toBe(false)
    expect(isWalletConnectPairingUri('')).toBe(false)
  })

  it('detects mobile UA, iPad desktop-UA, and narrow viewport', () => {
    expect(isWalletConnectMobileClient({ userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0)' })).toBe(true)
    expect(isWalletConnectMobileClient({ userAgent: 'Mozilla/5.0 (Linux; Android 14)' })).toBe(true)
    expect(
      isWalletConnectMobileClient({
        userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)',
        platform: 'MacIntel',
        maxTouchPoints: 5,
      })
    ).toBe(true)
    expect(
      isWalletConnectMobileClient({
        userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
        matchMedia: (query) => ({ matches: query.includes('max-width: 767') }),
      })
    ).toBe(true)
    expect(
      isWalletConnectMobileClient({
        userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
        platform: 'Win32',
        maxTouchPoints: 0,
        matchMedia: () => ({ matches: false }),
      })
    ).toBe(false)
  })

  it('builds the Lunc Dash scheme used by cosmes QRCodeModal', () => {
    const href = buildLuncDashDeepLink(WC_V1)
    expect(href.startsWith('luncdash://wallet_connect?')).toBe(true)
    expect(href).toContain(encodeURIComponent(`payload=${encodeURIComponent(WC_V1)}`))
  })

  it('inserts the pairing URI into Android intent templates before #Intent', () => {
    const template = 'intent://wcV2#Intent;package=com.chainapsis.keplr;scheme=keplrwallet;end;'
    const href = buildAndroidWalletIntent(template, WC_V2)
    expect(href.startsWith('intent://wcV2?')).toBe(true)
    expect(href).toContain(encodeURIComponent(WC_V2))
    expect(href).toContain('#Intent;package=com.chainapsis.keplr')
  })

  it('appends the pairing URI to iOS templates', () => {
    expect(buildIosWalletIntent('keplrwallet://wcV2', WC_V2)).toBe(`keplrwallet://wcV2?${encodeURIComponent(WC_V2)}`)
  })

  it('allowlists wallet schemes and rejects arbitrary https', () => {
    expect(isAllowedWalletConnectDeepLink(WC_V1)).toBe(true)
    expect(isAllowedWalletConnectDeepLink(buildLuncDashDeepLink(WC_V1))).toBe(true)
    expect(isAllowedWalletConnectDeepLink('https://station.hexxagon.io/wcV2?x')).toBe(true)
    expect(isAllowedWalletConnectDeepLink('https://evil.example/wc')).toBe(false)
    expect(isAllowedWalletConnectDeepLink('javascript:alert(1)')).toBe(false)
  })

  it('emits Open Lunc Dash + Open wallet for LuncDash details', () => {
    const links = buildWalletConnectDeepLinks(
      {
        name: 'LUNC Dash',
        android: '',
        ios: '',
        isStation: true,
        isLuncDash: true,
      },
      WC_V1
    )
    expect(links.map((l) => l.id)).toEqual(['wallet', 'generic'])
    expect(links[0]?.label).toBe('Open LUNC Dash')
    expect(links[0]?.href.startsWith('luncdash://')).toBe(true)
    expect(links[1]?.href).toBe(WC_V1)
  })

  it('emits Galaxy Station iOS universal link when not Android', () => {
    const links = buildWalletConnectDeepLinks(
      {
        name: 'Galaxy Station',
        android: 'https://station.hexxagon.io/wcV2#Intent;package=io.hexxagon.station;scheme=galaxystation;end;',
        ios: 'https://station.hexxagon.io/wcV2',
        isStation: false,
        isLuncDash: false,
      },
      WC_V2,
      { userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0)' }
    )
    expect(links[0]?.href.startsWith('https://station.hexxagon.io/wcV2?')).toBe(true)
    expect(isAllowedWalletConnectDeepLink(links[0]!.href)).toBe(true)
  })
})
