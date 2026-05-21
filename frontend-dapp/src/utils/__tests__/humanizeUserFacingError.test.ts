import { describe, expect, it } from 'vitest'
import {
  sanitizeOpaqueErrorMessage,
  tryHumanizeFetchLikeMessage,
  tryHumanizeWalletLikeMessage,
} from '../humanizeOffChainError'
import {
  getErrorMessage,
  humanizeUserFacingError,
  humanizeUserFacingErrorFromUnknown,
} from '../humanizeUserFacingError'

describe('tryHumanizeWalletLikeMessage', () => {
  it('maps rejection copy', () => {
    expect(tryHumanizeWalletLikeMessage('Failed to enable Keplr (User rejected the request)')).toMatch(/declined/)
  })

  it('maps missing extension', () => {
    expect(tryHumanizeWalletLikeMessage('Failed to connect Keplr: Keplr extension is not installed')).toMatch(
      /not found/
    )
  })

  it('maps WalletConnect partial success', () => {
    expect(tryHumanizeWalletLikeMessage('WalletConnect succeeded but session did not propagate')).toMatch(
      /WalletConnect finished/
    )
  })
})

describe('tryHumanizeFetchLikeMessage', () => {
  it('maps dynamic import chunk failures before generic fetch', () => {
    expect(
      tryHumanizeFetchLikeMessage(
        'TypeError: Failed to fetch dynamically imported module: http://localhost:3000/src/pages/SwapPage.tsx'
      )
    ).toMatch(/could not load/)
  })

  it('maps Failed to fetch', () => {
    expect(tryHumanizeFetchLikeMessage('TypeError: Failed to fetch')).toMatch(/Network request failed/)
  })

  it('maps NetworkError', () => {
    expect(tryHumanizeFetchLikeMessage('NetworkError when attempting to fetch resource.')).toMatch(/Network request/)
  })

  it('maps AbortError', () => {
    expect(tryHumanizeFetchLikeMessage('AbortError: The user aborted a request.')).toMatch(/cancelled/)
  })

  it('maps indexer errors excluding logical 404', () => {
    expect(tryHumanizeFetchLikeMessage('Indexer API error: 503 upstream')).toMatch(/Market data service/)
    expect(tryHumanizeFetchLikeMessage('Indexer API error: 404 no pair')).toMatch(/not found/)
  })
})

describe('sanitizeOpaqueErrorMessage', () => {
  it('drops stack-looking tails', () => {
    const raw = 'render crashed\n    at Component.foo (bundle.js:1:2)'
    expect(sanitizeOpaqueErrorMessage(raw)).toBe('render crashed')
  })

  it('truncates very long single-line noise', () => {
    const long = 'x'.repeat(300)
    expect(sanitizeOpaqueErrorMessage(long).length).toBeLessThanOrEqual(240)
  })
})

describe('humanizeUserFacingError', () => {
  it('still applies Terra tx classifiers', () => {
    expect(humanizeUserFacingError('Transaction failed: Max spread assertion')).toMatch(/slippage/)
  })

  it('chains wallet then fetch', () => {
    expect(humanizeUserFacingError('signing rejected by user')).toMatch(/declined/)
  })

  it('getErrorMessage coerces unknown', () => {
    expect(getErrorMessage({ message: 'x' })).toBe('[object Object]')
    expect(getErrorMessage(new Error(''))).toBe('Unknown error')
  })

  it('humanizeUserFacingErrorFromUnknown handles non-Error throws', () => {
    expect(humanizeUserFacingErrorFromUnknown('Failed to fetch')).toMatch(/Network request/)
  })
})
