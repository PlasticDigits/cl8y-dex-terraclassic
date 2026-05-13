import { useCallback, useState } from 'react'
import { LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT } from '@/utils/limitOrderExpiry'

const DEFAULT_MAX = LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT

/** How the limit escrow amount was last set — drives Bid/Ask switch behavior ([GitLab #155](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155)). */
export type LimitEscrowAmountSource = 'empty' | 'manual' | 'max'

/**
 * Shared place-limit field state for the standalone limit page and the trade ticket.
 * Contract calls still use `max_adjust_steps` and optional `expires_at` (Unix sec) as before.
 */
export function useLimitOrderForm() {
  const [maxSteps, setMaxSteps] = useState(DEFAULT_MAX)
  const [expiresAt, setExpiresAt] = useState<number | null>(null)
  const [amountHuman, setAmountHumanState] = useState('')
  const [escrowAmountSource, setEscrowAmountSource] = useState<LimitEscrowAmountSource>('empty')
  const [limitAdvancedOpen, setLimitAdvancedOpen] = useState(false)

  const onLimitAmountInputChange = useCallback((v: string) => {
    setAmountHumanState(v)
    setEscrowAmountSource(v.trim() === '' ? 'empty' : 'manual')
  }, [])

  const onLimitAmountMax = useCallback((human: string) => {
    setEscrowAmountSource('max')
    setAmountHumanState(human)
  }, [])

  const resetLimitEscrowAmount = useCallback(() => {
    setAmountHumanState('')
    setEscrowAmountSource('empty')
  }, [])

  const setLimitEscrowAmountFromDraft = useCallback((human: string) => {
    setAmountHumanState(human)
    setEscrowAmountSource(human.trim() === '' ? 'empty' : 'manual')
  }, [])

  /** Re-apply MAX after side switch without leaving `max` mode (internal + effect consumer). */
  const setLimitEscrowAmountFromMaxReapply = useCallback((human: string) => {
    setAmountHumanState(human)
  }, [])

  return {
    maxSteps,
    setMaxSteps,
    expiresAt,
    setExpiresAt,
    amountHuman,
    escrowAmountSource,
    onLimitAmountInputChange,
    onLimitAmountMax,
    resetLimitEscrowAmount,
    setLimitEscrowAmountFromDraft,
    setLimitEscrowAmountFromMaxReapply,
    limitAdvancedOpen,
    setLimitAdvancedOpen,
  }
}
