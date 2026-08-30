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

  it('maps Station wrong network, not missing extension (GitLab #207)', () => {
    const raw = 'Failed to connect Station: The requested chain is not available on Station.'
    expect(tryHumanizeWalletLikeMessage(raw)).toMatch(/wrong network/i)
    expect(tryHumanizeWalletLikeMessage(raw)).not.toMatch(/not found/)
    expect(humanizeUserFacingError(raw)).toMatch(/LocalTerra|localterra/)
  })

  it('maps WalletConnect partial success', () => {
    expect(tryHumanizeWalletLikeMessage('WalletConnect succeeded but session did not propagate')).toMatch(
      /WalletConnect finished/
    )
  })

  it('maps Station false popup-closed denial (GitLab #208)', () => {
    const raw = 'WalletError: User denied, extension popup was closed.'
    expect(tryHumanizeWalletLikeMessage(raw)).toMatch(/Station closed the signing popup/)
    expect(tryHumanizeWalletLikeMessage(raw)).not.toMatch(/declined in the extension/)
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

  it('maps Firefox and Safari production chunk strings before generic fetch (GitLab #706)', () => {
    expect(
      tryHumanizeFetchLikeMessage(
        'error loading dynamically imported module: https://dex.cl8y.com/assets/PoolPage-BrgV5Tp1.js'
      )
    ).toMatch(/could not load/)
    expect(tryHumanizeFetchLikeMessage('Importing a module script failed.')).toMatch(/could not load/)
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

  it('maps indexer 429 to calm retry guidance without raw status (SEC-E04 / GitLab #426)', () => {
    const out = tryHumanizeFetchLikeMessage('Indexer API error: 429 Too Many Requests')
    expect(out).toMatch(/wait a moment and try again/i)
    expect(out).not.toMatch(/\b429\b/)
    expect(out).not.toMatch(/Indexer API error/i)
    expect(out).not.toMatch(/https?:\/\//i)
  })

  it('maps LCD status code 429 to calm retry guidance (SEC-E04 / GitLab #426)', () => {
    const out = tryHumanizeFetchLikeMessage('Query failed: status code 429')
    expect(out).toMatch(/wait a moment and try again/i)
    expect(out).not.toMatch(/429|status code/i)
  })

  it('maps market quote failure shapes (#414)', () => {
    expect(tryHumanizeFetchLikeMessage('indexer unavailable')).toMatch(/temporarily unavailable/i)
    expect(tryHumanizeFetchLikeMessage('Hybrid quote unavailable')).toMatch(/Could not estimate output/i)
    expect(tryHumanizeFetchLikeMessage('lcd fail')).toMatch(/reach the chain/i)
    expect(tryHumanizeFetchLikeMessage('Unexpected token < in JSON at position 0')).toMatch(/invalid response/i)
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

  it('replaces raw undefined property reads (GitLab #327)', () => {
    expect(sanitizeOpaqueErrorMessage("Cannot read properties of undefined (reading 'length')")).toMatch(
      /something went wrong while loading/i
    )
  })
})

describe('humanizeUserFacingError', () => {
  it('still applies Terra tx classifiers', () => {
    expect(humanizeUserFacingError('Transaction failed: Max spread assertion')).toMatch(/slippage/)
  })

  it('humanizes nested Transaction failed + full wasm Max spread log (GitLab #134)', () => {
    const raw =
      'Transaction failed: failed to execute message; message index: 0: dispatch: submessages: Max spread assertion: actual spread (0.969) exceeds max allowed (0.01): execute wasm contract failed'
    const out = humanizeUserFacingError(raw)
    expect(out).toContain('Trade rejected')
    expect(out).not.toMatch(/execute wasm contract failed/)
  })

  it('chains wallet then fetch', () => {
    expect(humanizeUserFacingError('signing rejected by user')).toMatch(/declined/)
  })

  it('humanizes WalletConnect timeout (GitLab #554)', () => {
    expect(humanizeUserFacingError("Wallet didn't respond. Try again.")).toMatch(/didn't respond/)
  })

  it('getErrorMessage coerces unknown', () => {
    expect(getErrorMessage({ message: 'x' })).toBe('[object Object]')
    expect(getErrorMessage(new Error(''))).toBe('Unknown error')
  })

  it('humanizeUserFacingErrorFromUnknown handles non-Error throws', () => {
    expect(humanizeUserFacingErrorFromUnknown('Failed to fetch')).toMatch(/Network request/)
  })

  it('humanizes bech32 checksum contract errors (GitLab #382)', () => {
    const raw =
      'failed to execute message; message index: 0: Generic error: addr_validate errored: decoding bech32 failed: invalid checksum (expected 3hz2wp got 3hz289): execute wasm contract failed'
    const out = humanizeUserFacingError(raw)
    expect(out).toMatch(/checksum does not match/i)
    expect(out).not.toMatch(/addr_validate|execute wasm contract failed/)
  })

  it('humanizes extension signed fee undershoot diagnostics (GitLab #371)', () => {
    const raw =
      'Wallet signed a fee far below what this dApp submitted (GitLab #127). On LocalTerra with Station: disconnect, reconnect, and approve any chain-update prompt. Run `cd frontend-dapp && npm ci` so the cosmes patch is applied, then retry. Expected at least ~50985000 uluna; wallet returned ~29 uluna. Expected gas at least ~1800000; wallet returned ~1.'
    const out = humanizeUserFacingError(raw)
    expect(out).toBe(
      'Transaction fee mismatch. Please reconnect your wallet and try again. If the problem persists, contact support.'
    )
    expect(out).not.toMatch(/GitLab|uluna|npm ci|Station/)
  })
})

describe('humanizeUserFacingError — market quote shapes (#414)', () => {
  it('humanizes indexer transport failures', () => {
    expect(humanizeUserFacingError('indexer unavailable')).toMatch(/temporarily unavailable/i)
    expect(humanizeUserFacingError('Hybrid quote unavailable')).toMatch(/Could not estimate output/i)
  })

  it('humanizes indexer 429 with calm retry guidance and no raw HTTP status (SEC-E04 / GitLab #426)', () => {
    const out = humanizeUserFacingError('Indexer API error: 429 Too Many Requests')
    expect(out).toMatch(/wait a moment and try again/i)
    expect(out).not.toMatch(/\b429\b/)
    expect(out).not.toMatch(/Indexer API error/i)
  })

  it('humanizes LCD and malformed JSON failures', () => {
    expect(humanizeUserFacingError('lcd fail')).toMatch(/reach the chain/i)
    expect(humanizeUserFacingError('Unexpected token < in JSON at position 0')).toMatch(/invalid response/i)
  })

  it('preserves actionable pause and blacklist messages', () => {
    const pause =
      'Pair is paused — swaps are blocked until governance unpauses. Your wallet balances are unchanged; limit escrow in the pair contract stays until unpause.'
    expect(humanizeUserFacingError(pause)).toBe(pause)
    const blacklist = 'Trading is restricted for this wallet address on-chain.'
    expect(humanizeUserFacingError(blacklist)).toBe(blacklist)
  })
})
