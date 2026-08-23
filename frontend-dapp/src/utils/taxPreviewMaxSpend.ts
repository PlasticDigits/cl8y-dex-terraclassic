/**
 * Extra-debit sell max (GitLab #593 / #592 T592-2) + route policy (#607 / T592-13).
 * Pair-direct sell to a listed pair debits `declared + tax`. Max must leave room for
 * the extra debit so Swap/Trade cannot offer 100% of balance (self-DoS / failed tx spam).
 * Router hops are Honest — do not cap Max as if extra-debit will fire (**R607-7**).
 */

import { fromRawAmount } from '@/utils/formatAmount'
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
  const declared = maxDeclaredForExtraDebitSell(balance, sellBps)
  const human = fromRawAmount(declared.toString(), decimals)
  return isDecimalAmountDraft(human) ? human : '0'
}

export const SELL_TAX_EXTRA_HINT = 'Sell tax extra'
/** Create/Manage + glossary (#607 / C593-14). */
export const COMMUNITY_TAX_PAIR_DIRECT_COPY = 'Buy/sell tax is pair-direct only.'
/** Swap/Trade when execute uses the router (ops.length >= 2). */
export const ROUTER_TAX_SKIP_HINT = 'Route skips buy/sell tax'

/** Official dApp: router execute ⇔ `ops.length >= 2` (`swapOpsRequireRouter`). */
export function communityTaxExecuteUsesRouter(opsLength: number | undefined, clientMultiHop = false): boolean {
  return (opsLength ?? 0) >= 2 || clientMultiHop
}

/** Extra-debit Max only on pair-direct execute (**C593-9** / **R607-7**). */
export function extraDebitSellBpsForExecute(sellBps: number | null | undefined, usesRouter: boolean): number | null {
  if (usesRouter) return null
  if (sellBps == null || sellBps <= 0) return null
  return sellBps
}

export function communityTaxRouteHint(input: {
  payIsTax: boolean
  receiveIsTax?: boolean
  usesRouter: boolean
  sellBps?: number | null
}): string | null {
  if (input.usesRouter && (input.payIsTax || input.receiveIsTax)) {
    return ROUTER_TAX_SKIP_HINT
  }
  if (input.payIsTax && input.sellBps != null && input.sellBps > 0) {
    return SELL_TAX_EXTRA_HINT
  }
  return null
}
