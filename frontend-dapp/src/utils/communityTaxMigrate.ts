/**
 * Free listed-template adopt onto the community tax wasm (GitLab #626).
 * No invoice. Query params never prefill payee / admin / treasury.
 */

export const COLUMBUS5_ADOPT_CODE_IDS = [6036, 10184, 8266] as const
export const COLUMBUS5_NOGO_CODE_IDS = [8654] as const
export const COLUMBUS5_TAX_CODE_IDS = [11611, 11619] as const
export const ALPHA_COLUMBUS5_ADDR = 'terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz'

export const ALLOWED_ADOPT_CW2 = [
  'crates.io:cw20-base',
  'crates.io:cw20-mintable',
  'crates.io:terraport-token',
] as const

export type MigrateVerdictKind =
  | 'go'
  | 'nogo_8654'
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
  if (input.hasTaxMap || COLUMBUS5_NOGO_CODE_IDS.includes(input.codeId as 8654) || addr === ALPHA_COLUMBUS5_ADDR) {
    return {
      kind: input.hasTaxMap && input.codeId !== 8654 && addr !== ALPHA_COLUMBUS5_ADDR ? 'tax_map' : 'nogo_8654',
      reason:
        'This template has inbound tax_map (ALPHA / 8654). In-place adopt is not supported. Create a new 11619 token or use the wrap path on #558.',
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
  if (!input.whitelisted) {
    return {
      kind: 'unlisted',
      reason: 'Source code id is not on the factory whitelist. This page will not whitelist it.',
      canSubmit: false,
    }
  }
  if (isColumbus5(input.chainId) && !COLUMBUS5_ADOPT_CODE_IDS.includes(input.codeId as 6036 | 10184 | 8266)) {
    return {
      kind: 'unlisted',
      reason: 'Only factory-listed 6036, 10184, and 8266 can adopt on columbus-5.',
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
}): { adopt: Record<string, unknown> } {
  return {
    adopt: {
      manager: input.manager,
      treasury: input.treasury,
      factory: input.factory,
      router: input.router,
      ust1: input.ust1,
      cmm_treasury: input.cmmTreasury,
      official_launcher: input.officialLauncher,
      buy_bps: 0,
      sell_bps: 0,
      transfer_bps: null,
      max_buy_bps: 0,
      max_sell_bps: 0,
      max_transfer_bps: 0,
      source_code_id: input.sourceCodeId,
    },
  }
}

export const MIGRATE_LP_CONFIRM =
  'Address stays the same. Holders stay. Terraport/GDEX keep this CW20 (1:1 stay 1:1 on honest templates). CL8Y factory pairs freeze until governance Refresh. Extra-debit applies only after you register a CL8Y listed pair — never a Terraport or GDEX pair.'
