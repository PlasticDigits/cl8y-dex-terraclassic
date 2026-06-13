export const FEE_DISCOUNT_REGISTRY_WARNING_TEXT =
  'Fee discount unavailable; full pair fee may apply until the registry is reachable again.'

export type FeeDiscountRegistryStatus = 'unregistered' | 'registry_unreachable' | 'registered'

export type FeeDiscountHealthSlice = {
  configured: boolean
  fee_discount_registry_ok: boolean | null
}

export type FeeDiscountRegistryInputs = {
  registrationSucceeded: boolean
  registered: boolean
  registrationQueryError: boolean
  discountQueryError: boolean
  indexerHealth?: FeeDiscountHealthSlice | null
}

/** Distinguish unregistered vs registry unreachable vs healthy registration (GitLab #374). */
export function resolveFeeDiscountRegistryStatus(inputs: FeeDiscountRegistryInputs): FeeDiscountRegistryStatus {
  const { registrationSucceeded, registered, registrationQueryError, discountQueryError, indexerHealth } = inputs

  const indexerReportsDown = indexerHealth?.configured === true && indexerHealth.fee_discount_registry_ok === false

  if (indexerReportsDown) {
    if (registrationSucceeded && !registered) {
      return 'unregistered'
    }
    return 'registry_unreachable'
  }

  if (registrationQueryError || discountQueryError) {
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

export function shouldShowFeeDiscountRegistryWarning(status: FeeDiscountRegistryStatus): boolean {
  return status === 'registry_unreachable'
}
