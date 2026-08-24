/**
 * LCD queries + unpaid executes for community tax tokens (GitLab #593).
 * Paid SKU / settings paths go through PayWithAnyToken — do not assemble router ops here.
 */

import { MsgMigrateContract, MsgUpdateAdmin } from '@goblinhunt/cosmes/client'
import { executeTerraContract } from './transactions'
import { getChainContractInfo, queryContract } from './queries'
import { broadcastTerraClassicMsgs } from './terraBroadcast'
import { getConnectedWallet } from './wallet'
import {
  CMM_GOVERNANCE_ADDR,
  COMMUNITY_MIGRATE_ADOPT_GAS_LIMIT,
  COMMUNITY_TAX_CODE_ID,
  COMMUNITY_TOKEN_LAUNCHER,
  isCommunityTaxEnabled,
} from '@/utils/constants'
import { buildAdoptMigrateMsg } from '@/utils/communityTaxMigrate'
import { getTerraBroadcastScopeOptions } from './terraBroadcastScope'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import type { CreateTokenHookArgs } from '@/utils/communityTaxInvoice'
import { instantiateTaxCaps } from '@/utils/communityTaxSku'

export type CommunityTaxConfigResponse = {
  manager: string
  treasury: string
  buy_bps: number
  sell_bps: number
  transfer_bps: number
  max_buy_bps: number
  max_sell_bps: number
  max_transfer_bps: number
  factory: string
  router: string | null
  ust1: string
  cmm_treasury: string
  autolp: string | null
  sinks: { kind: string; addr?: string | null; bps: number }[]
  launch_guards: { max_wallet?: string | null; cooldown_blocks: number; trading_enabled: boolean } | null
  mint_revoked: boolean
}

export type CommunityTaxFeaturesResponse = {
  mint_control: boolean
  transfer_tax: boolean
  split_router: boolean
  auto_v2_lp: boolean
  exemption_directory: boolean
  variable_rates: boolean
  launch_guards: boolean
}

export type TaxPreviewResponse = {
  kind: string
  declared: string
  debit: string
  credit: string
  tax: string
  hop_trader?: string | null
  hop_trader_debit?: string | null
}

export type LauncherOriginResponse = {
  launcher: string | null
}

function requireLauncher(): string {
  if (!COMMUNITY_TOKEN_LAUNCHER) throw new Error('Community token launcher is not configured')
  return COMMUNITY_TOKEN_LAUNCHER
}

export function requireCommunityTaxTokenAddr(addr: string): string {
  const trimmed = addr.trim()
  if (!isValidTerraBech32Address(trimmed)) throw new Error('Invalid token address')
  return trimmed
}

export async function assertCommunityTaxTemplate(addr: string): Promise<{ code_id: number; admin: string }> {
  const info = await getChainContractInfo(addr)
  if (COMMUNITY_TAX_CODE_ID && info.code_id !== COMMUNITY_TAX_CODE_ID) {
    throw new Error('This contract is not the community tax template')
  }
  return info
}

export async function queryCommunityTaxConfig(addr: string): Promise<CommunityTaxConfigResponse> {
  return queryContract<CommunityTaxConfigResponse>(requireCommunityTaxTokenAddr(addr), { get_config: {} })
}

export async function queryCommunityTaxFeatures(addr: string): Promise<CommunityTaxFeaturesResponse> {
  return queryContract<CommunityTaxFeaturesResponse>(requireCommunityTaxTokenAddr(addr), { get_features: {} })
}

export async function queryCommunityTaxTokenInfo(
  addr: string
): Promise<{ name: string; symbol: string; decimals: number }> {
  return queryContract(requireCommunityTaxTokenAddr(addr), { token_info: {} })
}

export async function queryCommunityTaxExemptions(addr: string): Promise<{
  protocol: string[]
  manager: string[]
}> {
  return queryContract(requireCommunityTaxTokenAddr(addr), { get_exemptions: {} })
}

export type CommunityTaxIsExemptResponse = {
  address: string
  protocol: boolean
  manager: boolean
}

/** `IsProtocolExempt` also returns the manager-directory flag (#609). */
export async function queryCommunityTaxIsExempt(token: string, address: string): Promise<CommunityTaxIsExemptResponse> {
  return queryContract<CommunityTaxIsExemptResponse>(requireCommunityTaxTokenAddr(token), {
    is_protocol_exempt: { address },
  })
}

export async function queryLauncherOrigin(addr: string): Promise<LauncherOriginResponse> {
  return queryContract<LauncherOriginResponse>(requireCommunityTaxTokenAddr(addr), {
    get_launcher_origin: {},
  })
}

export async function queryTaxPreview(input: {
  token: string
  from: string
  to: string
  amount: string
  sendMsg?: string
}): Promise<TaxPreviewResponse> {
  const q: Record<string, unknown> = {
    tax_preview: {
      from: input.from,
      to: input.to,
      amount: input.amount,
      send_msg: input.sendMsg ?? null,
    },
  }
  return queryContract<TaxPreviewResponse>(requireCommunityTaxTokenAddr(input.token), q)
}

/** Free create (0 SKUs). Paid create uses PayWithAnyToken → launcher Receive. */
export function buildFreeCreateTokenMsg(args: CreateTokenHookArgs): Record<string, unknown> {
  if (args.features.length > 0) {
    throw new Error('Paid SKUs must settle via the invoice card')
  }
  const caps = instantiateTaxCaps({
    buyBps: args.buyBps,
    sellBps: args.sellBps,
    transferBps: undefined,
    variableRates: false,
    transferTax: false,
  })
  return {
    create_token: {
      name: args.name,
      symbol: args.symbol,
      decimals: args.decimals,
      initial_balances: args.initialBalances,
      manager: args.manager,
      treasury: args.treasury,
      buy_bps: args.buyBps,
      sell_bps: args.sellBps,
      max_buy_bps: caps.maxBuyBps,
      max_sell_bps: caps.maxSellBps,
      max_transfer_bps: caps.maxTransferBps,
      features: [],
    },
  }
}

export type LauncherConfigResponse = {
  token_code_id: number
  autolp_code_id: number | null
  ust1: string
  cmm_treasury: string
  cmm_governance: string
  factory: string
  router: string | null
}

export async function queryLauncherConfig(): Promise<LauncherConfigResponse> {
  return queryContract<LauncherConfigResponse>(requireLauncher(), { get_config: {} })
}

export async function createFreeCommunityToken(walletAddress: string, args: CreateTokenHookArgs): Promise<string> {
  if (!isCommunityTaxEnabled()) throw new Error('Create Token is not configured')
  return executeTerraContract(walletAddress, requireLauncher(), buildFreeCreateTokenMsg(args))
}

export async function mintCommunityTax(
  walletAddress: string,
  token: string,
  recipient: string,
  amount: string
): Promise<string> {
  await assertCommunityTaxTemplate(token)
  return executeTerraContract(walletAddress, token, { mint: { recipient, amount } })
}

export async function skimAutoLp(walletAddress: string, autolp: string): Promise<string> {
  if (!isValidTerraBech32Address(autolp)) throw new Error('Invalid AutoLP address')
  return executeTerraContract(walletAddress, autolp, { skim_to_lp: {} })
}

export async function registerListedPair(walletAddress: string, token: string, pair: string): Promise<string> {
  await assertCommunityTaxTemplate(token)
  return executeTerraContract(walletAddress, token, { register_listed_pair: { pair } })
}

export async function probeHasTaxMap(addr: string): Promise<boolean> {
  try {
    await queryContract(requireCommunityTaxTokenAddr(addr), { tax_map: {} })
    return true
  } catch {
    return false
  }
}

export type AdoptBundleInput = {
  token: string
  wasmAdmin: string
  factory: string
  router: string | null
  ust1: string
  cmmTreasury: string
  sourceCodeId: number
}

/** One-click adopt: `MsgMigrateContract` then `MsgUpdateAdmin` → CMM. No UST1 invoice. */
export async function migrateAdoptCommunityToken(input: AdoptBundleInput): Promise<string> {
  if (!isCommunityTaxEnabled()) throw new Error('Migrate Token is not configured')
  if (COMMUNITY_TAX_CODE_ID <= 0) throw new Error('Community tax code id is not configured')
  const token = requireCommunityTaxTokenAddr(input.token)
  const wallet = getConnectedWallet()
  if (!wallet) throw new Error('Wallet not connected. Please connect your wallet first.')
  if (wallet.address !== input.wasmAdmin) {
    throw new Error('Wallet address mismatch')
  }
  const migrateMsg = buildAdoptMigrateMsg({
    manager: input.wasmAdmin,
    treasury: input.cmmTreasury,
    factory: input.factory,
    router: input.router,
    ust1: input.ust1,
    cmmTreasury: input.cmmTreasury,
    officialLauncher: requireLauncher(),
    sourceCodeId: input.sourceCodeId,
    tokenAddr: token,
  })
  const msgs = [
    new MsgMigrateContract({
      sender: input.wasmAdmin,
      contract: token,
      codeId: BigInt(COMMUNITY_TAX_CODE_ID),
      msg: migrateMsg,
    }),
    new MsgUpdateAdmin({
      sender: input.wasmAdmin,
      newAdmin: CMM_GOVERNANCE_ADDR,
      contract: token,
    }),
  ]
  return broadcastTerraClassicMsgs(wallet, msgs, COMMUNITY_MIGRATE_ADOPT_GAS_LIMIT, getTerraBroadcastScopeOptions())
}
