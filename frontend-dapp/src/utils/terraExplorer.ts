import chainlistJson from '../../public/chains/chainlist.json'
import { DEFAULT_NETWORK, NETWORKS } from './constants'
import { isValidTerraBech32Address } from './terraAddressValidation'
import { shortenAddress } from './tokenDisplay'

export type ChainlistEntry = {
  id: string
  name: string
  chainId: number | string
  type: string
  icon: string
  rpcUrl?: string
  lcdUrl?: string
  explorerUrl?: string
  tier: string
}

export type ChainlistData = {
  name: string
  version: string
  chains: ChainlistEntry[]
}

const chainlist = chainlistJson as ChainlistData

type ExplorerPathSegment = 'tx' | 'address'

/** SHA-256 tx hash as returned by RPC/LCD (64 hex digits). Rejects injectable segments ([#430](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/430)). */
const EXPLORER_TX_HASH_RE = /^[0-9a-fA-F]{64}$/

function isSafeExplorerTxHash(txHash: string): boolean {
  return EXPLORER_TX_HASH_RE.test(txHash)
}

function isSafeExplorerAddress(address: string): boolean {
  const trimmed = address.trim()
  if (!trimmed) return false
  return isValidTerraBech32Address(trimmed)
}

function explorerPathBaseForChainId(chainId: string, segment: ExplorerPathSegment): string | null {
  const entry = chainlist.chains.find((c) => String(c.chainId) === chainId)
  if (!entry?.explorerUrl) return null
  const base = entry.explorerUrl.replace(/\/$/, '')
  return `${base}/${segment}/`
}

/**
 * Full URL to view a transaction on the block explorer for the active `VITE_NETWORK` build.
 */
export function getExplorerTxUrl(txHash: string): string | null {
  if (!isSafeExplorerTxHash(txHash)) return null

  const { chainId } = NETWORKS[DEFAULT_NETWORK].terra

  if (DEFAULT_NETWORK === 'local') {
    const lcd = NETWORKS.local.terra.lcd.replace(/\/$/, '')
    return `${lcd}/cosmos/tx/v1beta1/txs/${txHash}`
  }

  const base = explorerPathBaseForChainId(chainId, 'tx')
  if (!base) return null
  return `${base}${txHash}`
}

/**
 * Full URL to view an account on the block explorer for the active `VITE_NETWORK` build.
 * Used by [`AddressRow`](../components/ui/AddressRow.tsx) and wallet explorer ([#184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184), [#188](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188)).
 */
export function getExplorerAddressUrl(address: string): string | null {
  if (!isSafeExplorerAddress(address)) return null

  const { chainId } = NETWORKS[DEFAULT_NETWORK].terra

  if (DEFAULT_NETWORK === 'local') {
    const lcd = NETWORKS.local.terra.lcd.replace(/\/$/, '')
    return `${lcd}/cosmos/auth/v1beta1/accounts/${address}`
  }

  const base = explorerPathBaseForChainId(chainId, 'address')
  if (!base) return null
  return `${base}${address}`
}

/**
 * Defense-in-depth for explorer `<a href>` ([#671](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/671)).
 * `getExplorerAddressUrl` already returns http(s) or `null`; still omit `javascript:` / `data:`
 * if a mock or future helper leak would otherwise render a live non-http link.
 */
export function isSafeExplorerHref(href: string | null | undefined): href is string {
  if (!href) return false
  try {
    const parsed = new URL(href)
    return parsed.protocol === 'http:' || parsed.protocol === 'https:'
  } catch {
    return false
  }
}

/** Middle-elided hash for alerts, tables, and other compact tx displays. */
export function shortenTxHashForDisplay(txHash: string): string {
  return shortenAddress(txHash, 8, 6)
}
