import type { LimitBookEditContext } from '@/types/limitBookTicketDraft'

function normalizeExpiresAt(value: number | null | undefined): number | null {
  return value ?? null
}

function pricesEqual(a: string, b: string): boolean {
  const na = a.trim()
  const nb = b.trim()
  if (na === nb) return true
  const fa = Number(na)
  const fb = Number(nb)
  if (Number.isFinite(fa) && Number.isFinite(fb)) return fa === fb
  return false
}

export function buildLimitBookEditContext(draft: {
  orderId: number
  side: 'bid' | 'ask'
  price: string
  amountHuman: string
  expiresAt?: number | null
}): LimitBookEditContext {
  return {
    orderId: draft.orderId,
    side: draft.side,
    price: draft.price.trim(),
    amountHuman: draft.amountHuman.trim(),
    expiresAt: normalizeExpiresAt(draft.expiresAt),
  }
}

/** True when only limit price changed — safe to call `UpdateLimitOrderPrice` (GitLab #247). */
export function isPriceOnlyLimitEdit(
  context: LimitBookEditContext | null,
  current: {
    side: 'bid' | 'ask'
    price: string
    amountHuman: string
    expiresAt: number | null
  }
): boolean {
  if (!context) return false
  if (context.side !== current.side) return false
  if (context.amountHuman !== current.amountHuman.trim()) return false
  if (normalizeExpiresAt(current.expiresAt) !== context.expiresAt) return false
  if (pricesEqual(context.price, current.price)) return false
  return true
}

export const LIMIT_EDIT_NON_PRICE_CHANGE_MESSAGE =
  'To change size, side, or expiry, cancel this order first, then place a new limit.'
