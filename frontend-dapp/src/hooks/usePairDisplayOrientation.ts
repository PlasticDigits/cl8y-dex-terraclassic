import { useCallback, useMemo, useState } from 'react'
import {
  defaultDisplayInverted,
  displayPairAssets,
  pairDisplayInvertAriaLabel,
  pairDisplayPillLabel,
  readStoredPairDisplayInverted,
  writeStoredPairDisplayInverted,
  type PairDisplayLeg,
} from '@/utils/tradePairDisplayOrientation'

export type PairDisplayOrientation = {
  inverted: boolean
  toggleInverted: () => void
  displayBase: string
  displayQuote: string
  pillLabel: string
  invertAriaLabel: string
}

/**
 * One orientation flag per factory `pairAddr` (GitLab #524).
 * First visit uses the UST1-as-base default; later visits read sessionStorage.
 */
export function usePairDisplayOrientation(args: {
  pairAddr: string
  asset0: PairDisplayLeg | null | undefined
  asset1: PairDisplayLeg | null | undefined
  token0Symbol: string
  token1Symbol: string
}): PairDisplayOrientation {
  const { pairAddr, asset0, asset1, token0Symbol, token1Symbol } = args
  const productDefault = defaultDisplayInverted(asset0, asset1)
  const [tick, setTick] = useState(0)

  const inverted = useMemo(() => {
    void tick
    if (!pairAddr) return productDefault
    const stored = readStoredPairDisplayInverted(pairAddr)
    return stored ?? productDefault
  }, [pairAddr, productDefault, tick])

  const toggleInverted = useCallback(() => {
    if (!pairAddr) return
    const stored = readStoredPairDisplayInverted(pairAddr)
    const current = stored ?? productDefault
    writeStoredPairDisplayInverted(pairAddr, !current)
    setTick((n) => n + 1)
  }, [pairAddr, productDefault])

  const { displayBase, displayQuote } = displayPairAssets(token0Symbol, token1Symbol, inverted)

  return {
    inverted,
    toggleInverted,
    displayBase,
    displayQuote,
    pillLabel: pairDisplayPillLabel(displayBase, displayQuote),
    invertAriaLabel: pairDisplayInvertAriaLabel(displayBase, displayQuote),
  }
}
