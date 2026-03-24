import { DEFAULT_NETWORK, NETWORKS } from './constants'

/** Match bridge `WalletButton` / `getTerraChainLogoPath` (packages/frontend). */
export function getTerraChainLogoPath(chainId: string): string {
  if (chainId === 'localterra') return '/chains/localterra-icon.png'
  if (chainId === 'localterra1' || chainId === 'localterra2') return '/chains/localterra2-icon.png'
  return '/chains/terraclassic-icon.png'
}

export function getNetworkBadgeCopy(): {
  shortLabel: string
  fullLabel: string
  chainId: string
} {
  const chainId = NETWORKS[DEFAULT_NETWORK].terra.chainId
  if (DEFAULT_NETWORK === 'local') {
    return { shortLabel: 'Local', fullLabel: 'LocalTerra', chainId }
  }
  if (DEFAULT_NETWORK === 'testnet') {
    return { shortLabel: 'Testnet', fullLabel: 'Terra Classic Testnet', chainId }
  }
  return { shortLabel: 'Mainnet', fullLabel: 'Terra Classic', chainId }
}
