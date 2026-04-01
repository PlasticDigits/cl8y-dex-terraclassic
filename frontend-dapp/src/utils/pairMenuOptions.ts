import type { MenuSelectOption } from '@/components/ui/MenuSelect'
import type { IndexerPair, PairInfo } from '@/types'
import { assetInfoLabel, indexerPairToPairInfo } from '@/types'
import { getTokenDisplaySymbol, shortenAddress } from '@/utils/tokenDisplay'

const DEFAULT_PLACEHOLDER: MenuSelectOption = { value: '', label: 'Select pair…' }

/**
 * How to render a pair in menus and headings.
 *
 * - **`full`** (default): `SYMBOL_A / SYMBOL_B — shortened pair addr` — use for **pickers and tx-heavy flows**
 *   (e.g. Limit Orders) where disambiguation matters.
 * - **`compact`**: `SYMBOL_A / SYMBOL_B` only — use for **browse / dense UI** (e.g. Charts pair list on narrow widths).
 */
export type PairMenuLabelVariant = 'full' | 'compact'

export type PairMenuLabelOptions = {
  variant?: PairMenuLabelVariant
}

const DEFAULT_VARIANT: PairMenuLabelVariant = 'full'

export type PairMenuOptionsArgs = PairMenuLabelOptions & {
  /** When set, prepended as the first row (e.g. empty value + “Select pair…”). */
  placeholder?: MenuSelectOption
}

/** Single-line pair label for menus and headings. */
export function pairInfoMenuLabel(pair: PairInfo, opts?: PairMenuLabelOptions): string {
  const variant = opts?.variant ?? DEFAULT_VARIANT
  const a = assetInfoLabel(pair.asset_infos[0])
  const b = assetInfoLabel(pair.asset_infos[1])
  const la = getTokenDisplaySymbol(a)
  const lb = getTokenDisplaySymbol(b)
  const core = `${la} / ${lb}`
  if (variant === 'compact') return core
  return `${core} — ${shortenAddress(pair.contract_addr)}`
}

/** Same as {@link pairInfoMenuLabel} for indexer rows. */
export function indexerPairMenuLabel(p: IndexerPair, opts?: PairMenuLabelOptions): string {
  return pairInfoMenuLabel(indexerPairToPairInfo(p), opts)
}

/** {@link MenuSelect} rows from indexer pairs. */
export function indexerPairsToMenuSelectOptions(pairs: IndexerPair[], args?: PairMenuOptionsArgs): MenuSelectOption[] {
  if (pairs.length === 0) return []
  const variant = args?.variant ?? DEFAULT_VARIANT
  return pairs.map((p) => ({
    value: p.pair_address,
    label: pairInfoMenuLabel(indexerPairToPairInfo(p), { variant }),
  }))
}

/** {@link MenuSelect} rows from on-chain {@link PairInfo} (factory / LCD). */
export function pairInfosToMenuSelectOptions(pairs: PairInfo[], args?: PairMenuOptionsArgs): MenuSelectOption[] {
  if (pairs.length === 0) return []
  const variant = args?.variant ?? DEFAULT_VARIANT
  const placeholder = args?.placeholder ?? DEFAULT_PLACEHOLDER
  const rows = pairs.map((p) => ({
    value: p.contract_addr,
    label: pairInfoMenuLabel(p, { variant }),
  }))
  return [placeholder, ...rows]
}
