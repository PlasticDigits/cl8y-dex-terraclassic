import type { MenuSelectOption } from '@/components/ui/MenuSelect'
import type { IndexerPair, PairInfo } from '@/types'
import { assetInfoLabel, indexerPairToPairInfo } from '@/types'
import { getTokenDisplaySymbol, shortenAddress } from '@/utils/tokenDisplay'

const DEFAULT_PLACEHOLDER: MenuSelectOption = { value: '', label: 'Select pair…' }

export type PairMenuOptionsArgs = {
  /** When set, prepended as the first row (e.g. empty value + “Select pair…”). */
  placeholder?: MenuSelectOption
}

/** Single-line label for a pair (token display symbols + shortened pair contract). */
export function pairInfoMenuLabel(pair: PairInfo): string {
  const a = assetInfoLabel(pair.asset_infos[0])
  const b = assetInfoLabel(pair.asset_infos[1])
  const la = getTokenDisplaySymbol(a)
  const lb = getTokenDisplaySymbol(b)
  return `${la} / ${lb} — ${shortenAddress(pair.contract_addr)}`
}

/** {@link MenuSelect} rows from indexer pairs (same labels as on-chain {@link pairInfosToMenuSelectOptions}). */
export function indexerPairsToMenuSelectOptions(pairs: IndexerPair[]): MenuSelectOption[] {
  if (pairs.length === 0) return []
  return pairs.map((p) => ({
    value: p.pair_address,
    label: pairInfoMenuLabel(indexerPairToPairInfo(p)),
  }))
}

/** Labels for {@link MenuSelect} from on-chain {@link PairInfo} (factory / LCD). */
export function pairInfosToMenuSelectOptions(pairs: PairInfo[], args?: PairMenuOptionsArgs): MenuSelectOption[] {
  if (pairs.length === 0) return []
  const placeholder = args?.placeholder ?? DEFAULT_PLACEHOLDER
  const rows = pairs.map((p) => ({
    value: p.contract_addr,
    label: pairInfoMenuLabel(p),
  }))
  return [placeholder, ...rows]
}
