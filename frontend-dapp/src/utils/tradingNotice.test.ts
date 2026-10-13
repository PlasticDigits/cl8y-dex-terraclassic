import { describe, expect, it } from 'vitest'
import { firstTradingNotice, isPreSignTradingError } from './tradingNotice'

describe('notice presentation and transaction trust', () => {
  it('chooses one current blocker without weakening independent predicates', () => {
    const paused = true,
      restricted = true,
      priceBlocked = true
    expect(
      firstTradingNotice([
        ['paused', paused],
        ['restricted', restricted],
        ['price', priceBlocked],
      ])
    ).toBe('paused')
    expect([paused, restricted, priceBlocked]).toEqual([true, true, true])
  })
  it('only clears fixed known pre-sign refusals', () => {
    expect(isPreSignTradingError(new Error('Confirm the price difference first.'))).toBe(true)
    expect(isPreSignTradingError(new Error('User rejected request'))).toBe(true)
    expect(
      isPreSignTradingError(new Error('Quote unavailable: the response is invalid or no longer matches this trade.'))
    ).toBe(true)
    expect(isPreSignTradingError(new Error('Network fee unavailable for this route. Refresh the quote.'))).toBe(true)
    expect(isPreSignTradingError(new Error('Trading is paused.'))).toBe(true)
    expect(isPreSignTradingError(new Error('Broadcast status unknown — the transaction may still confirm.'))).toBe(
      false
    )
    expect(isPreSignTradingError(new Error('Transaction included with code 11; network fee paid.'))).toBe(false)
    expect(isPreSignTradingError(new Error('Failed to fetch'))).toBe(false)
  })
})
