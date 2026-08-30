/**
 * Classifiers for wallet extensions, fetch/indexer transport, and other off-chain errors.
 * Contract / LCD copy stays in `humanizeTerraTxError.ts` ([GitLab #134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)).
 * Surfaces covered here: [GitLab #145](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145).
 */

function norm(s: string): string {
  return s.trim()
}

import { CHUNK_LOAD_ROUTE_MESSAGE, isChunkLoadErrorMessage } from './chunkLoadError'
import { INDEXER_RATE_LIMIT_RETRY_MESSAGE } from './marketDataServiceCopy'
import {
  buildWrongNetworkConnectError,
  isWalletExtensionNotInstalledError,
  isWalletWrongNetworkError,
} from './walletNetworkError'

/** Wallet extension / WalletConnect — match substrings so prefixed SDK messages still classify. */
export function tryHumanizeWalletLikeMessage(message: string): string | null {
  const m = norm(message)
  if (!m) return null

  if (/walletconnect succeeded but/i.test(m)) {
    return 'WalletConnect finished connecting, but the wallet session did not complete. Close this and try again, or choose another wallet.'
  }

  if (/extension popup was closed/i.test(m) && !/transaction rejected by user/i.test(m)) {
    return 'Station closed the signing popup before the transaction completed. Disconnect and reconnect Station, approve any Terra Classic network update, then retry.'
  }

  if (/user rejected|rejected the request|request rejected|signing rejected|reject(ed)?\s+sign/i.test(m)) {
    return 'Wallet action was declined in the extension or mobile wallet. Nothing was submitted on-chain.'
  }

  if (/wallet didn't respond|wallet did not respond|walletconnect timed? ?out/i.test(m)) {
    return "Wallet didn't respond. Try again."
  }

  if (isWalletWrongNetworkError(m)) {
    const station = /station/i.test(m)
    const keplr = /keplr/i.test(m)
    const label = station ? 'Station' : keplr ? 'Keplr' : 'Your wallet'
    if (/switch.*network|wrong network|on the wrong network/i.test(m)) {
      return m.length <= 320 ? m : `${m.slice(0, 319)}…`
    }
    return buildWrongNetworkConnectError(label)
  }

  if (
    isWalletExtensionNotInstalledError(m) ||
    /install\s+(\w+\s+)?extension/i.test(m) ||
    /no\s+keplr\s+wallet\s+found/i.test(m)
  ) {
    return 'Wallet extension was not found. Install it for this browser, refresh the page, then try again.'
  }

  if (/unsupported\s+chain|wrong\s+network|chain\s+id\s+mismatch/i.test(m)) {
    return buildWrongNetworkConnectError('Your wallet')
  }

  return null
}

/** Browser `fetch`, indexer HTTP wrapper strings, and coarse transport failures. */
export function tryHumanizeFetchLikeMessage(message: string): string | null {
  const m = norm(message)
  if (!m) return null

  if (isChunkLoadErrorMessage(m)) {
    return CHUNK_LOAD_ROUTE_MESSAGE
  }

  if (/failed to fetch|networkerror when attempting to fetch|load failed|net::err_/i.test(m)) {
    return 'Network request failed. Check your connection, VPN, or whether the indexer/API is reachable, then retry.'
  }

  if (/aborterror|\baborted\b|the operation was aborted|signal is aborted without reason/i.test(m)) {
    return 'The request was cancelled or timed out. Try again.'
  }

  if (/cors|blocked by cors|not allowed by access-control-allow-origin/i.test(m)) {
    return 'Browser blocked the request (CORS). Confirm Vite origin matches indexer `CORS_ORIGINS` (see docs).'
  }

  if (/Indexer API error:\s*429\b/i.test(m) || /status code 429\b/i.test(m)) {
    return INDEXER_RATE_LIMIT_RETRY_MESSAGE
  }

  if (/Indexer API error:/i.test(m) && !/Indexer API error:\s*404\b/i.test(m)) {
    return 'Market data service returned an error. Wait a moment and retry, or check indexer status.'
  }

  if (/Indexer API error:\s*404\b/i.test(m)) {
    return 'Market data for this request was not found. The pair or route may not be indexed yet.'
  }

  if (/hybrid quote unavailable|quote unavailable/i.test(m)) {
    return 'Could not estimate output for this trade. Try a smaller amount or disable hybrid routing, then retry.'
  }

  if (/indexer (unavailable|down)|market data.*unavailable/i.test(m)) {
    return 'Market data is temporarily unavailable. Wait a moment and retry.'
  }

  if (/unexpected token|json parse|syntaxerror.*json/i.test(m)) {
    return 'Market data returned an invalid response. Wait a moment and retry.'
  }

  if (/lcd fail|query failed|status code 5\d\d/i.test(m)) {
    return 'Could not reach the chain to estimate this trade. Check your connection and retry.'
  }

  return null
}

/**
 * Last-resort scrub before showing unknown errors: drop obvious stack traces and cap length.
 * Preserves short, already-retail messages unchanged.
 */
export function sanitizeOpaqueErrorMessage(message: string): string {
  let s = norm(message)
  if (!s) return 'Something went wrong. Please try again.'

  if (/^cannot read properties of (null|undefined) \(reading '/i.test(s)) {
    return 'Something went wrong while loading this view. Wait a moment and try again.'
  }

  const lines = s.split('\n')
  if (lines.length > 1 && /\s+at\s+/.test(s)) {
    s = lines[0].trim()
  }

  const max = 240
  if (s.length > max) {
    return `${s.slice(0, max - 1)}…`
  }
  return s
}
