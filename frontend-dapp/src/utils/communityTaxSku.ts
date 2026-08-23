/**
 * Community tax SKU catalog + invoice math (GitLab #593 / #592).
 * On-chain price is always 50 UST1 per SKU unlock or one settings batch.
 */

export const COMMUNITY_TAX_INVOICE_UST1_RAW = 50_000_000n
export const COMMUNITY_TAX_MAX_BPS = 2500
export const COMMUNITY_TAX_BPS_DENOM = 10_000

export type CommunityTaxSkuId =
  | 'mint_control'
  | 'transfer_tax'
  | 'split_router'
  | 'auto_v2_lp'
  | 'exemption_directory'
  | 'variable_rates'
  | 'launch_guards'

export type CommunityTaxSkuDef = {
  id: CommunityTaxSkuId
  /** Retail label — never the raw JSON key in a heading. */
  label: string
  hint: string
  /** MintControl cannot be purchased after instantiate. */
  createOnly: boolean
}

export const COMMUNITY_TAX_SKUS: readonly CommunityTaxSkuDef[] = [
  { id: 'mint_control', label: 'Minting', hint: 'Issue more supply later. Cannot add after create.', createOnly: true },
  {
    id: 'transfer_tax',
    label: 'Wallet-to-wallet tax',
    hint: 'Tax wallet transfers, not just buys and sells.',
    createOnly: false,
  },
  {
    id: 'split_router',
    label: 'Split treasury',
    hint: 'Split tax across treasury, burn, AutoLP, or wallets.',
    createOnly: false,
  },
  { id: 'auto_v2_lp', label: 'Auto liquidity', hint: 'Optional skim into a CL8Y pair. Not a farm.', createOnly: false },
  { id: 'exemption_directory', label: 'Extra exemptions', hint: 'Manager-chosen wallets skip tax.', createOnly: false },
  {
    id: 'variable_rates',
    label: 'Change rates later',
    hint: 'Required to change buy/sell after create, and to set max rates above current at create.',
    createOnly: false,
  },
  { id: 'launch_guards', label: 'Launch guards', hint: 'Max wallet, cooldown, trading on/off.', createOnly: false },
]

export function skuInvoiceUst1Raw(skuCount: number): bigint {
  if (!Number.isFinite(skuCount) || skuCount <= 0) return 0n
  return COMMUNITY_TAX_INVOICE_UST1_RAW * BigInt(Math.floor(skuCount))
}

export function skuInvoiceUst1RawString(skuCount: number): string {
  return skuInvoiceUst1Raw(skuCount).toString()
}

export function settingsBatchInvoiceUst1Raw(): string {
  return COMMUNITY_TAX_INVOICE_UST1_RAW.toString()
}

export function isUnlockableAfterCreate(id: CommunityTaxSkuId): boolean {
  return !COMMUNITY_TAX_SKUS.find((s) => s.id === id)?.createOnly
}

export function parseTaxBps(raw: string): { ok: true; bps: number } | { ok: false; error: string } {
  const trimmed = raw.trim()
  if (!trimmed) return { ok: true, bps: 0 }
  if (!/^\d+$/.test(trimmed)) return { ok: false, error: 'Enter a whole number of basis points' }
  const bps = Number(trimmed)
  if (bps > COMMUNITY_TAX_MAX_BPS) {
    return { ok: false, error: `Tax cannot exceed ${COMMUNITY_TAX_MAX_BPS} bps (25%)` }
  }
  return { ok: true, bps }
}

/**
 * Retail percent → on-chain bps (GitLab #605). Exactly 2 decimal places.
 * `2.50` → 250. Empty → 0. Reject `2.501`, `1e2`, trailing `%`, overflow.
 */
export function parsePercentToBps(
  raw: string,
  maxBps: number
): { ok: true; bps: number } | { ok: false; error: string } {
  const trimmed = raw.trim()
  if (!trimmed) return { ok: true, bps: 0 }
  if (!/^\d+(\.\d{1,2})?$/.test(trimmed)) {
    return { ok: false, error: 'Enter a percent with up to 2 decimal places' }
  }
  const [wholeRaw, fracRaw = ''] = trimmed.split('.')
  if (wholeRaw.length > 6) return { ok: false, error: 'Percent is too large' }
  const whole = Number(wholeRaw)
  const frac = Number((fracRaw + '00').slice(0, 2))
  const bps = whole * 100 + frac
  if (!Number.isSafeInteger(bps) || bps < 0) return { ok: false, error: 'Percent is invalid' }
  if (bps > maxBps) {
    return { ok: false, error: `Cannot exceed ${(maxBps / 100).toFixed(2)}%` }
  }
  return { ok: true, bps }
}

export function parseTaxPercent(raw: string): { ok: true; bps: number } | { ok: false; error: string } {
  return parsePercentToBps(raw, COMMUNITY_TAX_MAX_BPS)
}

/** Sink shares are percents of tax (sum 100.00% → 10000 bps). */
export function parseSharePercent(raw: string): { ok: true; bps: number } | { ok: false; error: string } {
  return parsePercentToBps(raw, COMMUNITY_TAX_BPS_DENOM)
}

export function formatBpsAsPercent(bps: number): string {
  const n = Math.max(0, Math.floor(bps))
  const whole = Math.floor(n / 100)
  const frac = (n % 100).toString().padStart(2, '0')
  return `${whole}.${frac}`
}

export const MAX_INITIAL_EXEMPT = 20
export const MAX_SINKS = 4

export type SinkKindId = 'treasury' | 'burn' | 'auto_lp' | 'wallet'

export type SinkDraft = {
  kind: SinkKindId
  addr: string
  percent: string
}

export function sinkKindToHook(kind: SinkKindId): string {
  if (kind === 'auto_lp') return 'auto_lp'
  if (kind === 'wallet') return 'wallet'
  if (kind === 'burn') return 'burn'
  return 'treasury'
}

/** Immutable instantiate caps. Combined max_buy+max_sell+max_transfer must be ≤ 2500. */
export function instantiateTaxCaps(input: {
  buyBps: number
  sellBps: number
  transferBps?: number
  variableRates: boolean
  transferTax: boolean
  /** When variable-rates SKU is on, caller-chosen ceilings (#605). */
  maxBuyBps?: number
  maxSellBps?: number
  maxTransferBps?: number
}): { maxBuyBps: number; maxSellBps: number; maxTransferBps: number } {
  const buy = Math.max(0, input.buyBps)
  const sell = Math.max(0, input.sellBps)
  const transfer = Math.max(0, input.transferBps ?? 0)
  if (!input.variableRates) {
    return { maxBuyBps: buy, maxSellBps: sell, maxTransferBps: transfer }
  }
  if (input.maxBuyBps != null && input.maxSellBps != null && input.maxTransferBps != null) {
    return {
      maxBuyBps: input.maxBuyBps,
      maxSellBps: input.maxSellBps,
      maxTransferBps: input.maxTransferBps,
    }
  }
  let maxBuy = buy
  let maxSell = sell
  let maxTransfer = transfer
  let slack = COMMUNITY_TAX_MAX_BPS - buy - sell - transfer
  if (slack < 0) slack = 0
  const buckets = input.transferTax ? 3 : 2
  const share = Math.floor(slack / buckets)
  maxBuy += share
  maxSell += share
  if (input.transferTax) maxTransfer += share
  const leftover = slack - share * buckets
  maxBuy += leftover
  return { maxBuyBps: maxBuy, maxSellBps: maxSell, maxTransferBps: maxTransfer }
}
