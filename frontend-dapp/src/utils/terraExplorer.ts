import chainlistJson from '../../public/chains/chainlist.json'
import { DEFAULT_NETWORK, NETWORKS } from './constants'

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

function explorerTxBaseForChainId(chainId: string): string | null {
  const entry = chainlist.chains.find((c) => String(c.chainId) === chainId)
  if (!entry?.explorerUrl) return null
  const base = entry.explorerUrl.replace(/\/$/, '')
  return `${base}/tx/`
}

/**
 * Full URL to view a transaction on the block explorer for the active `VITE_NETWORK` build.
 */
export function getExplorerTxUrl(txHash: string): string | null {
  const { chainId } = NETWORKS[DEFAULT_NETWORK].terra

  if (DEFAULT_NETWORK === 'local') {
    const lcd = NETWORKS.local.terra.lcd.replace(/\/$/, '')
    return `${lcd}/cosmos/tx/v1beta1/txs/${txHash}`
  }

  const base = explorerTxBaseForChainId(chainId)
  if (!base) return null
  return `${base}${txHash}`
}
