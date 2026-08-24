/**
 * Free listed-template adopt onto the community tax wasm (GitLab #626).
 * No invoice. Query params never prefill payee / admin / treasury.
 *
 * Source gate is the **migrate code-id allowlist** (`VITE_COMMUNITY_MIGRATE_CODE_IDS`),
 * not factory `AddWhitelistedCodeId`. Factory listing is for pair assets (F6).
 */

/** Columbus-5 defaults. Append more ids via `VITE_COMMUNITY_MIGRATE_CODE_IDS`. */
export const DEFAULT_COMMUNITY_MIGRATE_CODE_IDS = [6036, 10184, 8266, 8654] as const
export const COLUMBUS5_TAX_CODE_IDS = [11611, 11619] as const
/** Default FoT leftover map: 4.5% buy / 1% sell (combined 550 ≤ 2500). */
export const FOT_WIPE_BUY_BPS = 450
export const FOT_WIPE_SELL_BPS = 100

export const ALLOWED_ADOPT_CW2 = [
  'crates.io:cw20-base',
  'crates.io:cw20-mintable',
  'crates.io:terraport-token',
  'crates.io:cw20-taxed',
  'cw20_taxed',
] as const

export type MigrateVerdictKind = 'go' | 'already_tax' | 'unlisted' | 'not_admin' | 'unavailable' | 'bad_addr'

export type MigrateVerdict = {
  kind: MigrateVerdictKind
  reason: string
  canSubmit: boolean
}

export function isColumbus5(chainId: string): boolean {
  return chainId === 'columbus-5'
}

export function parseCommunityMigrateCodeIds(raw?: string | null): number[] {
  const src = (raw ?? '').trim()
  if (!src) return [...DEFAULT_COMMUNITY_MIGRATE_CODE_IDS]
  const ids = src
    .split(/[,\s]+/)
    .map((p) => Number(p))
    .filter((n) => Number.isFinite(n) && n > 0)
  return ids.length ? ids : [...DEFAULT_COMMUNITY_MIGRATE_CODE_IDS]
}

export function isMigrateSourceCodeId(codeId: number, allowedCodeIds: readonly number[]): boolean {
  return allowedCodeIds.includes(codeId)
}

export function classifyMigrateSource(input: {
  chainId: string
  codeId: number
  taxCodeId: number
  /** LocalTerra-only fallback when env list does not include the local store id. */
  factoryWhitelisted?: boolean
  hasTaxMap: boolean
  wasmAdmin: string
  connectedWallet: string | null
  allowedCodeIds: readonly number[]
}): MigrateVerdict {
  if (input.codeId === input.taxCodeId || COLUMBUS5_TAX_CODE_IDS.includes(input.codeId as 11611 | 11619)) {
    return {
      kind: 'already_tax',
      reason: 'This contract is already the community tax wasm. Same-crate upgrades are CMM-only — not this page.',
      canSubmit: false,
    }
  }
  const onList = isMigrateSourceCodeId(input.codeId, input.allowedCodeIds)
  const localFactoryOk = !isColumbus5(input.chainId) && input.factoryWhitelisted === true
  if (!onList && !localFactoryOk) {
    return {
      kind: 'unlisted',
      reason:
        'Source code id is not on the migrate allowlist. Add it to VITE_COMMUNITY_MIGRATE_CODE_IDS. This page does not factory-whitelist code ids.',
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
    reason: input.hasTaxMap
      ? 'Allowlisted source with tax leftovers. One click wipes tax_info / tax_map, keeps this address, and lands on the tax wasm. No 50 UST1.'
      : 'Allowlisted source. One click migrates this address onto the tax wasm. No 50 UST1.',
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
  hasTaxMap?: boolean
}): { adopt: Record<string, unknown> } {
  const wipe = input.hasTaxMap === true
  return {
    adopt: {
      manager: input.manager,
      treasury: input.treasury,
      factory: input.factory,
      router: input.router,
      ust1: input.ust1,
      cmm_treasury: input.cmmTreasury,
      official_launcher: input.officialLauncher,
      buy_bps: wipe ? FOT_WIPE_BUY_BPS : 0,
      sell_bps: wipe ? FOT_WIPE_SELL_BPS : 0,
      transfer_bps: null,
      max_buy_bps: wipe ? FOT_WIPE_BUY_BPS : 0,
      max_sell_bps: wipe ? FOT_WIPE_SELL_BPS : 0,
      max_transfer_bps: 0,
      source_code_id: input.sourceCodeId,
    },
  }
}

export const MIGRATE_LP_CONFIRM =
  'Address stays the same. Holders stay. Terraport/GDEX keep this CW20 (honest templates stay 1:1). CL8Y factory pairs freeze until governance Refresh. Extra-debit applies only after you register a CL8Y listed pair — never a Terraport or GDEX pair.'

export const MIGRATE_LP_CONFIRM_WIPE =
  'Address stays the same. Holders stay. tax_info / tax_map are wiped — Terraport/GDEX forward flow becomes 1:1. Historical skim is not unwound. Do not RegisterListedPair those pairs. CL8Y factory pairs freeze until governance Refresh. Extra-debit is CL8Y listed pairs only.'
