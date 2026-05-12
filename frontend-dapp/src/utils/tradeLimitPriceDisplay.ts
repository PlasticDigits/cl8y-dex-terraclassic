import { formatNum } from '@/utils/formatAmount'

const USTC_SYMBOLS = new Set(['USTC', 'UST', 'USTC(CW20)'])

export function indexerAssetIsLikelyStableUsd(symbol: string | undefined | null): boolean {
  if (!symbol) return false
  return USTC_SYMBOLS.has(symbol.trim().toUpperCase())
}

/**
 * When **token1** (quote) is a USD-pegged stable tracked by the indexer oracle, show an
 * approximate **USD per token0** headline for a limit price expressed as token1/token0.
 */
export function limitPriceUsdHint(
  priceToken1PerToken0: string,
  token1Symbol: string | undefined,
  ustcUsd: string | null | undefined
): string | null {
  if (!indexerAssetIsLikelyStableUsd(token1Symbol)) return null
  const u = ustcUsd?.trim()
  if (!u) return null
  const p = parseFloat(priceToken1PerToken0.trim())
  const ou = parseFloat(u)
  if (!Number.isFinite(p) || !Number.isFinite(ou) || p <= 0 || ou <= 0) return null
  return `≈ $${formatNum(String(p * ou), 6)} per token0 (USTC/USD oracle × limit price)`
}

export function inverseLimitPriceHuman(priceToken1PerToken0: string): string | null {
  const p = parseFloat(priceToken1PerToken0.trim())
  if (!Number.isFinite(p) || p <= 0) return null
  return formatNum(String(1 / p), 8)
}
