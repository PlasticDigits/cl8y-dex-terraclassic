/**
 * Invoice + hook builders for community tax create / manage (GitLab #593 / #606).
 * Payee is always env launcher or the token contract — never a URL param.
 * Enable Feature stays on the launcher (**C593-4** / **T606-1**).
 */

import type { Invoice } from '@/utils/payInvoice'
import {
  COMMUNITY_TAX_INVOICE_UST1_RAW,
  instantiateTaxCaps,
  skuInvoiceUst1RawString,
  type CommunityTaxSkuId,
} from '@/utils/communityTaxSku'
import { toRawAmount } from '@/utils/formatAmount'

export type CreateTokenHookArgs = {
  name: string
  symbol: string
  decimals: number
  initialBalances: { address: string; amount: string }[]
  manager: string
  treasury: string
  buyBps: number
  sellBps: number
  maxBuyBps?: number
  maxSellBps?: number
  maxTransferBps?: number
  features: CommunityTaxSkuId[]
  mint?: { minter: string; cap?: string }
  transferBps?: number
  sinks?: { kind: string; addr?: string; bps: number }[]
  launchGuards?: { max_wallet?: string; cooldown_blocks: number; trading_enabled: boolean }
  initialExempt?: string[]
  autolpThreshold?: string
  autolpLpRecipient?: string
}

export function encodeInvoiceHook(inner: Record<string, unknown>): string {
  return btoa(JSON.stringify(inner))
}

/** First-seen SKU order. Launcher rejects duplicates (**T606-5**); do not double-charge. */
export function uniqueCommunityTaxSkus(skus: CommunityTaxSkuId[]): CommunityTaxSkuId[] {
  return [...new Set(skus)]
}

export function buildCreateTokenHook(args: CreateTokenHookArgs): string {
  const features = uniqueCommunityTaxSkus(args.features)
  const caps = instantiateTaxCaps({
    buyBps: args.buyBps,
    sellBps: args.sellBps,
    transferBps: args.transferBps,
    variableRates: features.includes('variable_rates'),
    transferTax: features.includes('transfer_tax'),
    maxBuyBps: args.maxBuyBps,
    maxSellBps: args.maxSellBps,
    maxTransferBps: args.maxTransferBps,
  })
  const create_token: Record<string, unknown> = {
    name: args.name,
    symbol: args.symbol,
    decimals: args.decimals,
    initial_balances: args.initialBalances,
    manager: args.manager,
    treasury: args.treasury,
    buy_bps: args.buyBps,
    sell_bps: args.sellBps,
    max_buy_bps: args.maxBuyBps ?? caps.maxBuyBps,
    max_sell_bps: args.maxSellBps ?? caps.maxSellBps,
    max_transfer_bps: args.maxTransferBps ?? caps.maxTransferBps,
    features,
  }
  if (args.mint && features.includes('mint_control')) create_token.mint = args.mint
  if (args.transferBps != null && features.includes('transfer_tax')) {
    create_token.transfer_bps = args.transferBps
  }
  if (args.sinks && features.includes('split_router')) create_token.sinks = args.sinks
  if (args.launchGuards && features.includes('launch_guards')) {
    create_token.launch_guards = args.launchGuards
  }
  if (args.initialExempt && features.includes('exemption_directory')) {
    create_token.initial_exempt = args.initialExempt
  }
  if (features.includes('auto_v2_lp')) {
    if (args.autolpThreshold != null) create_token.autolp_threshold = args.autolpThreshold
    if (args.autolpLpRecipient) create_token.autolp_lp_recipient = args.autolpLpRecipient
  }
  return encodeInvoiceHook({ create_token })
}

export function buildCreateTokenInvoice(input: { launcher: string; ust1: string; args: CreateTokenHookArgs }): Invoice {
  return {
    invoiceToken: input.ust1,
    invoiceAmount: skuInvoiceUst1RawString(uniqueCommunityTaxSkus(input.args.features).length),
    payee: input.launcher,
    hookMsg: buildCreateTokenHook(input.args),
  }
}

export function buildEnableFeatureInvoice(input: {
  launcher: string
  ust1: string
  token: string
  sku: CommunityTaxSkuId
}): Invoice {
  return {
    invoiceToken: input.ust1,
    invoiceAmount: COMMUNITY_TAX_INVOICE_UST1_RAW.toString(),
    payee: input.launcher,
    hookMsg: encodeInvoiceHook({ enable_feature: { token: input.token, sku: input.sku } }),
  }
}

export type SettingsBatchFields = {
  buy_bps?: number
  sell_bps?: number
  treasury?: string
  transfer_bps?: number
  sinks?: { kind: string; addr?: string; bps: number }[]
  add_exempt?: string[]
  remove_exempt?: string[]
  autolp?: { pair?: string; threshold: string; lp_recipient: string }
  launch_guards?: { max_wallet?: string; cooldown_blocks: number; trading_enabled: boolean }
  minter?: string
  revoke_mint?: boolean
}

export type AutoLpSisterConfig = {
  pair: string | null
  threshold: string
  lp_recipient: string
}

export type AutoLpDraft = {
  pair: string
  thresholdHuman: string
  lpRecipient: string
}

function sameTerraAddr(a: string, b: string | null | undefined): boolean {
  if (!b) return false
  return a.trim().toLowerCase() === b.trim().toLowerCase()
}

/**
 * Manage Token AutoLP dirty compare vs sister `GetConfig` (**T592-4** / #1237).
 * Blank fields are omitted (not defaulted to raw `1` or the connected wallet).
 * Returns `undefined` when nothing would change on the sister.
 */
export function buildAutolpSettingsDelta(
  draft: AutoLpDraft,
  sister: AutoLpSisterConfig,
  decimals: number
): SettingsBatchFields['autolp'] | undefined {
  const pairTrim = draft.pair.trim()
  const thTrim = draft.thresholdHuman.trim()
  const recipTrim = draft.lpRecipient.trim()
  const pairDelta = pairTrim.length > 0 && !sameTerraAddr(pairTrim, sister.pair)
  const thresholdRaw = thTrim ? toRawAmount(thTrim, decimals) : null
  const thresholdDelta = thresholdRaw != null && thresholdRaw !== String(sister.threshold)
  const recipientDelta = recipTrim.length > 0 && !sameTerraAddr(recipTrim, sister.lp_recipient)
  if (!pairDelta && !thresholdDelta && !recipientDelta) return undefined
  return {
    pair: pairDelta ? pairTrim : undefined,
    threshold: thresholdDelta && thresholdRaw != null ? thresholdRaw : sister.threshold,
    lp_recipient: recipientDelta ? recipTrim : sister.lp_recipient,
  }
}

export function settingsBatchIsEmpty(batch: SettingsBatchFields): boolean {
  return (
    batch.buy_bps == null &&
    batch.sell_bps == null &&
    batch.treasury == null &&
    batch.transfer_bps == null &&
    batch.sinks == null &&
    batch.add_exempt == null &&
    batch.remove_exempt == null &&
    batch.autolp == null &&
    batch.launch_guards == null &&
    batch.minter == null &&
    batch.revoke_mint == null
  )
}

export function buildSettingsBatchInvoice(input: {
  token: string
  ust1: string
  settings: SettingsBatchFields
}): Invoice {
  return {
    invoiceToken: input.ust1,
    invoiceAmount: COMMUNITY_TAX_INVOICE_UST1_RAW.toString(),
    payee: input.token,
    hookMsg: encodeInvoiceHook({ update_settings: { settings: input.settings } }),
  }
}
