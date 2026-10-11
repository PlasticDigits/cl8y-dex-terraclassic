import { describe, expect, it } from 'vitest'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import {
  isIndexerClientError,
  isIndexerPairNotFoundError,
  isIndexerRateLimitError,
  isIndexerRouteValidationError,
  isIndexerUnavailableError,
  isIndexerPricingDataPending,
} from '@/utils/indexerErrors'
import { IndexerApiError } from '@/utils/routeQuoteErrors'

describe('indexerErrors', () => {
  it('requires typed503 missing/stale observations for a pricing-data wait', () => {
    for (const reason of ['missing_observations', 'stale_observations'] as const) {
      expect(isIndexerPricingDataPending(new IndexerApiError(503, reason))).toBe(true)
      expect(isIndexerPricingDataPending(new IndexerApiError(422, reason))).toBe(false)
    }
    for (const error of [
      new Error('Indexer API error: 503 missing_observations'),
      new IndexerApiError(503),
      new IndexerApiError(503, 'accounting_unavailable'),
      new IndexerApiError(422, 'amount_too_small'),
      new IndexerApiError(422, 'protection'),
      new IndexerApiError(422, 'unsupported_semantics'),
    ]) {
      expect(isIndexerPricingDataPending(error)).toBe(false)
    }
  })
  it('keeps deterministic route rejection distinct from transport and other client failures (#1390)', () => {
    const error = new Error('Indexer API error: 422 Unprocessable Entity')
    expect(isIndexerRouteValidationError(error)).toBe(true)
    expect(isIndexerClientError(error)).toBe(true)
    expect(isIndexerUnavailableError(error)).toBe(false)
    expect(humanizeUserFacingErrorFromUnknown(error)).toMatch(/cannot execute/)
    expect(humanizeUserFacingErrorFromUnknown(error)).not.toMatch(/validation|refresh|smaller|disable hybrid/i)
    expect(humanizeUserFacingErrorFromUnknown(error)).not.toMatch(/indexer status|Market data service|422/i)
    expect(isIndexerRouteValidationError(new Error('Indexer API error: 503 Service Unavailable'))).toBe(false)
    expect(isIndexerRouteValidationError(new Error('Indexer API error: 404 Not Found'))).toBe(false)
  })
  it('treats indexer 404 as pair-not-found, not outage', () => {
    const err = new Error('Indexer API error: 404 Not Found')
    expect(isIndexerPairNotFoundError(err)).toBe(true)
    expect(isIndexerClientError(err)).toBe(true)
    expect(isIndexerUnavailableError(err)).toBe(false)
  })

  it('treats indexer 400 route-simulation failure as client error, not outage (GitLab #326)', () => {
    const err = new Error('Indexer API error: 400 Bad Request')
    expect(isIndexerPairNotFoundError(err)).toBe(false)
    expect(isIndexerClientError(err)).toBe(true)
    expect(isIndexerUnavailableError(err)).toBe(false)
  })

  it('treats indexer 429 as rate limit client error, not outage (SEC-E04 / GitLab #426)', () => {
    const err = new Error('Indexer API error: 429 Too Many Requests')
    expect(isIndexerRateLimitError(err)).toBe(true)
    expect(isIndexerClientError(err)).toBe(true)
    expect(isIndexerUnavailableError(err)).toBe(false)
  })

  it('treats indexer 502 as outage', () => {
    const err = new Error('Indexer API error: 502 Bad Gateway')
    expect(isIndexerPairNotFoundError(err)).toBe(false)
    expect(isIndexerClientError(err)).toBe(false)
    expect(isIndexerUnavailableError(err)).toBe(true)
  })
})
