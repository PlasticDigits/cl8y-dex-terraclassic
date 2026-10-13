import { LOCAL_ROUTE_UNAVAILABLE } from '@/utils/localRouteQuote'
import { OFFLINE_SWAP_BUDGET_UNAVAILABLE } from '@/services/terraclassic/offlineSwapBudget'

/** Presentation priority only. Execution gates are checked independently at submission. */
export function firstTradingNotice<K extends string>(candidates: readonly (readonly [K, boolean])[]): K | null {
  return candidates.find(([, active]) => active)?.[0] ?? null
}

/** Only known pre-sign refusals may be cleared by a new intent; uncertain/included failures remain visible. */
export function isPreSignTradingError(error: unknown): boolean {
  const message = error instanceof Error ? error.message : ''
  if (message === LOCAL_ROUTE_UNAVAILABLE || message === OFFLINE_SWAP_BUDGET_UNAVAILABLE) return true
  return /^(Connect wallet|Missing parameters|Token decimals unavailable|Quote unavailable|Insufficient Balance|Confirm the price difference first\.|User rejected request|Request rejected|Signing cancelled|Signing canceled|Transaction rejected by user|Trading is paused\.|Trading is temporarily unavailable\.|Trading is restricted\.|Price difference is too high|Quote exceeds your price protection\.)$/i.test(
    message
  )
}
