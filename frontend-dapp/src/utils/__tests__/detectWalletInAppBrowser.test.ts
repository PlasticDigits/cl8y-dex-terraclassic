import { describe, expect, it } from 'vitest'
import { detectWalletInAppBrowser } from '../detectWalletInAppBrowser'

describe('detectWalletInAppBrowser (GitLab #554 WC-M7)', () => {
  it('detects Keplr in-app UA', () => {
    expect(detectWalletInAppBrowser('Mozilla/5.0 Keplr Mobile')).toEqual({
      isInAppBrowser: true,
      browserName: 'Keplr',
    })
  })

  it('does not treat Android Chrome as in-app', () => {
    const ua =
      'Mozilla/5.0 (Linux; Android 16; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36'
    expect(detectWalletInAppBrowser(ua)).toEqual({ isInAppBrowser: false, browserName: null })
  })
})
