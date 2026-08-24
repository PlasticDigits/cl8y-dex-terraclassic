/**
 * Free listed-template adopt onto the community tax wasm (GitLab #626).
 * No invoice. Query params never prefill payee / admin / treasury.
 */

export const COLUMBUS5_ADOPT_CODE_IDS = [6036, 10184, 8266] as const
export const COLUMBUS5_ALPHA_CODE_ID = 8654
export const COLUMBUS5_TAX_CODE_IDS = [11611, 11619] as const
export const ALPHA_COLUMBUS5_ADDR = 'terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz'
/** ALPHA live map: 4.5% buy / 1% sell (combined 550 ≤ 2500). */
export const ALPHA_BUY_BPS = 450
export const ALPHA_SELL_BPS = 100

export const ALLOWED_ADOPT_CW2 = [
  'crates.io:cw20-base',
  'crates.io:cw20-mintable',
  'crates.io:terraport-token',
  'crates.io:cw20-taxed',
  'cw20_taxed',
] as const

export type MigrateVerdictKind =
  | 'go'
  | 'go_alpha'
  | 'already_tax'
  | 'unlisted'
  | 'tax_map'
  | 'not_admin'
  | 'unavailable'
  | 'bad_addr'

export type MigrateVerdict = {
  kind: MigrateVerdictKind
  reason: string
  canSubmit: boolean
}

export function isColumbus5(chainId: string): boolean {
  return chainId === 'columbus-5'
}

export function isAlphaSource(codeId: number, tokenAddr: string): boolean {
  return codeId === COLUMBUS5_ALPHA_CODE_ID || tokenAddr.trim().toLowerCase() === ALPHA_COLUMBUS5_ADDR
}

export function classifyMigrateSource(input: {
  chainId: string
  codeId: number
  taxCodeId: number
  whitelisted: boolean
  hasTaxMap: boolean
  wasmAdmin: string
  connectedWallet: string | null
  tokenAddr: string
}): MigrateVerdict {
  const addr = input.tokenAddr.trim().toLowerCase()
  const alpha = isAlphaSource(input.codeId, addr)
  if (input.hasTaxMap && !alpha) {
    return {
      kind: 'tax_map',
      reason:
        'This template has inbound tax_map and is not ALPHA. In-place adopt is not supported. Create a new 11619 token or use the wrap path on #558.',
      canSubmit: false,
    }
  }
  if (input.codeId === input.taxCodeId || COLUMBUS5_TAX_CODE_IDS.includes(input.codeId as 11611 | 11619)) {
    return {
      kind: 'already_tax',
      reason: 'This contract is already the community tax wasm. Same-crate upgrades are CMM-only — not this page.',
      canSubmit: false,
    }
  }
  if (!alpha && !input.whitelisted) {
    return {
      kind: 'unlisted',
      reason: 'Source code id is not on the factory whitelist. This page will not whitelist it.',
      canSubmit: false,
    }
  }
  if (!alpha && isColumbus5(input.chainId) && !COLUMBUS5_ADOPT_CODE_IDS.includes(input.codeId as 6036 | 10184 | 8266)) {
    return {
      kind: 'unlisted',
      reason: 'Only factory-listed 6036, 10184, 8266, or ALPHA 8654 can adopt on columbus-5.',
      canSubmit: false,
    }
  }
  if (!input.connectedWallet || input.connectedWallet !== input.wasmAdmin) {
    return {
      kind: 'not_admin',
      reason: 'Connect the current wasm admin. Holders cannot migrate someone else’s token.',
      canSubmit: false,
    }
  }
  if (alpha) {
    return {
      kind: 'go_alpha',
      reason:
        'ALPHA 8654. One click wipes tax_info / tax_map, keeps this address, and lands on 11619. Do not whitelist 8654. No 50 UST1.',
      canSubmit: true,
    }
  }
  return {
    kind: 'go',
    reason: 'Listed honest template. One click migrates this address onto the tax wasm. No 50 UST1.',
    canSubmit: true,
  }
}

export function buildAdoptMigrateMsg(input: {
  manager: string
  treasury: string
  factory: string
  router: string | null
  ust1: string
  cmmTreasury: string
  officialLauncher: string
  sourceCodeId: number
  tokenAddr?: string
}): { adopt: Record<string, unknown> } {
  const alpha = isAlphaSource(input.sourceCodeId, input.tokenAddr ?? '')
  return {
    adopt: {
      manager: input.manager,
      treasury: input.treasury,
      factory: input.factory,
      router: input.router,
      ust1: input.ust1,
      cmm_treasury: input.cmmTreasury,
      official_launcher: input.officialLauncher,
      buy_bps: alpha ? ALPHA_BUY_BPS : 0,
      sell_bps: alpha ? ALPHA_SELL_BPS : 0,
      transfer_bps: null,
      max_buy_bps: alpha ? ALPHA_BUY_BPS : 0,
      max_sell_bps: alpha ? ALPHA_SELL_BPS : 0,
      max_transfer_bps: 0,
      source_code_id: input.sourceCodeId,
    },
  }
}

export const MIGRATE_LP_CONFIRM =
  'Address stays the same. Holders stay. Terraport/GDEX keep this CW20 (1:1 stay 1:1 on honest templates). CL8Y factory pairs freeze until governance Refresh. Extra-debit applies only after you register a CL8Y listed pair — never a Terraport or GDEX pair.'

export const MIGRATE_LP_CONFIRM_ALPHA =
  'Address stays the same. Holders stay. tax_info / tax_map are wiped — Terraport/GDEX forward flow becomes 1:1. Historical 4.5% pair→user skim is not unwound. Do not RegisterListedPair those pairs. CL8Y factory pairs freeze until governance Refresh. Extra-debit is CL8Y listed pairs only.'
