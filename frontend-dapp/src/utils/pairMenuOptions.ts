import type { MenuSelectOption } from '@/components/ui/MenuSelect'
import type { PairInfo } from '@/types'
import { assetInfoLabel } from '@/types'
import { getTokenDisplaySymbol, shortenAddress } from '@/utils/tokenDisplay'

const DEFAULT_PLACEHOLDER: MenuSelectOption = { value: '', label: 'Select pair…' }

export type PairMenuOptionsArgs = {
  /** When set, prepended as the first row (e.g. empty value + “Select pair…”). */
  placeholder?: MenuSelectOption
}

/** Labels for {@link MenuSelect} from on-chain {@link PairInfo} (factory / LCD). */
export function pairInfosToMenuSelectOptions(pairs: PairInfo[], args?: PairMenuOptionsArgs): MenuSelectOption[] {
  if (pairs.length === 0) return []
  const placeholder = args?.placeholder ?? DEFAULT_PLACEHOLDER
  const rows = pairs.map((p) => {
    const a = assetInfoLabel(p.asset_infos[0])
    const b = assetInfoLabel(p.asset_infos[1])
    const la = getTokenDisplaySymbol(a)
    const lb = getTokenDisplaySymbol(b)
    return {
      value: p.contract_addr,
      label: `${la} / ${lb} — ${shortenAddress(p.contract_addr)}`,
    }
  })
  return [placeholder, ...rows]
}
