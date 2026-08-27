import { useCallback, useMemo, useState } from 'react'
import {
  chartsPriceTokenForInverted,
  defaultDisplayInverted,
  displayPairAssets,
  pairDisplayInvertAriaLabel,
  pairDisplayPillLabel,
  readStoredPairDisplayInverted,
  resolveChartsDisplayInverted,
  writeChartsStoredPairDisplayInverted,
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

export type ChartsPairDisplayOrientation = PairDisplayOrientation & {
  pricedToken: string
}

/**
 * Charts orientation (#680). Resolve: valid `?price=` match → Charts session →
 * Charts product default (not inverted). Toggle writes the Charts key only and
 * asks the page to replace `?price=`.
 */
export function useChartsPairDisplayOrientation(args: {
  pairAddr: string
  asset0: PairDisplayLeg | null | undefined
  asset1: PairDisplayLeg | null | undefined
  token0Symbol: string
  token1Symbol: string
  priceMatch: 'asset0' | 'asset1' | null
  onPriceTokenChange: (token: string) => void
}): ChartsPairDisplayOrientation {
  const { pairAddr, asset0, asset1, token0Symbol, token1Symbol, priceMatch, onPriceTokenChange } = args
  const [tick, setTick] = useState(0)

  const inverted = useMemo(() => {
    void tick
    return resolveChartsDisplayInverted(pairAddr, asset0, asset1, priceMatch)
  }, [pairAddr, asset0, asset1, priceMatch, tick])

  const toggleInverted = useCallback(() => {
    const next = !inverted
    if (pairAddr) writeChartsStoredPairDisplayInverted(pairAddr, next)
    onPriceTokenChange(chartsPriceTokenForInverted(next, asset0, asset1, token0Symbol, token1Symbol))
    setTick((n) => n + 1)
  }, [inverted, pairAddr, asset0, asset1, token0Symbol, token1Symbol, onPriceTokenChange])

  const { displayBase, displayQuote } = displayPairAssets(token0Symbol, token1Symbol, inverted)
  const pricedToken = chartsPriceTokenForInverted(inverted, asset0, asset1, token0Symbol, token1Symbol)

  return {
    inverted,
    toggleInverted,
    displayBase,
    displayQuote,
    pricedToken,
    pillLabel: pairDisplayPillLabel(displayBase, displayQuote),
    invertAriaLabel: pairDisplayInvertAriaLabel(displayBase, displayQuote),
  }
}
