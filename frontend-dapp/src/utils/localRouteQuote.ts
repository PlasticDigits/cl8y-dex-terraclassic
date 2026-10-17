import { spreadMatchesSlippagePercent } from '@/utils/slippageConstraint'
import { MAX_MAKER_FILLS_HARD_CAP } from '@/services/terraclassic/hybridBookWalkLimits'
import type { SwapOperation } from '@/services/terraclassic/router'
import type { IndexerRouteExecutionMetadata, IndexerRouteSolveResponse } from '@/types'
import { assetInfoLabel, getWrappedEquivalent, isNativeDenom } from '@/types'
import { TREASURY_CONTRACT_ADDRESS, WRAP_MAPPER_CONTRACT_ADDRESS } from '@/utils/constants'
import { applySlippagePercentFloor } from '@/utils/rawAmountMath'
import { quoteRelativeNetFloor, validatedLocalMarketSpread } from '@/utils/quoteRelativeProtection'
import { LOCAL_QUOTE_SIGNING_WINDOW_MS } from '@/utils/localQuoteWindow'
import {
  hybridNoBeliefMaterialPoolReject,
  hybridSpreadCmpAndTotal,
  spreadPercentOfGross,
  spreadRatioStrictlyExceedsMax,
} from '@/utils/swapMaxSpread'

export const LOCAL_ROUTE_UNAVAILABLE = 'Quote unavailable: the response is invalid or no longer matches this trade.'
const UINT128_MAX = (1n << 128n) - 1n
export function isLocalQuoteRaw(value: unknown, positive = true): value is string {
  return (
    typeof value === 'string' &&
    value.length <= 39 &&
    /^\d+$/.test(value) &&
    BigInt(value) <= UINT128_MAX &&
    (!positive || BigInt(value) > 0n)
  )
}
function reject(): never {
  throw new Error(LOCAL_ROUTE_UNAVAILABLE)
}

/** Omission retains reviewed compatibility; provided execution claims require Boolean semantics. */
export function hasValidLocalExecutionSupport(idx: IndexerRouteSolveResponse): boolean {
  const metadata = idx.execution_metadata
  if (!metadata) return false
  if (Object.hasOwn(metadata, 'execution_supported') && typeof metadata.execution_supported !== 'boolean') return false
  if (
    Object.hasOwn(idx, 'execution_supported') &&
    (typeof idx.execution_supported !== 'boolean' || idx.execution_supported !== (metadata.execution_supported ?? true))
  )
    return false
  return true
}

/** Freshness binds the original observation window; a new expiry cannot revive an old observation. */
export function hasFreshLocalQuoteWindow(
  metadata: Pick<IndexerRouteExecutionMetadata, 'observed_at_ms' | 'expires_at_ms'>,
  nowMs = Date.now()
): boolean {
  const observed = metadata.observed_at_ms,
    expires = metadata.expires_at_ms
  return (
    Number.isSafeInteger(nowMs) &&
    nowMs >= 0 &&
    Number.isSafeInteger(observed) &&
    observed >= 0 &&
    observed <= nowMs &&
    Number.isSafeInteger(expires) &&
    expires > nowMs &&
    expires > observed &&
    expires - observed <= LOCAL_QUOTE_SIGNING_WINDOW_MS
  )
}

/** The reviewed snapshot binds exact hybrid budgets, cap and hint at mutation time. */
function sameDeclaredHybrid(
  pinned: IndexerRouteExecutionMetadata['hops'][number]['declared_hybrid'],
  actual: SwapOperation['terra_swap']['hybrid']
): boolean {
  if (pinned === undefined) return false
  if (pinned === null) return actual == null
  return (
    actual != null &&
    pinned.pool_input === actual.pool_input &&
    pinned.book_input === actual.book_input &&
    pinned.max_maker_fills === actual.max_maker_fills &&
    (pinned.book_start_hint ?? null) === (actual.book_start_hint ?? null)
  )
}

/** Validate the exact local observation and emitted plan; no discovery, simulation or RPC. */
export function validateLocalRouteQuote(
  idx: IndexerRouteSolveResponse,
  operations: SwapOperation[],
  input: {
    simRaw: string
    maxSpreadStr: string
    slippageTolerancePercent: number
    trader?: string
    sender?: string
    maxMakerFills?: number
  },
  nowMs = Date.now()
) {
  if (
    !isLocalQuoteRaw(input.simRaw) ||
    input.maxSpreadStr.length > 20 ||
    !/^0(?:\.\d+)?$/.test(input.maxSpreadStr) ||
    Number(input.maxSpreadStr) <= 0 ||
    Number(input.maxSpreadStr) > 0.5 ||
    !spreadMatchesSlippagePercent(input.maxSpreadStr, input.slippageTolerancePercent)
  )
    reject()
  const m = idx.execution_metadata
  if (
    !m ||
    !hasValidLocalExecutionSupport(idx) ||
    m.version !== 1 ||
    m.execution_supported === false ||
    typeof m.generation !== 'string' ||
    !m.generation.trim() ||
    !['pool_only', 'declared_hybrid_hop0', 'declared_hybrid_manual'].includes(m.policy) ||
    !(
      (m.tax_funding_version == null &&
        idx.tax_funding_version == null &&
        (idx.tax_kind == null || idx.tax_kind === 'none') &&
        !idx.buy_tax_bps &&
        !idx.sell_tax_bps &&
        !idx.router_hops_tax) ||
      (m.tax_funding_version === 1 &&
        idx.tax_funding_version === 1 &&
        idx.tax_kind === 'route_local_v1' &&
        idx.router_hops_tax === true)
    ) ||
    m.input_amount !== input.simRaw ||
    m.max_spread !== input.maxSpreadStr ||
    (m.trader ?? '') !== (input.trader ?? '') ||
    (m.sender ?? '') !== (input.sender ?? '') ||
    !hasFreshLocalQuoteWindow(m, nowMs) ||
    !isLocalQuoteRaw(idx.estimated_amount_out) ||
    !isLocalQuoteRaw(m.minimum_receive) ||
    !Array.isArray(idx.hops) ||
    idx.hops.length !== operations.length ||
    !Array.isArray(m.hops) ||
    m.hops.length !== operations.length ||
    !operations.length ||
    operations.length > 4
  )
    reject()
  // A missing fair reference is explicit, never substituted with an invented impact.
  if (!isLocalQuoteRaw(idx.spot_amount_out)) reject()
  const executionMarketSpread = validatedLocalMarketSpread(m)
  if (executionMarketSpread == null) reject()
  const w = m.wrapper
  if (
    w &&
    (w.input_token !== idx.token_in ||
      w.output_token !== idx.token_out ||
      w.input_amount !== input.simRaw ||
      w.wrap_input !== isNativeDenom(idx.token_in) ||
      w.unwrap_output !== isNativeDenom(idx.token_out) ||
      (!w.wrap_input && !w.unwrap_output) ||
      w.treasury !== TREASURY_CONTRACT_ADDRESS ||
      w.mapper !== WRAP_MAPPER_CONTRACT_ADDRESS ||
      !w.treasury ||
      !w.mapper ||
      !isLocalQuoteRaw(w.route_input_amount) ||
      !isLocalQuoteRaw(w.route_output_amount) ||
      !isLocalQuoteRaw(w.minimum_receive_base) ||
      BigInt(w.route_input_amount) > BigInt(input.simRaw) ||
      (!w.wrap_input && w.route_input_amount !== input.simRaw) ||
      (w.wrap_input && m.hops[0].offer_token !== getWrappedEquivalent(idx.token_in)) ||
      (w.unwrap_output && m.hops[m.hops.length - 1].ask_token !== getWrappedEquivalent(idx.token_out)))
  )
    reject()
  let current = w?.route_input_amount ?? input.simRaw
  const visited = new Set<string>()
  let currentToken = w?.wrap_input ? getWrappedEquivalent(idx.token_in) : idx.token_in
  if (!currentToken) reject()
  visited.add(currentToken)
  let worstNum = 0n
  let worstDen = 1n
  for (const [i, op] of operations.entries()) {
    const h = m.hops[i]
    const declared = idx.hops[i]
    const ts = op.terra_swap
    if (
      !h ||
      !declared ||
      h.offer_token !== currentToken ||
      visited.has(h.ask_token) ||
      h.pair !== declared.pair ||
      !h.pair ||
      h.offer_token !== declared.offer_token ||
      h.ask_token !== declared.ask_token ||
      h.offer_amount !== current ||
      !isLocalQuoteRaw(h.return_amount) ||
      !isLocalQuoteRaw(h.min_return) ||
      !isLocalQuoteRaw(h.spread_amount, false) ||
      !isLocalQuoteRaw(h.commission_amount, false) ||
      !isLocalQuoteRaw(h.pool_return_amount, false) ||
      !isLocalQuoteRaw(h.book_return_net, false) ||
      !isLocalQuoteRaw(h.pool_input_actual, false) ||
      !isLocalQuoteRaw(h.book_offer_consumed, false) ||
      ts.min_return !== h.min_return ||
      !sameDeclaredHybrid(h.declared_hybrid, ts.hybrid) ||
      assetInfoLabel(ts.offer_asset_info) !== declared.offer_token ||
      assetInfoLabel(ts.ask_asset_info) !== declared.ask_token
    )
      reject()
    if (BigInt(h.return_amount) > BigInt(h.pool_return_amount) + BigInt(h.book_return_net)) reject()
    const floor =
      m.protection_policy === 'quote_relative_v1'
        ? quoteRelativeNetFloor(h.return_amount, input.maxSpreadStr)
        : applySlippagePercentFloor(h.return_amount, input.slippageTolerancePercent)
    if (floor == null || BigInt(h.min_return) < BigInt(floor) || BigInt(h.min_return) > BigInt(h.return_amount))
      reject()
    let economic = BigInt(h.offer_amount)
    if (m.tax_funding_version === 1) {
      if (
        !isLocalQuoteRaw(h.economic_offer_amount) ||
        !isLocalQuoteRaw(h.sell_tax_amount, false) ||
        !isLocalQuoteRaw(h.sell_residue, false) ||
        !isLocalQuoteRaw(h.buy_tax_amount, false) ||
        !isLocalQuoteRaw(h.post_hook_return_amount) ||
        !Number.isInteger(h.sell_tax_bps) ||
        !Number.isInteger(h.buy_tax_bps) ||
        h.sell_tax_bps! < 0 ||
        h.sell_tax_bps! > 10000 ||
        h.buy_tax_bps! < 0 ||
        h.buy_tax_bps! > 10000
      )
        reject()
      economic = BigInt(h.economic_offer_amount)
      const sellRate = BigInt(h.sell_tax_bps!),
        buyRate = BigInt(h.buy_tax_bps!)
      const sellTax = (economic * sellRate) / 10000n
      if (
        sellTax !== BigInt(h.sell_tax_amount) ||
        economic + sellTax + BigInt(h.sell_residue) !== BigInt(h.offer_amount) ||
        economic + 1n + ((economic + 1n) * sellRate) / 10000n <= BigInt(h.offer_amount) ||
        (BigInt(h.post_hook_return_amount) * buyRate) / 10000n !== BigInt(h.buy_tax_amount) ||
        BigInt(h.post_hook_return_amount) - BigInt(h.buy_tax_amount) !== BigInt(h.return_amount) ||
        BigInt(h.post_hook_return_amount) > BigInt(h.pool_return_amount) + BigInt(h.book_return_net)
      )
        reject()
    } else if (
      (h.economic_offer_amount != null && BigInt(h.economic_offer_amount) !== economic) ||
      (h.sell_tax_amount != null && h.sell_tax_amount !== '0') ||
      (h.sell_residue != null && h.sell_residue !== '0') ||
      h.sell_tax_bps ||
      h.buy_tax_bps ||
      (h.buy_tax_amount != null && h.buy_tax_amount !== '0')
    )
      reject()
    if (BigInt(h.pool_input_actual) + BigInt(h.book_offer_consumed) !== economic) reject()
    if (ts.hybrid != null) {
      const hybrid = ts.hybrid
      if (
        (m.policy !== 'declared_hybrid_manual' && (i !== 0 || m.policy !== 'declared_hybrid_hop0')) ||
        !isLocalQuoteRaw(hybrid.pool_input, false) ||
        !isLocalQuoteRaw(hybrid.book_input, false) ||
        BigInt(hybrid.pool_input) + BigInt(hybrid.book_input) !== economic ||
        !Number.isSafeInteger(hybrid.max_maker_fills) ||
        hybrid.max_maker_fills < 1 ||
        hybrid.max_maker_fills > (input.maxMakerFills ?? 8) ||
        hybrid.max_maker_fills > MAX_MAKER_FILLS_HARD_CAP ||
        hybrid.book_start_hint != null ||
        BigInt(h.book_offer_consumed) > BigInt(hybrid.book_input) ||
        hybridNoBeliefMaterialPoolReject(
          economic,
          BigInt(hybrid.pool_input),
          BigInt(hybrid.book_input),
          BigInt(h.pool_return_amount)
        )
      )
        reject()
    } else if (
      (i === 0 && m.policy !== 'pool_only' && m.policy !== 'declared_hybrid_manual') ||
      BigInt(h.book_return_net) !== 0n ||
      BigInt(h.book_offer_consumed) !== 0n
    )
      reject()
    const legs = { poolInput: BigInt(h.pool_input_actual), bookInput: BigInt(h.book_offer_consumed) }
    const { spreadCmp: spread, totalGrossOut: gross } = hybridSpreadCmpAndTotal(
      { ...h, book_return_amount: h.book_return_net },
      legs
    )
    if (spreadRatioStrictlyExceedsMax(spread, gross, executionMarketSpread)) reject()
    if (spread * worstDen > worstNum * gross) {
      worstNum = spread
      worstDen = gross
    }
    current = h.return_amount
    currentToken = h.ask_token
    visited.add(currentToken)
  }
  if (currentToken !== (w?.unwrap_output ? getWrappedEquivalent(idx.token_out) : idx.token_out)) reject()
  const finalFloor =
    m.protection_policy === 'quote_relative_v1'
      ? quoteRelativeNetFloor(idx.estimated_amount_out!, input.maxSpreadStr)
      : applySlippagePercentFloor(idx.estimated_amount_out!, input.slippageTolerancePercent)
  if (
    (w
      ? current !== w.route_output_amount || BigInt(idx.estimated_amount_out!) > BigInt(current)
      : current !== idx.estimated_amount_out) ||
    (idx.estimated_amount_out_net != null && idx.estimated_amount_out_net !== idx.estimated_amount_out) ||
    finalFloor == null ||
    BigInt(m.minimum_receive) < BigInt(finalFloor) ||
    BigInt(m.minimum_receive) > BigInt(idx.estimated_amount_out!)
  )
    reject()
  if (
    w &&
    (BigInt(w.minimum_receive_base) < BigInt(m.minimum_receive) ||
      BigInt(w.minimum_receive_base) > BigInt(w.route_output_amount) ||
      (!w.unwrap_output && (w.minimum_receive_base !== m.minimum_receive || current !== idx.estimated_amount_out)))
  )
    reject()
  return {
    metadata: m,
    routePreflight: {
      worstSpreadPercent: spreadPercentOfGross(worstNum, worstDen),
      anyHopExceedsMaxSpread: false,
      anyHopMissingSlippageFloor: false,
    },
  }
}

/** Direct wrapper mutation guard: bind capability, direction, actor and the
 * wallet receive floor. The external protocol measures actual recipient delta. */
export function assertWrapperOnlySubmit(
  metadata: IndexerRouteExecutionMetadata,
  operations: SwapOperation[],
  amount: string,
  maxSpread: string,
  trader: string,
  minimumReceive: string | undefined,
  nowMs = Date.now()
): void {
  if (validatedLocalMarketSpread(metadata) == null) reject()
  const w = metadata.wrapper
  if (metadata.protection_policy === 'quote_relative_v1' && w?.wrap_input) {
    const floor = quoteRelativeNetFloor(w.route_output_amount, maxSpread)
    if (floor == null || !isLocalQuoteRaw(metadata.minimum_receive) || BigInt(metadata.minimum_receive) < BigInt(floor))
      reject()
  }
  if (
    !w ||
    metadata.version !== 1 ||
    metadata.policy !== 'wrapper_only' ||
    metadata.execution_supported !== true ||
    w.floor_protocol_version !== 1 ||
    !metadata.generation?.trim() ||
    metadata.input_amount !== amount ||
    metadata.max_spread !== maxSpread ||
    (metadata.trader ?? '') !== trader ||
    (metadata.sender != null && metadata.sender !== trader) ||
    !hasFreshLocalQuoteWindow(metadata, nowMs) ||
    operations.length !== 0 ||
    metadata.hops.length !== 0 ||
    metadata.tax_funding_version != null ||
    w.treasury !== TREASURY_CONTRACT_ADDRESS ||
    w.mapper !== WRAP_MAPPER_CONTRACT_ADDRESS ||
    !w.treasury ||
    !w.mapper ||
    w.input_amount !== amount ||
    w.wrap_input === w.unwrap_output ||
    w.wrap_input !== isNativeDenom(w.input_token) ||
    w.unwrap_output !== isNativeDenom(w.output_token) ||
    (w.wrap_input
      ? getWrappedEquivalent(w.input_token) !== w.output_token
      : getWrappedEquivalent(w.output_token) !== w.input_token) ||
    !isLocalQuoteRaw(amount) ||
    !isLocalQuoteRaw(w.route_input_amount) ||
    w.route_input_amount !== w.route_output_amount ||
    (w.wrap_input ? BigInt(w.route_input_amount) > BigInt(amount) : w.route_input_amount !== amount) ||
    !isLocalQuoteRaw(metadata.minimum_receive) ||
    !isLocalQuoteRaw(minimumReceive) ||
    minimumReceive !== metadata.minimum_receive ||
    BigInt(metadata.minimum_receive) > BigInt(w.route_output_amount)
  )
    reject()
}

/** Mutation-time snapshot guard; preserves the reviewed floors without refreshing the route. */
export function assertLocalRouteSubmit(
  metadata: IndexerRouteExecutionMetadata,
  operations: SwapOperation[],
  amount: string,
  maxSpread: string,
  trader: string,
  minimumReceive: string | undefined,
  nowMs = Date.now()
): void {
  if (metadata.policy === 'wrapper_only') {
    assertWrapperOnlySubmit(metadata, operations, amount, maxSpread, trader, minimumReceive, nowMs)
    return
  }
  if (
    metadata.version !== 1 ||
    (Object.hasOwn(metadata, 'execution_supported') && metadata.execution_supported !== true) ||
    metadata.execution_supported === false ||
    !['pool_only', 'declared_hybrid_hop0', 'declared_hybrid_manual'].includes(metadata.policy) ||
    (metadata.tax_funding_version != null && metadata.tax_funding_version !== 1) ||
    !metadata.generation ||
    metadata.input_amount !== amount ||
    metadata.max_spread !== maxSpread ||
    (metadata.trader ?? '') !== trader ||
    (metadata.sender != null && metadata.sender !== trader) ||
    !hasFreshLocalQuoteWindow(metadata, nowMs) ||
    !isLocalQuoteRaw(minimumReceive) ||
    BigInt(minimumReceive) < BigInt(metadata.wrapper?.minimum_receive_base ?? metadata.minimum_receive) ||
    !operations.length ||
    metadata.hops.length !== operations.length
  )
    reject()
  if (validatedLocalMarketSpread(metadata) == null) reject()
  if (metadata.protection_policy === 'quote_relative_v1' && !metadata.wrapper) {
    const finalFloor = quoteRelativeNetFloor(metadata.hops.at(-1)?.return_amount, maxSpread)
    if (
      finalFloor == null ||
      !isLocalQuoteRaw(metadata.minimum_receive) ||
      BigInt(metadata.minimum_receive) < BigInt(finalFloor)
    )
      reject()
  }
  for (const [i, op] of operations.entries()) {
    const pinnedFloor =
      metadata.protection_policy === 'quote_relative_v1'
        ? quoteRelativeNetFloor(metadata.hops[i]?.return_amount, maxSpread)
        : undefined
    if (
      metadata.protection_policy === 'quote_relative_v1' &&
      (pinnedFloor == null ||
        !isLocalQuoteRaw(metadata.hops[i]?.min_return) ||
        BigInt(metadata.hops[i].min_return) < BigInt(pinnedFloor))
    )
      reject()
    if (
      assetInfoLabel(op.terra_swap.offer_asset_info) !== metadata.hops[i].offer_token ||
      assetInfoLabel(op.terra_swap.ask_asset_info) !== metadata.hops[i].ask_token ||
      (op.terra_swap.hybrid != null &&
        metadata.policy !== 'declared_hybrid_manual' &&
        (i !== 0 || metadata.policy !== 'declared_hybrid_hop0')) ||
      !isLocalQuoteRaw(op.terra_swap.min_return) ||
      op.terra_swap.min_return !== metadata.hops[i].min_return ||
      !sameDeclaredHybrid(metadata.hops[i].declared_hybrid, op.terra_swap.hybrid)
    )
      reject()
  }
}
