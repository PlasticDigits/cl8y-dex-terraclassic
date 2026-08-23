/**
 * Invoice + hook builders for community tax create / manage (GitLab #593).
 * Payee is always env launcher or the token contract — never a URL param.
 */

import type { Invoice } from '@/utils/payInvoice'
import {
  COMMUNITY_TAX_INVOICE_UST1_RAW,
  instantiateTaxCaps,
  skuInvoiceUst1RawString,
  type CommunityTaxSkuId,
} from '@/utils/communityTaxSku'

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
}

export function encodeInvoiceHook(inner: Record<string, unknown>): string {
  return btoa(JSON.stringify(inner))
}

export function buildCreateTokenHook(args: CreateTokenHookArgs): string {
  const caps = instantiateTaxCaps({
    buyBps: args.buyBps,
    sellBps: args.sellBps,
    transferBps: args.transferBps,
    variableRates: args.features.includes('variable_rates'),
    transferTax: args.features.includes('transfer_tax'),
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
    features: args.features,
  }
  if (args.mint) create_token.mint = args.mint
  if (args.transferBps != null) create_token.transfer_bps = args.transferBps
  return encodeInvoiceHook({ create_token })
}

export function buildCreateTokenInvoice(input: { launcher: string; ust1: string; args: CreateTokenHookArgs }): Invoice {
  return {
    invoiceToken: input.ust1,
    invoiceAmount: skuInvoiceUst1RawString(input.args.features.length),
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
