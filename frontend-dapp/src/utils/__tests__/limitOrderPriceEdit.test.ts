import { describe, expect, it } from 'vitest'
import {
  buildLimitBookEditContext,
  isPriceOnlyLimitEdit,
  LIMIT_EDIT_NON_PRICE_CHANGE_MESSAGE,
} from '../limitOrderPriceEdit'

describe('limitOrderPriceEdit', () => {
  const context = buildLimitBookEditContext({
    orderId: 42,
    side: 'bid',
    price: '1.0',
    amountHuman: '100',
    expiresAt: null,
  })

  it('detects price-only edit when price changes', () => {
    expect(
      isPriceOnlyLimitEdit(context, {
        side: 'bid',
        price: '0.95',
        amountHuman: '100',
        expiresAt: null,
      })
    ).toBe(true)
  })

  it('rejects when amount changes', () => {
    expect(
      isPriceOnlyLimitEdit(context, {
        side: 'bid',
        price: '0.95',
        amountHuman: '99',
        expiresAt: null,
      })
    ).toBe(false)
  })

  it('rejects when side changes', () => {
    expect(
      isPriceOnlyLimitEdit(context, {
        side: 'ask',
        price: '0.95',
        amountHuman: '100',
        expiresAt: null,
      })
    ).toBe(false)
  })

  it('rejects when price unchanged', () => {
    expect(
      isPriceOnlyLimitEdit(context, {
        side: 'bid',
        price: '1.0',
        amountHuman: '100',
        expiresAt: null,
      })
    ).toBe(false)
  })

  it('exports non-price change copy', () => {
    expect(LIMIT_EDIT_NON_PRICE_CHANGE_MESSAGE).toMatch(/cancel/i)
  })
})
