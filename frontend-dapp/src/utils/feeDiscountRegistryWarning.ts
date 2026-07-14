import type { DiscountResponse, RegistrationResponse } from '@/types'

export const FEE_DISCOUNT_REGISTRY_WARNING_TEXT = 'Fee discount unavailable; full fee may apply.'

/** @see feeDiscountUiCopy.ts for unregistered CTA / eligibility copy (GitLab #476). */

export type FeeDiscountRegistryStatus = 'unconfigured' | 'unregistered' | 'registered' | 'registry_unreachable'

export interface FeeDiscountHealthSnapshot {
  configured: boolean
  fee_discount_registry_ok: boolean | null
  consecutive_lcd_failures: number
}

export interface FeeDiscountRegistryWarningInput {
  feeDiscountContractConfigured: boolean
  registration?: RegistrationResponse | null
  discount?: DiscountResponse | null
  registrationQueryError: boolean
  discountQueryError: boolean
  indexerHealth?: FeeDiscountHealthSnapshot | null
}

/** Distinguish unregistered vs registry unreachable vs healthy registration (GitLab #374). */
export function resolveFeeDiscountRegistryStatus(input: FeeDiscountRegistryWarningInput): FeeDiscountRegistryStatus {
  if (!input.feeDiscountContractConfigured) {
    return 'unconfigured'
  }

  const registrationSucceeded = input.registration !== undefined
  const registered = input.registration?.registered ?? false

  const indexerReportsDown =
    input.indexerHealth?.configured === true && input.indexerHealth.fee_discount_registry_ok === false

  if (indexerReportsDown) {
    if (registrationSucceeded && !registered) {
      return 'unregistered'
    }
    return 'registry_unreachable'
  }

  if (input.registrationQueryError || input.discountQueryError) {
    if (registrationSucceeded && !registered) {
      return 'unregistered'
    }
    return 'registry_unreachable'
  }

  if (registered) {
    return 'registered'
  }

  return 'unregistered'
}

/** Non-blocking trader warning when registry outage may charge full fee despite registration. */
export function shouldShowFeeDiscountRegistryWarning(input: FeeDiscountRegistryWarningInput): boolean {
  return resolveFeeDiscountRegistryStatus(input) === 'registry_unreachable'
}
