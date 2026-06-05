import { describe, expect, it } from 'vitest'
import { normalizeLimitBookPageResponse } from '../limitBookPagination'

describe('normalizeLimitBookPageResponse', () => {
  it('defaults missing orders to empty array (GitLab #327)', () => {
    const page = normalizeLimitBookPageResponse({
      side: 'bid',
      has_more: false,
      next_after_order_id: null,
    })
    expect(page.orders).toEqual([])
    expect(page.has_more).toBe(false)
  })

  it('drops null entries and preserves valid rows', () => {
    const page = normalizeLimitBookPageResponse({
      side: 'ask',
      orders: [null, { order_id: 1, owner: 'terra1a', side: 'ask', price: '1', remaining: '10' }] as never,
      has_more: true,
      next_after_order_id: 1,
    })
    expect(page.orders).toHaveLength(1)
    expect(page.has_more).toBe(true)
    expect(page.next_after_order_id).toBe(1)
  })
})
