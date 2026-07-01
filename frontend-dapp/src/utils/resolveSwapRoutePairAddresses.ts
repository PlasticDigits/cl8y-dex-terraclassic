import { assetInfoLabel, type PairInfo } from '@/types'
import type { SwapOperation } from '@/services/terraclassic/router'

export type ResolveSwapRoutePairAddressesInput = {
  routeOps?: SwapOperation[] | null
  pairs: PairInfo[]
  directPair?: PairInfo | null
  fromToken?: string | null
  toToken?: string | null
}

/**
 * Factory-sourced pair contract addresses for the active swap route (LCD `pairs` list).
 * Used for pause/blacklist probes and pre-sign contract transparency ([#449](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/449)).
 */
export function resolveSwapRoutePairAddresses({
  routeOps,
  pairs,
  directPair,
  fromToken,
  toToken,
}: ResolveSwapRoutePairAddressesInput): string[] {
  const addresses = new Set<string>()

  if (routeOps && routeOps.length > 0) {
    for (const op of routeOps) {
      const offer = assetInfoLabel(op.terra_swap.offer_asset_info)
      const ask = assetInfoLabel(op.terra_swap.ask_asset_info)
      const matched = pairs.find((p) => {
        const a = assetInfoLabel(p.asset_infos[0])
        const b = assetInfoLabel(p.asset_infos[1])
        return (a === offer && b === ask) || (b === offer && a === ask)
      })
      if (matched?.contract_addr.startsWith('terra1')) {
        addresses.add(matched.contract_addr)
      }
    }
  } else if (directPair?.contract_addr.startsWith('terra1')) {
    addresses.add(directPair.contract_addr)
  }

  if (addresses.size === 0 && fromToken?.startsWith('terra1') && toToken?.startsWith('terra1')) {
    const fallback = pairs.find((p) => {
      const a = assetInfoLabel(p.asset_infos[0])
      const b = assetInfoLabel(p.asset_infos[1])
      return (a === fromToken && b === toToken) || (b === fromToken && a === toToken)
    })
    if (fallback?.contract_addr.startsWith('terra1')) {
      addresses.add(fallback.contract_addr)
    }
  }

  return [...addresses]
}
