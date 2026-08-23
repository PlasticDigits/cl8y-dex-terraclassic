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
  {
    id: 'exemption_directory',
    label: 'Extra exemptions',
    hint: 'Manager-chosen wallets skip buy, sell, and transfer tax.',
    createOnly: false,
  },
  {
    id: 'variable_rates',
    label: 'Change rates later',
    hint: 'Adjust buy/sell after launch (still capped).',
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

/** Immutable instantiate caps. Combined max_buy+max_sell+max_transfer must be ≤ 2500. */
export function instantiateTaxCaps(input: {
  buyBps: number
  sellBps: number
  transferBps?: number
  variableRates: boolean
  transferTax: boolean
}): { maxBuyBps: number; maxSellBps: number; maxTransferBps: number } {
  const buy = Math.max(0, input.buyBps)
  const sell = Math.max(0, input.sellBps)
  const transfer = Math.max(0, input.transferBps ?? 0)
  let maxBuy = buy
  let maxSell = sell
  let maxTransfer = transfer
  if (input.variableRates) {
    let slack = COMMUNITY_TAX_MAX_BPS - buy - sell - transfer
    if (slack < 0) slack = 0
    const buckets = input.transferTax ? 3 : 2
    const share = Math.floor(slack / buckets)
    maxBuy += share
    maxSell += share
    if (input.transferTax) maxTransfer += share
    const leftover = slack - share * buckets
    maxBuy += leftover
  }
  return { maxBuyBps: maxBuy, maxSellBps: maxSell, maxTransferBps: maxTransfer }
}
