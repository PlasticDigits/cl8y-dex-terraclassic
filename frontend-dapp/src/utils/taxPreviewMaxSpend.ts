/**
 * Extra-debit sell max (GitLab #593 / #592 T592-2).
 * Sell to a listed pair debits `declared + tax`. Max must leave room for the extra debit
 * so Swap/Trade cannot offer 100% of balance (self-DoS / failed tx spam).
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
