import { describe, expect, it } from 'vitest'
import {
  BASE_GAS_LIMIT,
  DEREGISTER_FEE_DISCOUNT_GAS_LIMIT,
  REGISTER_FEE_DISCOUNT_GAS_LIMIT,
  getGasLimitForTx,
} from '../terraGas'

/** LocalTerra measured gas (GitLab #384); constants must exceed with margin. */
const MEASURED_REGISTER_GAS = 204_438
const MEASURED_DEREGISTER_GAS = 160_932

describe('getGasLimitForTx fee-discount register/deregister (GitLab #384)', () => {
  it('does not fall through to BASE_GAS_LIMIT for register', () => {
    const limit = getGasLimitForTx({ register: { tier_id: 1 } })
    expect(limit).toBe(REGISTER_FEE_DISCOUNT_GAS_LIMIT)
    expect(limit).toBeGreaterThan(BASE_GAS_LIMIT)
    expect(limit).toBeGreaterThan(MEASURED_REGISTER_GAS)
  })

  it('does not fall through to BASE_GAS_LIMIT for deregister', () => {
    const limit = getGasLimitForTx({ deregister: {} })
    expect(limit).toBe(DEREGISTER_FEE_DISCOUNT_GAS_LIMIT)
    expect(limit).toBeGreaterThan(BASE_GAS_LIMIT)
    expect(limit).toBeGreaterThan(MEASURED_DEREGISTER_GAS)
  })
})
