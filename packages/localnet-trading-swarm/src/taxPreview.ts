/** TaxPreview debit math + fail-closed fallback (GitLab #621 / T592-2 / T592-13). */

import { DEFAULT_SELL_BPS, TAX_BPS_DENOM } from './taxDetect.js'

export type TaxPath = 'pair' | 'router'

export interface TaxPreviewView {
  kind?: string
  declared: string
  debit: string
  credit: string
  tax: string
  hop_trader?: string | null
  hop_trader_debit?: string | null
}

export function parseUint(raw: string | undefined | null): bigint {
  if (!raw || !/^\d+$/.test(raw)) return 0n
  try {
    return BigInt(raw)
  } catch {
    return 0n
  }
}

/** `amount + floor(amount * sellBps / 10000)` — fail-closed when preview is missing. */
export function failClosedSellDebit(amount: bigint, sellBps: number = DEFAULT_SELL_BPS): bigint {
  if (amount <= 0n) return 0n
  const bps = Math.max(0, Math.floor(sellBps))
  return amount + (amount * BigInt(bps)) / BigInt(TAX_BPS_DENOM)
}

/**
 * Wallet tokens that must be available before broadcasting a tax-token sell.
 *
 * Pair-direct: `TaxPreview.debit` (declared + extra-debit from `from`).
 * Router: user `Send` is 1:1 (`debit` == declared when previewing the hop from
 * the router) plus `hop_trader_debit`. Missing hop extra-debit falls back to
 * fail-closed sell tax so we never size `Send.amount` as 100% of balance.
 */
export function requiredWalletDebit(
  preview: TaxPreviewView | null | undefined,
  amount: bigint,
  path: TaxPath,
  sellBps: number = DEFAULT_SELL_BPS
): bigint {
  if (!preview) return failClosedSellDebit(amount, sellBps)
  const debit = parseUint(preview.debit)
  const hop = parseUint(preview.hop_trader_debit ?? undefined)
  if (path === 'router') {
    const extra = hop > 0n ? hop : failClosedSellDebit(amount, sellBps) - amount
    return debit + extra
  }
  return debit > 0n ? debit : failClosedSellDebit(amount, sellBps)
}

export function balanceCoversDebit(balance: bigint, required: bigint): boolean {
  return balance >= required && required > 0n
}

export interface TaxLogFields {
  tax_debit: string
  tax_credit: string
  bps: number
  path: TaxPath
}

export function taxLogFields(
  preview: TaxPreviewView | null | undefined,
  amount: bigint,
  path: TaxPath,
  sellBps: number = DEFAULT_SELL_BPS
): TaxLogFields {
  const required = requiredWalletDebit(preview, amount, path, sellBps)
  const credit = preview ? parseUint(preview.credit) : amount
  return {
    tax_debit: required.toString(),
    tax_credit: credit.toString(),
    bps: sellBps,
    path,
  }
}
