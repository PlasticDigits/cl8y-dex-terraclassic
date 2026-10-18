import { describe, expect, it } from 'vitest'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import { isIndexerRouteValidationError, isIndexerUnavailableError } from '@/utils/indexerErrors'
import { currentIndicativeAmount, parseRouteQuoteError, ROUTE_QUOTE_REASONS } from './routeQuoteErrors'
import { LOCAL_QUOTE_SIGNING_WINDOW_MS } from '@/utils/localQuoteWindow'

const now = 1_800_000_000_000
const priceOnly = {
  error: 'protection',
  execution_supported: false,
  indicative_amount_out: '147439300000000000000',
  observed_at_ms: now - 1000,
  expires_at_ms: now + 19_000,
  generation: 'fixture1',
}

describe('public quote reason boundary (#1473)', () => {
  it.each(Object.keys(ROUTE_QUOTE_REASONS))('maps %s without reflecting remote prose', (reason) => {
    const status = ['missing_observations', 'stale_observations', 'accounting_unavailable'].includes(reason)
      ? 503
      : reason === 'invalid_request'
        ? 400
        : 422
    const error = parseRouteQuoteError(
      status,
      JSON.stringify({ error: reason, message: 'private body actor endpoint' })
    )
    expect(humanizeUserFacingErrorFromUnknown(error)).toBe(
      ROUTE_QUOTE_REASONS[reason as keyof typeof ROUTE_QUOTE_REASONS]
    )
    expect(error.message).not.toContain('private')
    expect(error.indicativeAmountOut).toBeUndefined()
    expect(isIndexerRouteValidationError(error)).toBe(status === 422)
    expect(isIndexerUnavailableError(error)).toBe(false)
  })

  it('accepts only a fresh protection estimate and never returns signing fields', () => {
    const error = parseRouteQuoteError(
      422,
      JSON.stringify({ ...priceOnly, router_operations: ['poison'], execution_metadata: {} }),
      now
    )
    expect(currentIndicativeAmount(error, true, now)).toBe(priceOnly.indicative_amount_out)
    expect(currentIndicativeAmount(error, false, now)).toBeUndefined()
    expect(currentIndicativeAmount(error, true, now + 19_000)).toBeUndefined()
    expect(error).not.toHaveProperty('router_operations')
    expect(error).not.toHaveProperty('execution_metadata')
  })

  it.each(['0', '-1', '01', '1e18', '1.0', '9'.repeat(40), (1n << 128n).toString()])(
    'drops noncanonical/overflow estimate %s',
    (amount) => {
      expect(
        parseRouteQuoteError(422, JSON.stringify({ ...priceOnly, indicative_amount_out: amount }), now)
          .indicativeAmountOut
      ).toBeUndefined()
    }
  )

  it.each([
    { error: 'insufficient_liquidity' },
    { execution_supported: true },
    { expires_at_ms: now },
    { expires_at_ms: now + LOCAL_QUOTE_SIGNING_WINDOW_MS + 1 },
    { observed_at_ms: now + 1 },
    { observed_at_ms: -1 },
    { observed_at_ms: now - LOCAL_QUOTE_SIGNING_WINDOW_MS },
    { generation: undefined },
    { generation: 'private.example/actor' },
  ])('drops unsafe indicative provenance %j', (changes) => {
    expect(
      parseRouteQuoteError(422, JSON.stringify({ ...priceOnly, ...changes }), now).indicativeAmountOut
    ).toBeUndefined()
  })

  it.each([
    '<html>private upstream</html>',
    '{',
    'null',
    '[]',
    '{"error":"__proto__"}',
    '{"error":"constructor"}',
    '{"error":"private secret"}',
    'x'.repeat(4097),
  ])('keeps unknown/malformed bodies authoritative and private', (text) => {
    const error = parseRouteQuoteError(422, text, now)
    expect(isIndexerRouteValidationError(error)).toBe(true)
    expect(isIndexerUnavailableError(error)).toBe(false)
    expect(error.quoteReason).toBeUndefined()
    expect(humanizeUserFacingErrorFromUnknown(error)).toBe(
      'This trade cannot execute with the current amount and settings.'
    )
  })

  it('does not accept a valid public reason with inconsistent HTTP status', () => {
    expect(parseRouteQuoteError(503, JSON.stringify(priceOnly), now).quoteReason).toBeUndefined()
  })
})
