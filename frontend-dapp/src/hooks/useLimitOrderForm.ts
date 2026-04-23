import { useState } from 'react'
import { LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT } from '@/utils/limitOrderExpiry'

const DEFAULT_MAX = LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT

/**
 * Shared place-limit field state for the standalone limit page and the trade ticket.
 * Contract calls still use `max_adjust_steps` and optional `expires_at` (Unix sec) as before.
 */
export function useLimitOrderForm() {
  const [maxSteps, setMaxSteps] = useState(DEFAULT_MAX)
  const [expiresAt, setExpiresAt] = useState<number | null>(null)
  const [amountHuman, setAmountHuman] = useState('')
  const [limitAdvancedOpen, setLimitAdvancedOpen] = useState(false)

  return {
    maxSteps,
    setMaxSteps,
    expiresAt,
    setExpiresAt,
    amountHuman,
    setAmountHuman,
    limitAdvancedOpen,
    setLimitAdvancedOpen,
  }
}
