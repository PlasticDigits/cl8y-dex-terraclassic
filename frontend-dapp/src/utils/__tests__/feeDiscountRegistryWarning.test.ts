import { describe, expect, it } from 'vitest'
import {
  FEE_DISCOUNT_REGISTRY_WARNING_TEXT,
  resolveFeeDiscountRegistryStatus,
  shouldShowFeeDiscountRegistryWarning,
} from '../feeDiscountRegistryWarning'

describe('feeDiscountRegistryWarning', () => {
  it('returns unconfigured when fee-discount contract is absent', () => {
    expect(
      resolveFeeDiscountRegistryStatus({
        feeDiscountContractConfigured: false,
        registrationQueryError: false,
        discountQueryError: false,
      })
    ).toBe('unconfigured')
  })

  it('returns unregistered when LCD registration says not registered', () => {
    expect(
      resolveFeeDiscountRegistryStatus({
        feeDiscountContractConfigured: true,
        registration: { registered: false, tier_id: null, tier: null },
        registrationQueryError: false,
        discountQueryError: false,
      })
    ).toBe('unregistered')
  })

  it('returns registry_unreachable when registration LCD query fails', () => {
    expect(
      resolveFeeDiscountRegistryStatus({
        feeDiscountContractConfigured: true,
        registrationQueryError: true,
        discountQueryError: false,
      })
    ).toBe('registry_unreachable')
    expect(
      shouldShowFeeDiscountRegistryWarning({
        feeDiscountContractConfigured: true,
        registrationQueryError: true,
        discountQueryError: false,
      })
    ).toBe(true)
  })

  it('returns registry_unreachable when indexer health reports registry down', () => {
    expect(
      resolveFeeDiscountRegistryStatus({
        feeDiscountContractConfigured: true,
        registration: { registered: true, tier_id: 1, tier: null },
        registrationQueryError: false,
        discountQueryError: false,
        indexerHealth: {
          configured: true,
          fee_discount_registry_ok: false,
          consecutive_lcd_failures: 2,
        },
      })
    ).toBe('registry_unreachable')
  })

  it('returns registered when trader is registered and probes are healthy', () => {
    expect(
      resolveFeeDiscountRegistryStatus({
        feeDiscountContractConfigured: true,
        registration: { registered: true, tier_id: 1, tier: null },
        discount: { discount_bps: 50, needs_deregister: false, registration_epoch: 1 },
        registrationQueryError: false,
        discountQueryError: false,
        indexerHealth: {
          configured: true,
          fee_discount_registry_ok: true,
          consecutive_lcd_failures: 0,
        },
      })
    ).toBe('registered')
    expect(
      shouldShowFeeDiscountRegistryWarning({
        feeDiscountContractConfigured: true,
        registration: { registered: true, tier_id: 1, tier: null },
        discount: { discount_bps: 50, needs_deregister: false, registration_epoch: 1 },
        registrationQueryError: false,
        discountQueryError: false,
        indexerHealth: {
          configured: true,
          fee_discount_registry_ok: true,
          consecutive_lcd_failures: 0,
        },
      })
    ).toBe(false)
  })

  it('exposes stable warning copy', () => {
    expect(FEE_DISCOUNT_REGISTRY_WARNING_TEXT).toMatch(/full pair fee/i)
  })
})
