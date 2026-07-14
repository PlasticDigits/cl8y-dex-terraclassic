import { describe, expect, it } from 'vitest'
import {
  FEE_DISCOUNT_REGISTRY_WARNING_TEXT,
  resolveFeeDiscountRegistryStatus,
  shouldShowFeeDiscountRegistryWarning,
} from '@/utils/feeDiscountRegistryWarning'

describe('feeDiscountRegistryWarning', () => {
  it('exposes stable warning copy without LCD or address details', () => {
    expect(FEE_DISCOUNT_REGISTRY_WARNING_TEXT).toMatch(/full fee/i)
    expect(FEE_DISCOUNT_REGISTRY_WARNING_TEXT).not.toMatch(/terra1|@|error/i)
  })

  it('returns unconfigured when fee-discount contract is absent', () => {
    expect(
      resolveFeeDiscountRegistryStatus({
        feeDiscountContractConfigured: false,
        registrationQueryError: false,
        discountQueryError: false,
      })
    ).toBe('unconfigured')
  })

  describe('resolveFeeDiscountRegistryStatus', () => {
    it('returns unregistered when registration succeeded and wallet is not registered', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          feeDiscountContractConfigured: true,
          registration: { registered: false, tier_id: null, tier: null },
          registrationQueryError: false,
          discountQueryError: false,
        })
      ).toBe('unregistered')
    })

    it('returns registered when registration succeeded and wallet is registered', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          feeDiscountContractConfigured: true,
          registration: { registered: true, tier_id: 1, tier: null },
          registrationQueryError: false,
          discountQueryError: false,
        })
      ).toBe('registered')
    })

    it('returns registry_unreachable when registration LCD query fails', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          feeDiscountContractConfigured: true,
          registrationQueryError: true,
          discountQueryError: false,
        })
      ).toBe('registry_unreachable')
    })

    it('returns registry_unreachable when discount LCD query fails for a registered wallet', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          feeDiscountContractConfigured: true,
          registration: { registered: true, tier_id: 1, tier: null },
          registrationQueryError: false,
          discountQueryError: true,
        })
      ).toBe('registry_unreachable')
    })

    it('returns unregistered when only discount fails but registration succeeded as unregistered', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          feeDiscountContractConfigured: true,
          registration: { registered: false, tier_id: null, tier: null },
          registrationQueryError: false,
          discountQueryError: true,
        })
      ).toBe('unregistered')
    })

    it('returns registry_unreachable when indexer reports registry down', () => {
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

    it('keeps unregistered when indexer reports registry down but LCD registration read succeeded as unregistered', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          feeDiscountContractConfigured: true,
          registration: { registered: false, tier_id: null, tier: null },
          registrationQueryError: false,
          discountQueryError: false,
          indexerHealth: {
            configured: true,
            fee_discount_registry_ok: false,
            consecutive_lcd_failures: 1,
          },
        })
      ).toBe('unregistered')
    })
  })

  describe('shouldShowFeeDiscountRegistryWarning', () => {
    it('shows only for registry_unreachable', () => {
      expect(
        shouldShowFeeDiscountRegistryWarning({
          feeDiscountContractConfigured: true,
          registrationQueryError: true,
          discountQueryError: false,
        })
      ).toBe(true)
      expect(
        shouldShowFeeDiscountRegistryWarning({
          feeDiscountContractConfigured: true,
          registration: { registered: false, tier_id: null, tier: null },
          registrationQueryError: false,
          discountQueryError: false,
        })
      ).toBe(false)
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
  })
})
