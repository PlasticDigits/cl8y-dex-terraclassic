import { describe, expect, it } from 'vitest'
import {
  CANCEL_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT,
  CANCEL_LIMIT_ORDER_BATCH_PER_ORDER_GAS_LIMIT,
  gasLimitForLimitOrderCancelBatch,
} from '../terraGas'

describe('gasLimitForLimitOrderCancelBatch (GitLab #246)', () => {
  it('is monotonic in order count', () => {
    const a = gasLimitForLimitOrderCancelBatch(1)
    const b = gasLimitForLimitOrderCancelBatch(5)
    const c = gasLimitForLimitOrderCancelBatch(10)
    expect(b).toBeGreaterThan(a)
    expect(c).toBeGreaterThan(b)
  })

  it('matches base + per-order formula', () => {
    expect(gasLimitForLimitOrderCancelBatch(3)).toBe(
      CANCEL_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT + CANCEL_LIMIT_ORDER_BATCH_PER_ORDER_GAS_LIMIT * 3
    )
  })
})
