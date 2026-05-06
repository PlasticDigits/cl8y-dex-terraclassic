import { describe, expect, it } from 'vitest'
import { humanizeCosmwasmLimitOrderMissingMessage, orderIdHasIndexedCancellation } from '../limitOrderCancelUserMessage'

describe('humanizeCosmwasmLimitOrderMissingMessage', () => {
  it('maps LimitOrder map key not found', () => {
    const raw = `failed to execute message; message index: 0: type: cl8y_dex_pair::state::LimitOrder; key: [00, 0C, 6C, 69, 6D, 69, 74, 5F, 6F, 72, 64, 65, 72, 73, 00, 00, 00, 00, 00, 00, 00, 03] not found: execute wasm contract failed`
    const m = humanizeCosmwasmLimitOrderMissingMessage(raw)
    expect(m).toContain('no longer on the book')
  })

  it('returns null for unrelated errors', () => {
    expect(humanizeCosmwasmLimitOrderMissingMessage('out of gas')).toBeNull()
    expect(humanizeCosmwasmLimitOrderMissingMessage('paused')).toBeNull()
  })
})

describe('orderIdHasIndexedCancellation', () => {
  it('detects order id in list', () => {
    expect(orderIdHasIndexedCancellation([{ order_id: 3 }, { order_id: 9 }], 9)).toBe(true)
    expect(orderIdHasIndexedCancellation([{ order_id: 3 }], 9)).toBe(false)
  })
})
