/**
 * Extra-debit sell max (GitLab #593 / #592 T592-2) + route policy (#607 / T592-13)
 * and Swap/Trade submit gate ([#1267](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1267)).
 * Pair-direct and official-router sells debit the trader `declared + tax` (router hop:
 * user Sends `declared` 1:1, then extra-debit `tax` from leftover). Max must leave
 * room so Swap/Trade cannot offer 100% of balance (**R607-7**). Typed 100% / reverse
 * offer must not broadcast when `TaxPreview.debit > balance` (**S1267**).
 */

import { fromRawAmount, toRawAmount } from '@/utils/formatAmount'
import { isDecimalAmountDraft } from '@/utils/decimalAmountInput'
import { COMMUNITY_TAX_BPS_DENOM } from '@/utils/communityTaxSku'

/** Largest `declared` such that `declared + floor(declared * sellBps / 10000) <= balance`. */
export function maxDeclaredForExtraDebitSell(balanceRaw: bigint, sellBps: number): bigint {
  if (balanceRaw <= 0n) return 0n
  const bps = Math.max(0, Math.floor(sellBps))
  if (bps === 0) return balanceRaw
  const denom = BigInt(COMMUNITY_TAX_BPS_DENOM + bps)
  return (balanceRaw * BigInt(COMMUNITY_TAX_BPS_DENOM)) / denom
}

export function applyExtraDebitSellCap(spendableRaw: bigint, sellBps: number | null | undefined): bigint {
  if (sellBps == null || sellBps <= 0) return spendableRaw
  const capped = maxDeclaredForExtraDebitSell(spendableRaw, sellBps)
  return capped < spendableRaw ? capped : spendableRaw
}

/**
 * Extra-debit Max for a connected wallet (#609 / C593-9).
 * Manager-directory exempt → 0 extra-debit (TaxPreview Honest).
 * Unknown exempt (`null`/`undefined`) keeps `sellBps` — fail closed, never unlock 100% early.
 */
export function effectiveExtraDebitSellBps(
  sellBps: number | null | undefined,
  managerExempt: boolean | null | undefined
): number | null {
  if (sellBps == null) return null
  if (managerExempt === true) return 0
  return sellBps
}

export function extraDebitSellHuman(balanceRaw: string, decimals: number, sellBps: number): string {
  let balance = 0n
  try {
    if (balanceRaw && /^\d+$/.test(balanceRaw)) balance = BigInt(balanceRaw)
  } catch {
    balance = 0n
  }
  const declared = extraDebitMaxDeclaredRaw(balance, sellBps, decimals)
  const human = fromRawAmount(declared.toString(), decimals)
  return isDecimalAmountDraft(human) ? human : '0'
}

/** Economic debit `declared + floor(declared * sellBps / 10000)` — same as wasm extra-debit, not a second formula. */
export function extraDebitFromDeclared(declaredRaw: bigint, sellBps: number): bigint {
  if (declaredRaw <= 0n) return 0n
  const bps = Math.max(0, Math.floor(sellBps))
  if (bps === 0) return declaredRaw
  return declaredRaw + (declaredRaw * BigInt(bps)) / BigInt(COMMUNITY_TAX_BPS_DENOM)
}

/** Parse LCD uint strings; hostile / non-numeric → `null` (no `BigInt` throw). */
export function parseUintString(value: unknown): bigint | null {
  if (typeof value === 'number') {
    if (!Number.isInteger(value) || !Number.isFinite(value) || value < 0) return null
    return BigInt(value)
  }
  if (typeof value !== 'string') return null
  const s = value.trim()
  if (!/^\d+$/.test(s)) return null
  try {
    return BigInt(s)
  } catch {
    return null
  }
}

export type CommunityTaxConfigParse = { kind: 'tax'; sellBps: number } | { kind: 'not_tax' } | { kind: 'unresolved' }

/** `GetConfig.sell_bps` is the live tax switch — not `code_id === VITE_COMMUNITY_TAX_CODE_ID`. */
export function parseCommunityTaxSellBps(cfg: unknown): CommunityTaxConfigParse {
  if (cfg == null || typeof cfg !== 'object') return { kind: 'unresolved' }
  const bps = (cfg as { sell_bps?: unknown }).sell_bps
  if (bps === undefined) return { kind: 'not_tax' }
  if (typeof bps === 'number' && Number.isFinite(bps) && bps >= 0) {
    return { kind: 'tax', sellBps: Math.floor(bps) }
  }
  if (typeof bps === 'string' && /^\d+$/.test(bps)) {
    return { kind: 'tax', sellBps: Number.parseInt(bps, 10) }
  }
  return { kind: 'unresolved' }
}

/**
 * CosmWasm unknown-query on an honest CW20 is not tax.
 * Timeouts / 5xx stay `unresolved` only when the caller already knows the instance is tax.
 */
export function classifyCommunityTaxQueryError(err: unknown): 'not_tax' | 'unresolved' {
  const msg = err instanceof Error ? err.message : String(err ?? '')
  if (/unknown variant/i.test(msg) || /Error parsing into type/i.test(msg) || /unknown query/i.test(msg)) {
    return 'not_tax'
  }
  return 'unresolved'
}

export function sellDebitExceedsBalance(input: { declaredRaw: bigint; balanceRaw: bigint; debitRaw: bigint }): boolean {
  return input.debitRaw > input.balanceRaw
}

/**
 * Largest declared whose human round-trip still has `debit ≤ balance`.
 * Steps down 1 raw unit if `fromRawAmount` → `toRawAmount` would overshoot.
 */
export function extraDebitMaxDeclaredRaw(balanceRaw: bigint, sellBps: number, decimals: number): bigint {
  let declared = maxDeclaredForExtraDebitSell(balanceRaw, sellBps)
  while (declared > 0n) {
    const human = fromRawAmount(declared.toString(), decimals)
    if (!isDecimalAmountDraft(human)) {
      declared -= 1n
      continue
    }
    let roundTrip: bigint
    try {
      const raw = toRawAmount(human, decimals)
      if (!/^\d+$/.test(raw)) {
        declared -= 1n
        continue
      }
      roundTrip = BigInt(raw)
    } catch {
      declared -= 1n
      continue
    }
    if (roundTrip <= declared && extraDebitFromDeclared(roundTrip, sellBps) <= balanceRaw) {
      return roundTrip
    }
    declared -= 1n
  }
  return 0n
}

/** Prefer LCD preview declared when its debit fits; else offline max. */
export function declaredRawForMax(input: {
  balanceRaw: bigint
  debitPreview?: { declared: unknown; debit: unknown } | null
  sellBps?: number | null
  decimals?: number
}): bigint {
  if (input.debitPreview) {
    const declared = parseUintString(input.debitPreview.declared)
    const debit = parseUintString(input.debitPreview.debit)
    if (declared != null && debit != null && debit <= input.balanceRaw && declared <= input.balanceRaw) {
      return declared
    }
  }
  const bps = input.sellBps
  if (bps == null || bps <= 0) return input.balanceRaw
  if (input.decimals != null) return extraDebitMaxDeclaredRaw(input.balanceRaw, bps, input.decimals)
  return maxDeclaredForExtraDebitSell(input.balanceRaw, bps)
}

export type ExtraDebitSubmitGate = {
  /** Classic #9 or extra-debit shortfall — CTA Insufficient Balance. */
  insufficientBalance: boolean
  /** Must not build or sign a tx (includes unresolved tax detection). */
  blockSubmit: boolean
}

/**
 * Swap/Trade execute gate (**S1267**). When `sell_bps > 0`, debit is
 * `max(LCD TaxPreview.debit, extraDebitFromDeclared(declared, sellBps))` so Honest LCD
 * (`debit === declared`, no `send_msg`) cannot enable a 100% tax sell. Missing LCD debit
 * still uses the local floor. Both unknown → fail closed (do not assume 0-tax).
 * Honest / `sell_bps = 0` stay `amount ≤ balance`.
 */
export function extraDebitSubmitGate(input: {
  declaredRaw: bigint | null
  balanceRaw: bigint | null
  debitRaw: bigint | null
  sellBps: number | null
  extraDebitUnresolved: boolean
  isNativePay: boolean
}): ExtraDebitSubmitGate {
  if (input.declaredRaw == null || input.declaredRaw <= 0n) {
    return { insufficientBalance: false, blockSubmit: false }
  }
  if (input.balanceRaw == null) {
    return { insufficientBalance: false, blockSubmit: false }
  }
  if (input.declaredRaw > input.balanceRaw) {
    return { insufficientBalance: true, blockSubmit: true }
  }
  if (input.isNativePay) {
    return { insufficientBalance: false, blockSubmit: false }
  }
  if (input.extraDebitUnresolved) {
    return { insufficientBalance: false, blockSubmit: true }
  }
  const localFloor =
    input.sellBps != null && input.sellBps > 0 ? extraDebitFromDeclared(input.declaredRaw, input.sellBps) : null
  if (input.debitRaw != null) {
    const debit = localFloor != null && localFloor > input.debitRaw ? localFloor : input.debitRaw
    const over = sellDebitExceedsBalance({
      declaredRaw: input.declaredRaw,
      balanceRaw: input.balanceRaw,
      debitRaw: debit,
    })
    return { insufficientBalance: over, blockSubmit: over }
  }
  if (localFloor != null) {
    const over = localFloor > input.balanceRaw
    return { insufficientBalance: over, blockSubmit: over }
  }
  return { insufficientBalance: false, blockSubmit: false }
}

export const INSUFFICIENT_FOR_SELL_TAX_TX_MESSAGE = 'Not enough tokens after sell tax. Reduce the amount or tap Max.'

export const SELL_TAX_EXTRA_HINT = 'Sell tax extra'
export const BUY_TAX_HINT = 'Buy tax applies'
/** Create/Manage + glossary (#607 / C593-14). */
export const COMMUNITY_TAX_SCOPE_COPY = 'Buy/sell tax applies on every listed-pair swap.'
/** @deprecated Use {@link COMMUNITY_TAX_SCOPE_COPY} — option 1 pair-direct-only wording. */
export const COMMUNITY_TAX_PAIR_DIRECT_COPY = COMMUNITY_TAX_SCOPE_COPY

/** Official dApp: router execute ⇔ `ops.length >= 2` (`swapOpsRequireRouter`). */
export function communityTaxExecuteUsesRouter(opsLength: number | undefined, clientMultiHop = false): boolean {
  return (opsLength ?? 0) >= 2 || clientMultiHop
}

/** Extra-debit Max on pair-direct **and** router-hop sells (**C593-9** / **R607-7**). */
export function extraDebitSellBpsForExecute(sellBps: number | null | undefined, usesRouter?: boolean): number | null {
  void usesRouter
  if (sellBps == null || sellBps <= 0) return null
  return sellBps
}

export function communityTaxRouteHint(input: {
  payIsTax: boolean
  receiveIsTax?: boolean
  usesRouter: boolean
  sellBps?: number | null
}): string | null {
  if (input.payIsTax && input.sellBps != null && input.sellBps > 0) {
    return SELL_TAX_EXTRA_HINT
  }
  if (input.receiveIsTax) {
    return BUY_TAX_HINT
  }
  return null
}
