import type { IndexerRouteSolveResponse } from '@/types'
import { getWrappedEquivalent, isNativeDenom } from '@/types'
import { TREASURY_CONTRACT_ADDRESS, WRAP_MAPPER_CONTRACT_ADDRESS } from '@/utils/constants'
import {
  hasFreshLocalQuoteWindow,
  hasValidLocalExecutionSupport,
  isLocalQuoteRaw,
  LOCAL_ROUTE_UNAVAILABLE,
} from '@/utils/localRouteQuote'
import { applySlippagePercentFloor } from '@/utils/rawAmountMath'
import { spreadMatchesSlippagePercent } from '@/utils/slippageConstraint'
import { quoteRelativeNetFloor, validatedLocalMarketSpread } from '@/utils/quoteRelativeProtection'

export const DIRECT_WRAPPER_FLOOR_UNSUPPORTED = 'Direct wrapping cannot enforce the reviewed receive floor'

/** Version 1 direct settlement is admitted only from independently observed
 * capabilities. Legacy arithmetic remains explicitly nonexecutable. */
export function validateWrapperOnlyQuote(
  idx: IndexerRouteSolveResponse,
  input: { simRaw: string; maxSpreadStr: string; slippageTolerancePercent: number; trader?: string; sender?: string },
  nowMs = Date.now()
) {
  const m = idx.execution_metadata
  const w = m?.wrapper
  const wrap = isNativeDenom(idx.token_in)
  const unwrap = isNativeDenom(idx.token_out)
  const floor = isLocalQuoteRaw(idx.estimated_amount_out)
    ? m?.protection_policy === 'quote_relative_v1'
      ? quoteRelativeNetFloor(idx.estimated_amount_out, input.maxSpreadStr)
      : applySlippagePercentFloor(idx.estimated_amount_out, input.slippageTolerancePercent)
    : null
  if (
    !m ||
    !hasValidLocalExecutionSupport(idx) ||
    !w ||
    m.version !== 1 ||
    m.policy !== 'wrapper_only' ||
    validatedLocalMarketSpread(m) == null ||
    (m.execution_supported !== true && m.execution_supported !== false) ||
    (m.execution_supported === true
      ? w.floor_protocol_version !== 1 || idx.execution_blocker != null
      : w.floor_protocol_version != null || idx.execution_blocker !== DIRECT_WRAPPER_FLOOR_UNSUPPORTED) ||
    idx.quote_kind !== 'indexer_wrapper_db' ||
    !m.generation?.trim() ||
    m.input_amount !== input.simRaw ||
    !isLocalQuoteRaw(input.simRaw) ||
    m.max_spread !== input.maxSpreadStr ||
    !spreadMatchesSlippagePercent(input.maxSpreadStr, input.slippageTolerancePercent) ||
    (m.trader ?? '') !== (input.trader ?? '') ||
    (m.sender ?? '') !== (input.sender ?? '') ||
    !hasFreshLocalQuoteWindow(m, nowMs) ||
    !Array.isArray(idx.router_operations) ||
    idx.router_operations.length !== 0 ||
    !Array.isArray(idx.hops) ||
    idx.hops.length !== 0 ||
    !Array.isArray(m.hops) ||
    m.hops.length !== 0 ||
    wrap === unwrap ||
    w.wrap_input !== wrap ||
    w.unwrap_output !== unwrap ||
    (wrap
      ? getWrappedEquivalent(idx.token_in) !== idx.token_out
      : getWrappedEquivalent(idx.token_out) !== idx.token_in) ||
    w.treasury !== TREASURY_CONTRACT_ADDRESS ||
    w.mapper !== WRAP_MAPPER_CONTRACT_ADDRESS ||
    !w.treasury ||
    !w.mapper ||
    w.input_token !== idx.token_in ||
    w.output_token !== idx.token_out ||
    w.input_amount !== input.simRaw ||
    !isLocalQuoteRaw(w.route_input_amount) ||
    w.route_input_amount !== w.route_output_amount ||
    (wrap ? BigInt(w.route_input_amount) > BigInt(input.simRaw) : w.route_input_amount !== input.simRaw) ||
    !isLocalQuoteRaw(idx.estimated_amount_out) ||
    idx.estimated_amount_out_net !== idx.estimated_amount_out ||
    BigInt(idx.estimated_amount_out) > BigInt(w.route_output_amount) ||
    !isLocalQuoteRaw(idx.spot_amount_out) ||
    !isLocalQuoteRaw(m.minimum_receive) ||
    !isLocalQuoteRaw(w.minimum_receive_base) ||
    floor == null ||
    BigInt(m.minimum_receive) < BigInt(floor) ||
    BigInt(m.minimum_receive) > BigInt(idx.estimated_amount_out) ||
    BigInt(w.minimum_receive_base) < BigInt(m.minimum_receive) ||
    BigInt(w.minimum_receive_base) > BigInt(w.route_output_amount) ||
    (!unwrap && (w.minimum_receive_base !== m.minimum_receive || w.route_output_amount !== idx.estimated_amount_out)) ||
    m.tax_funding_version != null ||
    idx.tax_funding_version != null ||
    idx.router_hops_tax ||
    idx.buy_tax_bps ||
    idx.sell_tax_bps ||
    (idx.tax_kind != null && idx.tax_kind !== 'none')
  )
    throw new Error(LOCAL_ROUTE_UNAVAILABLE)
  const actual = BigInt(idx.estimated_amount_out!)
  const fair = BigInt(idx.spot_amount_out!)
  if ((actual > fair ? actual - fair : fair - actual) * 100n >= (actual > fair ? actual : fair) * 99n)
    throw new Error(LOCAL_ROUTE_UNAVAILABLE)
  return m
}
