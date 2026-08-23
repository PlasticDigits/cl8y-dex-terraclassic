/**
 * Shared Create Token form → hook args (GitLab #604 / #605).
 * Paid and free create both go through `buildValidatedCreateArgs`.
 */

import { toRawAmount } from '@/utils/formatAmount'
import { getTerraAddressInputError, isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import { parseTokenDecimals, parseTokenName, parseTokenSymbol } from '@/utils/communityTaxIdentity'
import {
  COMMUNITY_TAX_MAX_BPS,
  MAX_INITIAL_EXEMPT,
  MAX_SINKS,
  parseSharePercent,
  parseTaxPercent,
  sinkKindToHook,
  type CommunityTaxSkuId,
  type SinkDraft,
} from '@/utils/communityTaxSku'
import type { CreateTokenHookArgs } from '@/utils/communityTaxInvoice'

export type CreateTokenFormDraft = {
  name: string
  symbol: string
  decimals: string
  buyPercent: string
  sellPercent: string
  transferPercent: string
  treasury: string
  manager: string
  skus: CommunityTaxSkuId[]
  mintCapHuman: string
  sinks: SinkDraft[]
  exemptList: string
  maxBuyPercent: string
  maxSellPercent: string
  maxTransferPercent: string
  autolpThresholdHuman: string
  autolpRecipient: string
  maxWalletHuman: string
  cooldownBlocks: string
  tradingEnabled: boolean
  autolpCodeId: number | null
}

function parseHumanRaw(
  raw: string,
  decimals: number,
  label: string
): { ok: true; raw: string } | { ok: false; error: string } {
  const trimmed = raw.trim()
  if (!trimmed) return { ok: true, raw: '' }
  if (!/^\d+(\.\d+)?$/.test(trimmed)) return { ok: false, error: `${label} must be a human amount` }
  try {
    return { ok: true, raw: toRawAmount(trimmed, decimals) }
  } catch {
    return { ok: false, error: `${label} is invalid` }
  }
}

function parseExemptList(raw: string): { ok: true; addrs: string[] } | { ok: false; error: string } {
  const addrs = raw
    .split(/[\s,]+/)
    .map((s) => s.trim())
    .filter(Boolean)
  if (addrs.length > MAX_INITIAL_EXEMPT) {
    return { ok: false, error: `At most ${MAX_INITIAL_EXEMPT} extra exemptions` }
  }
  for (const a of addrs) {
    if (!isValidTerraBech32Address(a)) return { ok: false, error: 'Exemption list has an invalid address' }
  }
  return { ok: true, addrs }
}

export function buildValidatedCreateArgs(
  draft: CreateTokenFormDraft
): { ok: true; args: CreateTokenHookArgs } | { ok: false; errors: Record<string, string> } {
  const errors: Record<string, string> = {}
  const name = parseTokenName(draft.name)
  const symbol = parseTokenSymbol(draft.symbol)
  const decimals = parseTokenDecimals(draft.decimals)
  if (!name.ok) errors.name = name.error
  if (!symbol.ok) errors.symbol = symbol.error
  if (!decimals.ok) errors.decimals = decimals.error

  const buy = parseTaxPercent(draft.buyPercent)
  const sell = parseTaxPercent(draft.sellPercent)
  if (!buy.ok) errors.buy = buy.error
  if (!sell.ok) errors.sell = sell.error

  const transferOn = draft.skus.includes('transfer_tax')
  const transfer = parseTaxPercent(draft.transferPercent)
  if (transferOn && !transfer.ok) errors.transfer = transfer.error

  const treasuryErr = draft.treasury.trim() ? getTerraAddressInputError(draft.treasury.trim()) : 'Treasury required'
  const managerErr = draft.manager.trim() ? getTerraAddressInputError(draft.manager.trim()) : 'Manager required'
  if (treasuryErr) errors.treasury = treasuryErr
  if (managerErr) errors.manager = managerErr

  const buyBps = buy.ok ? buy.bps : 0
  const sellBps = sell.ok ? sell.bps : 0
  const transferBps = transferOn && transfer.ok ? transfer.bps : 0
  if (buyBps + sellBps + transferBps > COMMUNITY_TAX_MAX_BPS) {
    errors.combined = 'Combined buy+sell+wallet tax cannot exceed 25.00%'
  }

  let mint: CreateTokenHookArgs['mint']
  if (draft.skus.includes('mint_control') && decimals.ok) {
    const cap = parseHumanRaw(draft.mintCapHuman, decimals.value, 'Mint cap')
    if (!cap.ok) errors.mintCap = cap.error
    else {
      mint = {
        minter: draft.manager.trim(),
        cap: cap.raw || undefined,
      }
    }
  }

  let sinks: CreateTokenHookArgs['sinks']
  if (draft.skus.includes('split_router')) {
    if (draft.sinks.length < 1 || draft.sinks.length > MAX_SINKS) {
      errors.sinks = `Split treasury needs 1–${MAX_SINKS} rows`
    } else {
      const parsed: { kind: string; addr?: string; bps: number }[] = []
      let sum = 0
      draft.sinks.forEach((row, i) => {
        const share = parseSharePercent(row.percent)
        if (!share.ok) {
          errors.sinks = share.error
          return
        }
        sum += share.bps
        const hook: { kind: string; addr?: string; bps: number } = {
          kind: sinkKindToHook(row.kind),
          bps: share.bps,
        }
        if (row.kind === 'wallet') {
          if (!row.addr.trim() || getTerraAddressInputError(row.addr.trim())) {
            errors.sinks = `Sink ${i + 1} needs a valid wallet address`
          } else {
            hook.addr = row.addr.trim()
          }
        }
        parsed.push(hook)
      })
      if (!errors.sinks && sum !== 10_000) {
        errors.sinks = 'Sink percents must sum to 100.00%'
      }
      if (!errors.sinks) sinks = parsed
    }
  }

  let initialExempt: string[] | undefined
  if (draft.skus.includes('exemption_directory')) {
    const list = parseExemptList(draft.exemptList)
    if (!list.ok) errors.exempt = list.error
    else initialExempt = list.addrs
  }

  let maxBuyBps: number | undefined
  let maxSellBps: number | undefined
  let maxTransferBps: number | undefined
  if (draft.skus.includes('variable_rates')) {
    const maxBuy = parseTaxPercent(draft.maxBuyPercent)
    const maxSell = parseTaxPercent(draft.maxSellPercent)
    const maxTransfer = parseTaxPercent(draft.maxTransferPercent)
    if (!maxBuy.ok) errors.maxBuy = maxBuy.error
    if (!maxSell.ok) errors.maxSell = maxSell.error
    if (!maxTransfer.ok) errors.maxTransfer = maxTransfer.error
    if (maxBuy.ok && maxBuy.bps < buyBps) errors.maxBuy = 'Max buy must be at least the current buy tax'
    if (maxSell.ok && maxSell.bps < sellBps) errors.maxSell = 'Max sell must be at least the current sell tax'
    if (maxTransfer.ok && maxTransfer.bps < transferBps) {
      errors.maxTransfer = 'Max wallet-to-wallet must be at least the current rate'
    }
    if (maxBuy.ok && maxSell.ok && maxTransfer.ok) {
      if (maxBuy.bps + maxSell.bps + maxTransfer.bps > COMMUNITY_TAX_MAX_BPS) {
        errors.maxCombined = 'Combined max taxes cannot exceed 25.00%'
      } else {
        maxBuyBps = maxBuy.bps
        maxSellBps = maxSell.bps
        maxTransferBps = maxTransfer.bps
      }
    }
  }

  let launchGuards: CreateTokenHookArgs['launchGuards']
  if (draft.skus.includes('launch_guards') && decimals.ok) {
    const cooldown = draft.cooldownBlocks.trim()
    if (cooldown && !/^\d+$/.test(cooldown)) {
      errors.cooldown = 'Cooldown must be a whole number of blocks'
    }
    const wallet = parseHumanRaw(draft.maxWalletHuman, decimals.value, 'Max wallet')
    if (!wallet.ok) errors.maxWallet = wallet.error
    if (!errors.cooldown && !errors.maxWallet) {
      launchGuards = {
        max_wallet: wallet.ok && wallet.raw ? wallet.raw : undefined,
        cooldown_blocks: cooldown ? Number(cooldown) : 0,
        trading_enabled: draft.tradingEnabled,
      }
    }
  }

  let autolpThreshold: string | undefined
  let autolpLpRecipient: string | undefined
  if (draft.skus.includes('auto_v2_lp')) {
    if (!draft.autolpCodeId) {
      errors.autolp = 'Auto liquidity is not configured on this launcher. Unlock it later on Manage.'
    } else if (decimals.ok) {
      const th = parseHumanRaw(draft.autolpThresholdHuman, decimals.value, 'AutoLP threshold')
      if (!th.ok) errors.autolp = th.error
      const recip = draft.autolpRecipient.trim()
      const recipErr = recip ? getTerraAddressInputError(recip) : 'LP recipient required'
      if (recipErr) errors.autolpRecipient = recipErr
      if (!errors.autolp && !errors.autolpRecipient) {
        autolpThreshold = th.ok && th.raw ? th.raw : '1'
        autolpLpRecipient = recip
      }
    }
  }

  if (Object.keys(errors).length > 0) return { ok: false, errors }
  if (!name.ok || !symbol.ok || !decimals.ok || !buy.ok || !sell.ok) return { ok: false, errors }

  return {
    ok: true,
    args: {
      name: name.value,
      symbol: symbol.value,
      decimals: decimals.value,
      initialBalances: [],
      manager: draft.manager.trim(),
      treasury: draft.treasury.trim(),
      buyBps,
      sellBps,
      maxBuyBps,
      maxSellBps,
      maxTransferBps,
      features: draft.skus,
      mint,
      transferBps: transferOn ? transferBps : undefined,
      sinks,
      launchGuards,
      initialExempt,
      autolpThreshold,
      autolpLpRecipient,
    },
  }
}
