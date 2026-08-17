/**
 * Compact token identity targets for Pool / Trade / Charts (GitLab #541).
 *
 * Copy/explorer payloads come from factory or indexer `AssetInfo` after
 * bech32 / denom checks — never from a display symbol. Invert changes
 * visible order only (T541-5).
 *
 * Invariants **T541-1–T541-8** — see `docs/frontend.md` § Token identity
 * and `skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md`.
 */

import type { AssetInfo, IndexerAssetBrief } from '@/types'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import { getExplorerAddressUrl } from '@/utils/terraExplorer'

export type TokenIdentityCw20 = {
  kind: 'cw20'
  address: string
  explorerUrl: string | null
}

export type TokenIdentityNative = {
  kind: 'native'
  denom: string
}

export type TokenIdentityTarget = TokenIdentityCw20 | TokenIdentityNative

export type PairIdentityRole = 'base' | 'quote'

const UNSAFE_DENOM_RE = /[<>'"\\\s]|:\/\//

/** Native denoms are copy-only — never passed to `getExplorerAddressUrl` (T541-2). */
export function isSafeNativeDenom(denom: string): boolean {
  const trimmed = denom.trim()
  if (!trimmed || trimmed.length > 128) return false
  if (UNSAFE_DENOM_RE.test(trimmed)) return false
  const lower = trimmed.toLowerCase()
  if (lower.startsWith('javascript:') || lower.startsWith('data:')) return false
  if (trimmed.startsWith('terra1')) return false
  return trimmed === denom
}

export function tokenIdentityTarget(info: AssetInfo | null | undefined): TokenIdentityTarget | null {
  if (!info) return null
  if ('token' in info) {
    const address = info.token.contract_addr?.trim() ?? ''
    if (!address || !isValidTerraBech32Address(address)) return null
    return {
      kind: 'cw20',
      address,
      explorerUrl: getExplorerAddressUrl(address),
    }
  }
  const denom = info.native_token.denom ?? ''
  if (!isSafeNativeDenom(denom)) return null
  return { kind: 'native', denom }
}

/** Indexer brief → `AssetInfo` without throwing when both fields are missing. */
export function assetInfoFromIndexerBrief(brief: IndexerAssetBrief | null | undefined): AssetInfo | null {
  if (!brief) return null
  const contract = brief.contract_addr?.trim() ?? ''
  if (contract) return { token: { contract_addr: contract } }
  const denom = brief.denom?.trim() ?? ''
  if (denom) return { native_token: { denom } }
  return null
}

export function tokenIdentityTargetFromIndexerBrief(
  brief: IndexerAssetBrief | null | undefined
): TokenIdentityTarget | null {
  return tokenIdentityTarget(assetInfoFromIndexerBrief(brief))
}

/** Factory-stable targets. Invert must not swap these (T541-5 / A7). */
export function pairIdentityTargets(args: {
  asset0: AssetInfo | null | undefined
  asset1: AssetInfo | null | undefined
}): { base: TokenIdentityTarget | null; quote: TokenIdentityTarget | null } {
  return {
    base: tokenIdentityTarget(args.asset0),
    quote: tokenIdentityTarget(args.asset1),
  }
}

/** Visible chip order follows #524 invert; payloads stay factory `asset_0` / `asset_1`. */
export function pairIdentityLegOrder(inverted: boolean): PairIdentityRole[] {
  return inverted ? ['quote', 'base'] : ['base', 'quote']
}

export function isPairIdentityAddress(pairAddress: string | null | undefined): boolean {
  return !!pairAddress && isValidTerraBech32Address(pairAddress.trim())
}

export function copyPayload(target: TokenIdentityTarget): string {
  return target.kind === 'cw20' ? target.address : target.denom
}
